# Desktop IPC contract

The contract between the desktop app's webview (React, `apps/desktop/src`) and its
Rust core (`apps/desktop/src-tauri`). The webview never holds keys, ciphertext or the
store. It asks Rust for summaries, and it sees a secret only when the user asks for it.

Mosaic is pre-audit. Nothing here has been externally reviewed.

- Rust types: `apps/desktop/src-tauri/src/contract/` (one module per area below)
- TypeScript types: `apps/desktop/src/ipc/contract/` (same modules)
- Command list: `src-tauri/src/commands/names.rs`, the only place a command name is
  added; `build.rs`, the capability test and the fixtures all read it
- Type parity: ADR-0018. The security baseline: ADR-0019.

## 1. Rules

1. **Transport.** Every command is a Tauri command that takes exactly one argument
   named `request`, or none (`null` in the TypeScript map). Events from Rust travel
   over one `Channel` that the webview passes to `subscribe_events`. With a channel,
   the capability needs no `core:event` permission.
2. **Results.** Every command returns `Result<T, IpcError>`. The TypeScript client
   (`MosaicIpc.call`) never throws. It returns `{ ok: true, value }` or
   `{ ok: false, error }`. Any rejection that is not a contract error (an ACL denial, a
   Tauri error) becomes `{ code: "internal" }`.
3. **Errors carry a code and nothing else.** No message, no ID and no value from the
   vault. The UI shows the i18n message `error.<code>`, which says what happened and
   what to do next (UX_GUIDELINES §5). A type-level check and a test keep every code
   translated in DE and EN.
4. **Wire format.** JSON with camelCase keys. Tagged unions carry their tag in `kind`,
   `state`, `op` or `type`. String enums are camelCase. Every key is always present:
   "no value" is `null`, never a missing key, and the TypeScript types have no optional
   properties. Requests reject unknown keys. IDs are canonical lowercase UUID strings.
   Times are Unix milliseconds (`UnixMillis`).
5. **Secrets.** See §6. In short, a secret enters Rust as `SecretIn` and leaves only as
   `SecretOut`, and only in the four responses listed there. Lists, details, events and
   errors never contain one. Copying is done by Rust.
6. **Locking.** Every command that needs keys fails with `locked` while the vault is
   locked. On the `locked` event, the UI drops every revealed secret and every form
   value, then shows the unlock screen.

## 2. Session state machine

```
             create_account                  lock / auto-lock / sleep / screen lock
 noAccount ─────────────────► unlocked ◄──────────────────────────────────────┐
                                  │                                            │
                                  └────────────────► locked ──── unlock ───────┘
                                                            └── unlock_quick ──┘
```

`SessionState` is `{ state: "noAccount" }`,
`{ state: "locked", email, secretKeyStored, quickUnlock }` or
`{ state: "unlocked", accountId, email, kitVerified }`. Every session command returns the new state.
`quickUnlock` lists the methods the unlock screen may offer: `biometric` once Windows
Hello or Touch ID is enrolled, `hardwareKey` once a security key is registered and the
system supports keys. It is empty after an app start until the master password has been
used once (quick unlock never works right after a restart), and a method drops out when
its enrollment expires (`maxAgeDays`, from `Settings.quickUnlockMaxAgeDays`) or the
platform invalidated it. The UI offers a method only from this list, never prompts on
screen load, and re-reads `session_state` after `quickUnlockExpired`,
`quickUnlockInvalidated` or `passwordRequired` so the buttons vanish and the password
field takes focus (UX_GUIDELINES §8).
The store can also be unavailable, for example when another process holds it. In that
case every command fails with `storeInUse` or `storeUnavailable`, and the UI shows that
sentence.

Account creation returns the Emergency Kit data once. The UI keeps the vault UI hidden
until the user has typed back the last group of the Secret Key (`verify_emergency_kit`,
UX_GUIDELINES §8). The Secret Key lives in the OS keystore (ADR-0020 item 1), behind an in-memory layer. `kitVerified` is stored per account; while it is `false` the UI shows the kit step again. Without a stored key, the locked state reports
`secretKeyStored: false`, and `unlock` takes the key as typed from the kit. A typed key
that unlocks is stored.

## 3. Commands

Status: **live** means wired now. Security keys report `supported: false` on every OS
until the FIDO2 transport lands (ADR-0029, owner decision); the UI hides all key controls then.

| Command | Request → response | Errors besides `locked` / `internal` | Status |
|---|---|---|---|
| **Session** | | | |
| `session_state` | `null` → `SessionState` | `storeInUse`, `storeUnavailable` | live |
| `create_account` | `{ email, password, defaultVaultName }` → `{ emergencyKit: { accountId, email, secretKey }, session }` | `accountExists`, `invalidInput`, `emptyPassword` | live (Argon2 calibrated on the device) |
| `save_emergency_kit` | `{ labels: { title, intro, email, accountId, signInUrl, signInUrlValue, secretKey, masterPassword, masterPasswordHint, footer }, pending: bool, password \| null }` → `{ kind: "saved" } \| { kind: "cancelled" }`. `pending: true` (M7) writes the new Secret Key a rotation staged, needs no password and does not touch the kit state | `rotationNone` (nothing staged), `reauthRequired`, `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `secretKeyRequired`, `invalidInput` | live (Rust shows the save dialog, then renders and writes the PDF; no password needed from `create_account` until the kit is verified or the vault locks) |
| `verify_emergency_kit` | `{ lastGroup, pending: bool }` → `{ matches }`. `pending: true` (M7) checks the staged Secret Key of a rotation and sets only the flag its commit checks (`kitNotVerified` otherwise); the wrong-answer counter is shared | `secretKeyRequired`, `tooManyAttempts` (the fifth wrong answer since the last unlock locks the vault and requires the password for the next unlock) | live (needs unlocked; a match sets `kitVerified`) |
| `unlock` | `{ password, secretKey \| null }` → `SessionState` | `wrongCredentials` (never says which secret; from the third in a row, `unlockDelayed` answers the next attempts for 1 s, 2 s, 4 s ... up to 30 s before any key work, nothing sleeps and nothing else is blocked; the UI shows `error.unlockDelayed` and the unlock button can retry after a few seconds; the wait lives in memory and resets on a right password or an app restart), `unlockDelayed`, `secretKeyRequired`, `secretKeyMalformed`, `secretKeyMistyped`, `alreadyUnlocked`, `noAccount` | live |
| `unlock_quick` | `{ method: "biometric" \| "hardwareKey", pin \| null }` → `SessionState` | `quickUnlockUnavailable`, `quickUnlockExpired`, `quickUnlockInvalidated`, `passwordRequired`, `cancelled`, `alreadyUnlocked`, `busy`, `hardwareKey*` codes | live (OS prompt; `pin` only for `hardwareKey`; without it a key that needs one fails with `hardwareKeyPinRequired`) |
| `lock` | `null` → `SessionState` | none; locking a locked vault is a no-op | live |
| `reauthenticate` | `{ password }` → `null` | `locked`, `secretKeyRequired`, `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `emptyPassword` | live (a right password opens a 2-minute window in which `enroll_*`, `add_hardware_vault_key` and `move_item` out of a hardware vault accept `password: null`; the UI sends those first without a password and calls this on `reauthRequired`, then retries) |
| `subscribe_events` | a `Channel<AppEvent>` → `null` | none | live |
| `app_status` | `null` → `{ shortcut: "registered" \| "conflict" \| "unavailable" \| "unsupported", screenLockDetection, otpScreenScan }` (`unsupported`: no global shortcuts on this system, Wayland; the settings field is disabled) | none | live |
| **Quick unlock** | | | |
| `quick_unlock_status` | `null` → `QuickUnlockStatus` = `{ biometric: { platform: "windowsHello" \| "touchId" \| null, available, enrolled, enrolledAt \| null, expiresAt \| null }, hardwareKeys: { supported, keys: { methodId, addedAt }[], maxKeys }, maxAgeDays }` | | live (needs unlocked) |
| `enroll_biometric` | `{ password \| null }` → `QuickUnlockStatus` | `reauthRequired`, `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `quickUnlockUnavailable`, `cancelled`, `busy` (another enrollment is running) | live (the UI sends `null` first and asks for the password on `reauthRequired`; a successful enrollment uses up the 2-minute grant) |
| `remove_biometric` | `null` → `QuickUnlockStatus` | | live |
| `enroll_hardware_key` | `{ password \| null, pin \| null }` → `QuickUnlockStatus` | `reauthRequired`, `wrongCredentials`, `hardwareKeyUnsupported`, `hardwareKeyNotFound`, `hardwareKeyTimeout`, `hardwareKeyPin*`, `hardwareKeyNoPin`, `hardwareKeyLimit`, `cancelled`, `busy` | live (unsupported until ADR-0029) |
| `remove_hardware_key` | `{ methodId }` → `QuickUnlockStatus` | `invalidInput`, `hardwareKeyNotRecognized` | live |
| `hardware_key_cancel` | `null` → `null` | | live (the pending call fails with `cancelled`) |
| **Vaults** | | | |
| `list_vaults` | `null` → `VaultSummary[]` (`vaultId`, `name \| null`, `itemCount`, `available`, `protection: "accountKey" \| "hardwareKey"`, `openUntil \| null`) | | live (a closed hardware vault has `name: null`, `available: false`; `openUntil` is a wall-clock projection of the monotonic deadline, display only) |
| `create_vault` | `{ name }` → `VaultSummary` | `invalidInput` | live |
| `rename_vault` | `{ vaultId, name }` → `VaultSummary` | `invalidInput`, `vaultNotFound`, `vaultUnavailable` | live |
| `create_hardware_vault` | `{ name, pin \| null }` → `VaultSummary` | as `enroll_hardware_key`, `invalidInput` | live (unsupported until ADR-0029) |
| `open_hardware_vault` | `{ vaultId, pin \| null }` → `VaultSummary` | `vaultNotFound`, `invalidInput`, `hardwareKeyNotRecognized`, device codes | live |
| `close_hardware_vault` | `{ vaultId }` → `null` | `vaultNotFound`, `invalidInput` | live |
| `hardware_vault_keys` | `{ vaultId }` → `{ keys }` | `vaultNotFound`, `invalidInput` | live |
| `add_hardware_vault_key` | `{ vaultId, pin \| null }` → `{ keys }` | `reauthRequired` (no `reauthenticate` in the last two minutes; a success uses that re-authentication up, a failure keeps it), `vaultUnavailable`, `hardwareKeyLimit`, device codes | live |
| `remove_hardware_vault_key` | `{ vaultId, methodId }` → `{ keys }` | `vaultUnavailable`, `hardwareKeyLast`, `hardwareKeyNotRecognized` | live |
| **Items** | | | |
| `list_items` | `{ vaultId \| null, trash: "exclude" \| "only" \| "include", favoritesOnly, query \| null }` → `ItemSummary[]` | | live |
| `get_item` | `{ vaultId, itemId }` → `ItemDetail` (secrets as field names only) | `vaultNotFound`, `itemNotFound`, `itemDeleted` | live |
| `reveal_secret` | `{ vaultId, itemId, secret: SecretRef }` → `{ value, hideAfterSeconds }` | `itemNotFound`, `fieldEmpty` | live |
| `add_item` | `{ vaultId, kind, draft: ItemDraft }` → `ItemDetail` | `invalidInput`, `sshKeyInvalid`, `vaultUnavailable`, `otpPendingExpired` | live (at most 512 entries per draft list) |
| `update_item` | `{ vaultId, itemId, draft }` → `ItemDetail` | `invalidInput`, `sshKeyInvalid`, `itemNotFound`, `otpPendingExpired` | live (the kind of an item cannot change, ADR-0033) |
| `set_favorite` | `{ vaultId, itemId, favorite }` → `ItemSummary` | `itemNotFound` | live |
| `trash_item`, `restore_item`, `purge_item` | `{ vaultId, itemId }` → `null`. `purge_item` (delete forever) is refused while a password change of the login is open, so the new password is not lost; the trash is allowed | `itemNotFound`, `itemDeleted`, `rotationOpen` (`purge_item` only) | live |
| `move_item` | `{ vaultId, itemId, targetVaultId }` → `{ vaultId, itemId }` (a new ID) | `sameVault`, `vaultNotFound`, `itemNotFound`, `reauthRequired` (the item leaves a hardware vault without a fresh `reauthenticate`; a success uses it up, a failure keeps it) | live |
| **Attachments** | | | |
| `attachment_add` | `{ vaultId, itemId }` → `{ kind: "added", item: ItemDetail } \| { kind: "cancelled" }` | `vaultNotFound`, `vaultUnavailable`, `itemNotFound`, `itemDeleted`, `itemTypeNotAllowed` (hardware-protected vault), `attachmentTooLarge` (above 50 MiB, checked before reading), `attachmentQuota` (1 GiB per account), `attachmentFileUnreadable`, `storeUnavailable` | live (Rust opens the native file dialog, reads the file bounded, encrypts it; ADR-0034) |
| `attachment_preview` | `{ vaultId, itemId, attachmentId }` → **binary** `ArrayBuffer`: `[width u32 LE][height u32 LE][width * height * 4 bytes RGBA]` | `itemNotFound`, `itemDeleted`, `attachmentNotFound`, `attachmentUnavailable` (not on this device, damaged or failed authentication), `attachmentPreviewUnsupported`, `attachmentPreviewTooLarge`, `busy` (one preview at a time) | live (PNG and JPEG only; refused for trashed items with `itemDeleted`) |
| `attachment_save` | `{ vaultId, itemId, attachmentId }` → `{ kind: "saved" } \| { kind: "cancelled" }` | `itemNotFound`, `itemDeleted`, `attachmentNotFound`, `attachmentUnavailable`, `attachmentSaveFailed` | live (Rust opens the native save dialog and decrypts only after it returns a path; refused for trashed items with `itemDeleted`) |
| `attachment_remove` | `{ vaultId, itemId, attachmentId }` → `ItemDetail` | `itemNotFound`, `itemDeleted`, `attachmentNotFound` | live (a synced edit; the encrypted file leaves this device's store) |
| **Clipboard** | | | |
| `copy_to_clipboard` | `{ source: { kind: "item", vaultId, itemId, field: CopyField } \| { kind: "generated" } \| { kind: "pendingOtp", pendingId } }` → `{ clearsAt \| null, clearAfterSeconds \| null }` | `fieldEmpty`, `noOtp`, `clipboardUnavailable`, `otpPendingExpired`, `invalidInput` (HOTP) | live |
| `clear_clipboard` | `null` → `null` | `clipboardUnavailable` | live |
| **Generator, codes** | | | |
| `generate_password` | `{ kind: "password", length, lowercase, uppercase, digits, symbols, excludeAmbiguous } \| { kind: "passphrase", words, separator, capitalize, appendDigit } \| { kind: "forSite", host }` → `{ value, entropyBits, strength }` | `invalidInput` | live (works while locked) |
| `totp_code` | `{ vaultId, itemId }` → `{ code, secondsRemaining \| null, period \| null }` | `noOtp`, `vaultUnavailable` | live |
| **One-time code setup** | | | |
| `otp_scan` | `{ source: "screen" \| "clipboard" \| "file" }` → `{ kind: "found", pending: OtpPending, clipboardCleared } \| { kind: "cancelled" }` | `busy`, `otpScanUnsupported` (screen), `clipboardUnavailable`, `otpClipboardNoImage`, `otpImageUnreadable`, `otpImageTooLarge`, `otpQrNotFound`, `otpQrNotOtp`, `otpQrMigration`, `otpQrMultiple`, `otpQrInvalid` | live, needs unlocked; `cancelled` only for `file`; `clipboardCleared` is `true` only for `clipboard` |
| `otp_parse` | `{ kind: "uri", uri } \| { kind: "key", secret, issuer \| null, accountName \| null, algorithm, digits, period }` → `OtpPending` | `otpUriInvalid`, `otpQrMigration` (a pasted `otpauth-migration://` link), `otpKeyInvalid`, `invalidInput` (labels, digits 6–8, period 1–300) | live, needs unlocked; `key` always makes a TOTP |
| `otp_pending_code` | `{ pendingId }` → `{ code, secondsRemaining, period }` | `otpPendingExpired`, `invalidInput` (HOTP: no preview, the counter must not move before saving) | live, needs unlocked; a background refresh |
| `otp_pending_discard` | `{ pendingId }` → `null` | none (unknown IDs are ignored) | live, works while locked (no-op) |
| **Settings** | | | |
| `get_settings` | `null` → `Settings` | | live (stored in `settings.json` in the app data directory) |
| `update_settings` | `{ settings, password \| null }` → `Settings` | `reauthRequired` (turning off auto-lock, a lock trigger or clipboard clearing, or raising `quickUnlockMaxAgeDays`), `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `invalidInput` (no lock trigger this system can deliver is on, CRYPTO_SPEC §11; screen lock counts only where `screenLockDetection`), `locked` (a protection setting changes while locked; theme and language can; the protection settings carry a MAC that needs the unlocked vault, ADR-0030), `shortcutUnavailable` (the new `globalShortcut` could not be registered: another program holds it; the old one stays and nothing is stored), `shortcutUnsupported` (no global shortcuts here, Wayland), `invalidInput` also for a `globalShortcut` that `parse_accelerator` refuses | live |
| **Security insights** | | | |
| `security_report` | `null` → `{ generatedAt, itemsChecked, counts: { weak, reused, reusedGroups, old, missingTotp, insecureUrl, breached, high, medium, low }, findings: { vaultId, itemId, issues: { kind, severity, action }[], reusedWith: { vaultId, itemId }[], totpSite: { name, documentation \| null } \| null, insecureHost \| null }[] }` | | live (mosaic-insights, ADR-0022 to ADR-0024) |
| **Import** | | | |
| `import_choose_file` | `{ format }` → `{ kind: "chosen", fileToken, fileName } \| { kind: "cancelled" }` | | live |
| `import_preview` | `{ fileToken, format, password \| null }` → `{ previewId, format, entries: { index, kind, title \| null, subtitle \| null, duplicateOf: { vaultId, itemId } \| null, duplicateInFile \| null, attachments, warnings: ImportEntryWarning[] }[], skipped: { index, reason: "malformed" \| "empty" \| "tooLarge" }[], counts: { kind, count }[], attachments, attachmentBytes, warnings: ImportWarning[] }` | `importFileUnreadable`, `importFormatInvalid`, `importTooLarge`, `importUnsupported`, `importEncrypted`, `wrongCredentials` | live (mosaic-import, ADR-0021) |
| `import_commit` | `{ previewId, targetVaultId, skip }` → `{ imported, skipped, failed, attachments, attachmentsFailed }` (`imported` counts every item added, passkeys included) | `importPreviewExpired`, `vaultNotFound`, `vaultUnavailable` | live |
| `import_cancel` | `{ previewId }` → `null` | | live |
| **Export** | | | |
| `export_passkeys_cxf` | `{ vaultId, password: SecretIn \| null }` → `{ kind: "saved", count } \| { kind: "cancelled" }` | `reauthRequired` (no `password` and no right `reauthenticate` in the last two minutes; a success spends the re-authentication, a cancel or a failed write hands it back), `wrongCredentials` (counts towards the five-failure lock), `vaultNotFound`, `vaultUnavailable` (closed or hardware-protected), `itemNotFound` (no ES256 passkey in the vault; answered before the password is checked), `attachmentSaveFailed` (the file could not be written there), `locked`, `internal` | live (M5 T4, ADR-0041: Rust opens the native save dialog, builds a FIDO CXF v1.0 JSON file with the ES256 passkeys of the vault in a zeroizing buffer and writes it atomically, mode 0600, Mark of the Web on Windows; the file holds the private keys unencrypted) |
| **SSH agent** | | | |
| `ssh_agent_status` | `null` → `{ enabled, running, endpoint \| null, keyCount, skipped: { vaultId, itemId, reason: "unsupported" \| "malformed" \| "duplicate" }[], error \| null }` | | live |
| `ssh_agent_set_enabled` | `{ enabled }` → `SshAgentStatus` | `sshAgentUnavailable`, `sshAgentEndpointInUse`; turning it on needs unlocked | live (mosaic-ssh-agent, ADR-0025; off by default) |
| `ssh_respond` | `{ requestId, decision: "allowOnce" \| "allowFor15Minutes" \| "deny" }` → `null` | `sshRequestExpired`, `sshAllowTooSoon` (an allow less than 1 s after the prompt appeared is refused and the request stays open; a deny is always taken) | live |
| **Browser extension** | | | |
| `browser_status` | `null` → `{ endpoint: "running" \| "inUse" \| "unavailable", registry: "ok" \| "unreadable", pairings: { pairingId, label, browser: "chromium" \| "firefox", createdAt, lastUsedAt \| null, connected }[] }` | none | live |
| `browser_remove_pairing` | `{ pairingId }` → the same status | `browserPairingNotFound` (also for an ID that is not 32 hex digits). The pairing's live sessions get `unpaired` and close | live |
| `browser_pairing_respond` | `{ requestId, decision: "allow" \| "deny", label: string \| null }` → `null` | `browserRequestExpired`, `browserAllowTooSoon` (an allow less than 1 s after the prompt appeared is refused and the request stays open; a deny is always taken), `invalidInput` (a label that is not 1 to 64 characters, or contains control, format or default-ignorable characters; the request stays open) | live |
| `browser_save_respond` | `{ requestId, decision: { kind: "saveNew", vaultId, title, allowHttp } \| { kind: "update", vaultId, itemId } \| { kind: "cancel" } }` → `null` | `browserRequestExpired`, `browserAllowTooSoon` (as above; a cancel is always taken), `invalidInput` (a title that is not 1 to 256 characters or has spoofing characters), `vaultNotFound`, `vaultUnavailable` (closed, or hardware-protected), `itemNotFound` (an update of a login that is not one of the request's candidates, or gone), `locked`. The request stays open after all of them except the first | live |
| `browser_rotation_respond` | `{ requestId (the desktop's handle, from the event), decision: { kind: "allow" } \| { kind: "regenerate" } \| { kind: "confirm" } \| { kind: "keepOld" } \| { kind: "addOrigin" } \| { kind: "cancel" } }` → `null`. The answer to a `browserRotationRequested` dialog (M7b, ADR-0068). `allow` and `regenerate` fit a `begin` (`allow` stores the generated password as a new attempt of the login, one edit; `regenerate` keeps the dialog open with a new password that keeps the page's rules), `confirm` a `result` the user reported as succeeded (as `rotation_close` `succeeded`), `keepOld` a `result` (as `failed`), `addOrigin` an `addOrigin` (adds the origin to the login as an exact-origin URL), `cancel` fits all. The extension is answered with the outcome. No password and no address travels in the answer, and an unknown key or kind is refused | `browserRequestExpired`, `browserAllowTooSoon` (`allow`, `regenerate`'s follow-up `allow`, `confirm` and `addOrigin` less than 1 s after the dialog appeared or after a new password; `cancel`, `keepOld` and `regenerate` are always taken), `invalidInput` (a decision that does not fit the request), `itemNotFound`, `vaultUnavailable`, `rotationNotFound`, `rotationLimit`, `sharingRoleForbidden`, `locked`. The request stays open after all of them except the first | live |
| `browser_rotation_reveal` | `{ requestId }` → `{ value: SecretOut, hideAfterSeconds (15) }`: the password Rust generated for an open `begin` dialog, to show it on "Show". It is never in the event | `browserRequestExpired`, `invalidInput` (not a `begin`) | live |
| `browser_passkey_respond` | `{ requestId (the dialog's desktop-generated handle, from the event), decision: { kind: "create", vaultId } \| { kind: "assert", vaultId, itemId } \| { kind: "useAnotherDevice" } \| { kind: "cancel" } \| { kind: "acknowledge" }, password: SecretIn \| null }` → `null` | `browserRequestExpired` (also for the extension's own request ID or the handle of an earlier dialog), `browserAllowTooSoon` (a create, an assert or an acknowledge less than 1 s after the dialog appeared; a cancel and `useAnotherDevice` are always taken), `reauthRequired` (verification is needed now and no password came: the dialog asked for it with `needsPassword`, or the 300 s window ran out while the dialog was open, and the dialog then asks for the password and sends it with the next click), `wrongCredentials` (counts towards the five-failure lock of `reauthenticate`; a right password verifies the user for passkeys but earns no re-authentication grant for other commands), `secretKeyRequired` (the Secret Key is not on this device, so the password cannot be checked), store errors (`storeUnavailable` and the other storage codes of the vault commands), `internal`, `vaultNotFound`, `vaultUnavailable` (closed, or hardware-protected), `itemNotFound` (an assert of a passkey that is not one of the request's candidates, or no longer an answer), `invalidInput` (an answer that does not fit the request: `create` for a sign-in, `acknowledge` when not `excluded`, `create` or `useAnotherDevice` for an exclusion), `locked`. The request stays open after all of them except the first | live |

| `sync_status` | `null` → `SyncOverview` | `locked` | live |
| `sync_check_server` | `{ serverUrl }` → `{ serverUrl, registration: "open" \| "invite" \| "closed" }` (`serverUrl` is the canonical origin) | `syncServerInvalid`, `syncServerInsecure` (`http` to anything but this computer), `syncServerUnreachable`, `syncServerMismatch`, `syncServerIncompatible`, `invalidInput` (an address over 2048 bytes), `locked` (the app is locked and has an account: the probe is closed). Works while unlocked, and in the onboarding while the app has no account | live |
| `sync_hosted_connect` | `{ serverUrl, mode: "register" \| "signIn", password: SecretIn, inviteCode: SecretIn \| null, deviceName }` → `SyncOverview` | `locked`, `syncAlreadyConfigured`, `invalidInput` (device name over 64 characters, address over 2048 bytes, invite code over 128 bytes, a code that is not 6 to 8 ASCII digits, no email on file), `emptyPassword`, `wrongCredentials` (the master password, counted towards the five-failure lock; also a rejected server login), `secretKeyRequired`, `syncAccountMismatch` (sign-in: the server's account is not this device's), `syncPasswordChanged` (sign-in: the password was just verified on this device and the server refuses it, so the server holds a newer one; `SyncOverview.passwordChanged` turns `true` and `sync_recover_password` brings it in), `syncAccountExists`, `syncRegistrationClosed`, `syncInviteInvalid`, `syncTwoFactorRequired` (the login waits five minutes for `sync_hosted_two_factor`, or, M9, for a security key: `sync_login_methods`, `sync_hosted_webauthn_begin`), the server codes of `sync_check_server`, `syncRateLimited` | live |
| `sync_hosted_two_factor` | `{ code: SecretIn }` → `SyncOverview` | `syncSignedOut` (no login waits, or it ran out), `syncTwoFactorInvalid` (the login keeps waiting), `syncRateLimited` | live |
| `sync_pick_folder` | `null` → `{ choiceId, displayPath, contents: "empty" \| "mosaic" \| "other", accounts: AccountId[] } \| null` (`null`: the user cancelled) | the store's code when it is unavailable, `internal`. Works without an account too. The path stays in Rust; `choiceId` is random and expires after 5 minutes (ADR-0034) | live |
| `sync_folder_connect` | `{ choiceId, deviceName }` → `SyncOverview` | `locked`, `syncChoiceExpired`, `syncFolderNotEmpty`, `syncFolderUnavailable`, `syncAlreadyConfigured`, `invalidInput` | live |
| `sync_now` | `null` → `SyncOverview` (after the run ended, at most two minutes) | `locked`, `syncNotConfigured` | live |
| `sync_disconnect` | `null` → `SyncOverview` (signs out of the server best effort, deletes the token and the backend choice; the local data stays) | `locked`, `syncNotConfigured` | live |
| `sync_acknowledge` | `{}` (no fields, unknown keys refused) → `SyncOverview`. Rust asks **in a native OK/Cancel dialog** with its own texts (English or German by `settings.language`; `system` reads the OS locale; they equal the `sync.acknowledge.*` catalog entries, which a test checks); the webview can neither skip, reword nor answer it, and the slot is not held while it is open. Only on OK it clears the halt (`rollbackDetected`, `identityDuplicated`) and the findings against the server (`serverInconsistent`, `serverRollback`, `noCommonView`) and starts a sync; on Cancel it returns the overview unchanged. A build without dialogs answers Cancel. The user does this on every device | `locked`, `syncNotConfigured` | live (ADR-0047) |
| `sync_recover_password` | `{ newPassword: SecretIn, secretKey: SecretIn \| null, code: SecretIn \| null }` → `SyncOverview`. **M7: with `secretKey`** (the Secret Key from the newest Emergency Kit) the account's keys were renewed on another device. A key this device holds or held before (the current one or a retired commitment) is refused at once with `secretKeyUnchanged`. Hosted: the engine's `adopt_hosted_rekey` (login with the typed password and key, only the next epoch, a wrapping that opens under them); an equal epoch with the same account key (a Secret Key rotation elsewhere) falls back to the password path below with the typed key. Folder: `adopt_folder_rekey` (candidates ranked, at most eight Argon2id derivations, MAC checked), else the same fallback. The typed key becomes the key in use; quick unlock and security-key unlocks are removed; `settings.json` is protected again under the new account key; sync restarts (the hosted session is a fresh login with the adopted password, or `signedOut` if a second factor needs a new code). Without `secretKey`: The server or the folder holds a master password this device does not know (another device changed it, or this one was closed, killed or locked past the three-minute deferral between the backend's change and its own commit). Rust derives the keys from the typed password and the Secret Key under the backend's parameters (the join derivation), logs in (hosted) or reads the folder's verified account file, and adopts the change only if it opens the backend's wrapping to this session's own account key (CRYPTO_SPEC §9.4, `adopt_remote_password`). A wrong password counts towards the five-failure lock. Quick unlock is removed, the token of the login is kept, sync restarts | `locked`, `syncNotConfigured`, `emptyPassword`, `invalidInput` (a code that is not 6 to 8 digits), `wrongCredentials`, `secretKeyRequired`, `syncTwoFactorRequired`, `syncTwoFactorInvalid`, `syncFolderNoAccount`, | `syncPasswordRollback` (the backend offers a salt this device retired, or a folder file at or below the last version it wrote or adopted), `secretKeyMalformed`, `secretKeyMistyped`, `secretKeyUnchanged`, `rekeyNoAccountFile`, `syncEpochRollback`, `syncRekeyRefused`, transport codes | live (ADR-0047, M7) |
| `sync_devices` | `null` → `{ devices: { deviceId, name \| null, platform: "windows" \| "macos" \| "linux" \| null (this device only), current, lastSeenAt \| null, canRemove }[] }` | `locked`, `syncNotConfigured`, `syncSignedOut`, transport codes | live |
| `sync_device_rename` | `{ deviceId, name }` → `null` (hosted: any device; folder: this device only) | `locked`, `syncNotConfigured`, `invalidInput`, transport codes | live |
| `sync_device_remove` | `{ deviceId, password: SecretIn, code: SecretIn \| null }` → `null` (hosted only; a fresh login with the master password, plus a code when the account has a second factor; M9: or a security-key `confirm` from the last five minutes) | `syncDeviceIsCurrent`, `syncNotConfigured` (folder), `wrongCredentials`, `syncTwoFactorRequired` (a code is needed), `syncTwoFactorInvalid`, transport codes | live |
| `sync_totp_begin` | `{ password: SecretIn }` → `{ secret: SecretOut, otpauthUri: SecretOut }` (the fifth secret response, §6) | `invalidInput` (a second factor is on already), `wrongCredentials`, transport codes | live |
| `sync_totp_confirm` | `{ code: SecretIn }` → `null` | `syncTwoFactorInvalid`, `syncFreshLoginExpired` (more than five minutes since `sync_totp_begin`), transport codes | live |
| `sync_totp_disable` | `{ password: SecretIn, code: SecretIn \| null }` → `null` (M9: `code: null` when the session has a security-key `confirm` from the last five minutes, `sync_webauthn_confirm_begin`) | `wrongCredentials`, `syncTwoFactorInvalid`, `syncTwoFactorRequired` (the server accepts a code once: the first attempt spends the code on the login and answers this; send again with the next code), transport codes | live (M9: `code: null` after a security-key confirm) |
| `join_hosted` | `{ serverUrl, email, password: SecretIn, secretKey: SecretIn, deviceName }` → `{ state: "unlocked" } \| { state: "twoFactorRequired" }` (only without an account) | `accountExists`, `alreadyUnlocked`, `invalidInput`, `emptyPassword`, `secretKeyMalformed`, `secretKeyMistyped`, `wrongCredentials`, transport codes | live |
| `join_hosted_two_factor` | `{ code: SecretIn }` → `{ state: "unlocked" }` | `syncSignedOut`, `syncTwoFactorInvalid` (the join keeps waiting), the codes of `join_hosted` | live |
| `join_folder` | `{ choiceId, accountId, password: SecretIn, secretKey: SecretIn, deviceName }` → `{ state: "unlocked" }` | `syncChoiceExpired`, `syncFolderNoAccount`, `syncFolderUnavailable`, `syncFolderDamaged` (the account file's MAC fails; nothing was kept), `wrongCredentials`, the codes of `join_hosted` | live |
| `change_master_password` | `{ currentPassword: SecretIn, newPassword: SecretIn, code: SecretIn \| null }` → `null` | `locked`, `emptyPassword`, `wrongCredentials`, `secretKeyRequired`, `syncPasswordNeedsServer` (hosted sync is set up and the server is unreachable before the change was sent: nothing changed), `syncPasswordOutcomeUnknown` (the change was sent, its answer was lost and the check with the new password failed too: the device keeps both passwords and `passwordChanged` is set; `sync_recover_password` settles it), `syncTwoFactorRequired`, `syncPasswordChanged` (the server took the change but the local commit failed: the app was locked past the cap; `sync_recover_password` finishes it), `busy` (another change is running), transport codes. Removes quick unlock (CRYPTO_SPEC §6.2). While it runs a lock (manual, idle, screen lock) waits for it, for three minutes at most; a sleep locks at once (recovery covers it); the local commit follows the server's answer at once, and a token the keystore then refuses does not undo it (the device is `signedOut` and signs in again) | live (Settings > Password and keys, `ChangePasswordDialog`) |
| `sync_dismiss_account_file` | `{ name, sha256 }` → `SyncOverview`. An account file in the sync folder that does not verify with this device's keys (the warning `unverifiedAccountFile`) is remembered by name and SHA-256 and never reported again; nothing halts either way | `locked`, `invalidInput` (an empty or control-character name, a digest that is not 64 hex digits) | live (M7) |

### Security keys for the sync account, revocation reasons and other M9 commands (CRYPTO_SPEC §7.1, §9.3, §9.5, ADR-0077 to ADR-0085)

**Browser handoff.** A WebAuthn ceremony for the hosted account runs in the system browser on the server's page
(ADR-0077). Rust asks the server for a handoff, gets `{ handoffId, matchCode, expiresAt }`, and opens
`<canonical origin>/webauthn/#<handoffId>` through `Backend::open_handoff(origin, id)` in
`sync/webauthn_flow.rs`, the only call site of the `open` crate (`opener_has_one_call_site`). The page URL is
built by `mosaic_sync::http::hosted::handoff_page_url`, which refuses an origin that is not canonical. **No request type carries a URL**
and the webview never gets an opener. The webview shows the 6-digit `matchCode` in large mono type, "Re-open",
"Cancel" and the expiry, and polls the status command every 2 s. `matchCode` is not a `SecretOut`: it authorizes
nothing by itself (the page also needs the handoff ID from the browser's address bar and a touch of the key), and the
user has to read it. A lock, a cancel, a disconnect and the expiry drop the handoff in Rust.

| Command | Request → response | Errors besides `locked` / `internal` | Status |
|---|---|---|---|
| `sync_login_methods` | `null` → `{ totp: bool, webauthn: bool }` for the login (`sync_hosted_connect`, `sync_recover_password`) or join (`join_hosted`) that waits for its second factor (`GET /v1/login/methods`) | `syncSignedOut` (no login waits, or it ran out), transport codes | live (M9 T4) |
| `sync_hosted_webauthn_begin` | `null` → `{ matchCode, expiresAt }`. Starts the login handoff of the waiting `sync_hosted_connect` or `sync_recover_password` and opens the browser | `syncSignedOut`, `syncWebauthnUnavailable` (the server or the account has no security key), `syncRateLimited`, transport codes | live (M9 T4) |
| `join_hosted_webauthn_begin` | `null` → `{ matchCode, expiresAt }`. The same for the waiting `join_hosted` (only without an account) | the codes of `sync_hosted_webauthn_begin`, `accountExists`, `alreadyUnlocked` | live (M9 T4) |
| `sync_webauthn_status` | `null` → `{ state: "waiting" \| "done" \| "failed" \| "expired" }`. On `done` Rust runs the existing finish of the waiting connect (`finish_connect`, then read `sync_status`), recovery or join (`finish_join`: `session_state` becomes unlocked) before it answers | `syncSignedOut`, the codes of the finished connect, recovery or join, transport codes | live (M9 T4) |
| `sync_webauthn_reopen` | `null` → `null`. Opens the browser again for the running handoff (login, join or account) | `syncHandoffExpired` (none is running) | live (M9 T4) |
| `sync_webauthn_list` | `null` → `{ totp: bool, keys: { credentialRef, label, createdAt, lastUsedAt \| null }[] }` (`credentialRef`: 32 hex digits) | `syncNotConfigured` (folder), `syncSignedOut`, transport codes | live (M9 T4) |
| `sync_webauthn_add_begin` | `{ password: SecretIn, code: SecretIn \| null, label }` → `{ step: "login" \| "register", matchCode, expiresAt }`. A fresh login like `sync_totp_begin`; when the account has a second factor the login needs it: a TOTP `code`, or (without `code`, on an account with security keys) a login handoff first (`step: "login"`). Then the register handoff (`step: "register"`, purpose `register`, the label) | `invalidInput` (a label that is not 1 to 64 characters, a code that is not 6 to 8 digits), `wrongCredentials`, `syncTwoFactorRequired` (a code is needed and the account has no security key), `syncTwoFactorInvalid`, `syncWebauthnUnavailable`, `syncWebauthnLimit` (10 keys), `syncRateLimited`, transport codes | live (M9 T4) |
| `sync_webauthn_add_status` | `null` → `{ state: "waiting" \| "done" \| "failed" \| "expired", step: "login" \| "register" \| "confirm", matchCode \| null, expiresAt \| null }`. Serves `sync_webauthn_add_begin` and `sync_webauthn_confirm_begin`: when a login step is `done`, Rust starts the next handoff and answers `waiting` with the new step and code; `done` on `register` means the key is stored, on `confirm` that removals are allowed for five minutes | `syncFreshLoginExpired`, `syncSignedOut`, transport codes | live (M9 T4) |
| `sync_webauthn_confirm_begin` | `{ password: SecretIn }` → `{ step: "login" \| "confirm", matchCode, expiresAt }`. A fresh login (through a login handoff on an account with security keys) and then a `confirm` handoff, which sets the session's `second_factor_until` for five minutes | `wrongCredentials`, `syncWebauthnUnavailable`, `syncRateLimited`, transport codes | live (M9 T4) |
| `sync_webauthn_remove` | `{ credentialRef, code: SecretIn \| null }` → `null`. Needs the session of a fresh login and a TOTP `code` or a `confirm` from the last five minutes. The UI warns before the last factor goes (two-step verification then turns off) and recommends keeping two methods | `invalidInput`, `syncTwoFactorRequired` (no code and no recent confirm), `syncTwoFactorInvalid`, `syncFreshLoginExpired`, transport codes | live (M9 T4) |
| `vault_rotate_key` | `{ vaultId, resealAttachments: bool, proof: SecretIn \| null }` → `RotationDone`. "Rotate vault key…" in the vault menu (the top bar's "Vault actions"): re-authentication through the proof `reauthenticate` (`verify_master_password`) leaves, or the password itself as `proof`; `null` uses the proof (`reauthRequired` without one; a successful rotation spends it). A vault-only rotation (`RotationKind` `vault`) with the optional attachment re-seal loop, on the existing journal and `rotationProgress` events (`kind: "vault"`). `finished: false`: the key rotated but the re-seal was cut off (a failure after the commit is not an error); `rotation_resume` finishes it (the "Continue key rotation" banner, also after a restart). The re-seal runs only when this device's key won. The UI offers it for the open account-key vault on a device with sync, and (shared vault) to the owner only | `locked`, `reauthRequired`, `wrongCredentials`, `vaultNotFound`, `vaultUnavailable`, `rotationInProgress` (any rotation job is open), `syncNotConfigured` (a vault key is offered to a backend; a device without sync has none), the owner-only refusal of a shared vault, transport codes | live (M9 T6; `vault_rotate_key_reseals_attachments_and_resumes`) |

**How T4 wired it.** The handoff of a waiting login lives in that login (a lock, an expiry or the next sign-in drops
both); the handoff of `sync_webauthn_add_begin` and `sync_webauthn_confirm_begin` is one `AccountFlow` on the sync
service. `sync_hosted_webauthn_begin` and `sync_webauthn_status` also serve a fresh login that waits for a code
(`PendingKind::Fresh`); on `done` the new token is kept and the session counts as fresh. `matchCode` is 6 digits.
`sync_webauthn_reopen` serves whichever handoff runs. There is no cancel command: the webview forgets the step and the
handoff runs out in Rust. Mock options: `&syncWebauthn=both|only` (a key next to a code, or instead of one; the browser
step ends on its second status poll) and `&syncHandoff=expired|failed`. Not covered in M9: `sync_recover_password` for an
account with a security key only (it answers `syncTwoFactorRequired`; `TransportError::SecondFactorUnsupported`), and
removing a key with a code when the session is not fresh (the code path needs `code` on `sync_webauthn_confirm_begin`).

**Revocation reasons (CRYPTO_SPEC §9.5).** `SyncOverview.revoked` is `"rekey" \| "passwordChange" \| "deviceRemoved" \|
"operator" \| null`; it is set with status `signedOut` when the server answered `401` with a revocation detail, and the
recovery card shows `sync.revoked.<reason>` (for `rekey`: sign in with the new Secret Key from the newest Emergency
Kit, and the recovery form that asks for it, like `syncAccountRekeyed`). The reason is the server's claim and only
selects the text. It is `null` for a sign-out without a reason (an older server, a failed token save) and clears with
the next status. Live (M9 T6; `m9_ui_tests::the_revocation_reason_reaches_the_overview_until_the_next_sign_in`).

**Second factor in `SyncOverview`.** The hosted `config` gains `webauthnKeys` (a count) next to `totpEnabled`. Removals
that took `code: SecretIn` (`sync_totp_disable`, `sync_device_remove`) accept `code: null` when the session has a
`confirm` from the last five minutes.

**Other M9 additions.**
- `ApplyOutcome::MetaValueDropped` (CRYPTO_SPEC §7.1) reaches the webview as the event `vaultMetaDropped { vaultId,
  fields: ("name" | "icon" | "description")[] }`; the UI shows a notice the user can dismiss that names
  those fields (live, M9 T6: `MetaDroppedNotice` in the vault; mock `&metaDropped=name,description`). The event
  carries field names, never the dropped values.
- `settings.globalShortcut` (an accelerator string, validated in Rust by `shortcut::parse_accelerator`) is
  re-registered when it changes; a registration failure is `shortcutUnavailable`, on Wayland `shortcutUnsupported`
  and the field is disabled (live, M9 T6: Settings > Quick search shortcut). `parse_accelerator` takes modifiers plus
  one key (`Ctrl+Shift+Space`), at most 64 ASCII characters, and needs Ctrl, Alt or Command/Super (a bare key or
  Shift plus a key would take typing from every program). The default is `Ctrl+Shift+Space`, or `Alt+Super+Space` on
  macOS; a file with an unusable value reads as the default. The shortcut is not a protection setting (no MAC, changes
  while locked). `app_status.shortcut` is `unsupported` on Wayland. Mock: `&shortcut=conflict|unsupported|unavailable`.
- Two commands for the connector audit (the shape is live since M9 T6; **T8 wires them** to
  `mosaic_native_messaging::install::audit_manifests` and the rewrite, and until then both are stubs that answer no
  findings): `browser_connector_audit` (`null` → `{ findings: { browser: "chromium" \| "firefox", location, path }[] }`;
  `location` names the manifest file or registry key, `path` the host program it points to; run when the vault
  opens and when Settings > Browsers opens) and `browser_connector_repair` (`null` → the same shape after the
  repair; it rewrites the per-user entries to the installed host, never a path from the webview). A finding
  whose path is not the installed host shows the banner `browserConnectorForeign` (`BrowserConnectorBanner`: a
  warning with a Repair button, at the top of the vault and in Settings > Browsers, tested with mock findings:
  `&connector=foreign`). Repair goes to `browser_connector_repair`; on success the banner goes away and focus moves to
  the page.

### Account keys (M7, CRYPTO_SPEC §5.3, §6.4 to §6.7, ADR-0057 to ADR-0062)

The master password change is `change_master_password` above. The Secret Key rotation, the KDF profile upgrade
and the full re-key share one journal (`rotation_job`, one row, never a secret) and one rule: **the new Secret
Key is made in Rust, kept in the pending keystore entry and printed only into the new Emergency Kit** (the
webview never receives it); the backend learns nothing before that kit was saved and its last group typed back
(`kitNotVerified`). Every command takes the busy guard of the password change: a lock waits for the running step
(three minutes at most), the sync thread stops for the run, and the lock epoch is checked before any write.
Responses of the commits are `RotationDone { quickUnlockRemoved, hardwareKeysRemoved, unreachableAttachments,
unreachable: { itemTitle \| null, fileName \| null }[], finished }`; `unreachableAttachments` is the total of the
attachments the re-seal could not read (they keep their old file key, THREAT_MODEL R85) and `unreachable` names at most
the first 50 (display-cleaned); `finished: false` means the data follows from "Continue key rotation".

| Command | Request → response | Errors (besides `locked`) | Status |
|---|---|---|---|
| `rotation_status` | `null` → `{ job: null \| { kind: "secretKey" \| "rekey" \| "profile" \| "vault", phase: "prepared" \| "backendStarted" \| "backendAccepted" \| "folderWritten" \| "localCommitted" \| "attachments" \| "uploading", done, total, needs: "none" \| "password" \| "kitGroup" } }`. `needs`: `password` at phase 1 (the wizard starts again at the passwords, the pending key is reused), `kitGroup` at phase 2 or 3 of a Secret Key rotation or a re-key (a profile upgrade at phase 2 needs `password`), else `none` | | live |
| `secret_key_rotate_prepare` | `{ password: SecretIn }` → `null`. Checks the password (counted), opens the journal at phase 1, stages the new Secret Key (an open phase-1 job of the same kind keeps its key) | `emptyPassword`, `wrongCredentials`, `rotationInProgress` (another kind, or a job past phase 1) | live |
| `secret_key_rotate_commit` | `{ password: SecretIn, code: SecretIn \| null }` → `RotationDone`. Phase 2 before the backend is told, a fresh login (hosted) or a new account file (folder), the vault commit (a new salt, the old Secret Key retired by commitment), the key moves to the normal entry, quick unlock is removed, the job closes. Errors that prove nothing was sent put the job back at phase 1 | `kitNotVerified`, `rotationNone`, `wrongCredentials`, `secretKeyUnchanged`, `syncPasswordNeedsServer`, `syncPasswordOutcomeUnknown`, `syncPasswordChanged`, `syncTwoFactorRequired`, `syncTwoFactorInvalid`, transport codes | live |
| `profile_upgrade` | `{ password: SecretIn, profile: "standard" \| "strong" \| "max", code: SecretIn \| null, force: bool }` → `RotationDone`. No kit (the Secret Key is unchanged). Unless `force`, one Argon2id run with the new profile is timed on this device first | `profileDowngrade` (not stronger than the account's), `profileTooSlow` (more than two seconds), the codes of `secret_key_rotate_commit` | live |
| `rekey_review` | `null` → `{ ownedVaults, sharedOwned: { vaultId, name \| null, members }[], memberVaults: { vaultId, name \| null }[], hardwareVaults, grants, hardwareKeys, quickUnlock, unreachableAttachments, backend: "local" \| "hosted" \| "folder", accountId }` | `rekeyNeedsCleanSync` (a halt, finding or pending revocation is open) | live |
| `rekey_prepare` | `{ password: SecretIn, newPassword: SecretIn }` → `null`. The new password has to differ; opens the journal and stages the key | `wrongCredentials`, `passwordUnchanged`, `syncNotConfigured` (a re-key renews what a backend holds), `rotationInProgress` | live |
| `rekey_commit` | `{ password: SecretIn, newPassword: SecretIn, code: SecretIn \| null }` → `RotationDone`. `SyncEngine::rekey_hosted` (the engine's run to head, strict plan, atomic request, at most three attempts) or `rekey_folder`, then `rekey_continue` until the job closes (attachments within the download budget per call). After the vault committed a failure of the follow-up is not an error: `finished: false`. `rotationProgress` events follow the phases; the sync thread restarts with the new session | `kitNotVerified`, `passwordUnchanged`, `wrongCredentials`, `rekeyNeedsCleanSync`, `rekeyMemberKeyMismatch` (a member's safety code is not verified), `rekeyBusy`, `rekeyNeedsServer`, `rekeyOutcomeUnknown`, `syncTwoFactorRequired`, `syncRekeyRefused`, `syncEpochRollback`, transport codes | live |
| `rotation_resume` | `{ password: SecretIn \| null, kitGroup: SecretIn \| null, code: SecretIn \| null }` → `RotationDone`. By `needs`: `none` continues phases 4 to 7 of a re-key (and closes a finished credential job); `kitGroup` at phase 2 or 3: a re-key goes through `resume_hosted_rekey` (the pending key, the new password and the retyped last group; a probe at phase 2 that proves the server did not apply it returns the job to phase 1), a Secret Key rotation or profile upgrade probes by deriving the keys the change would have made and adopting them if the backend holds them; `password` (phase 1) is not resumable here: the wizard starts again | `rotationNone`, `invalidInput`, `kitGroupMismatch`, `rotationNotApplied`, `wrongCredentials`, the codes of `rekey_commit` | live |
| `rotation_abandon` | `null` → `null`. Drops a job the backend never saw (phase 1) and deletes the pending Secret Key | `rotationNone`, `rotationTooFar` | live |
| `rotation_abandon_committed` | `null` → `null`. Drops a job whose local part is committed (phases 5 to 7), for when "Continue key rotation" cannot finish (an attachment re-seal keeps failing). Attachments still waiting keep their old file key and are listed; the job stops blocking a change of backend. The UI confirms first and says so (`UnlockedSession::abandon_committed_rotation_job`, review L4) | `rotationNone`, `rotationTooFar` (before the local commit: phase 4 `folderWritten` and earlier; the banner offers it from phase 5 `localCommitted` on, after a continue that did not finish) | live |
| `sync_rekey_markers` | `null` → `{ name, sha256, toEpoch }[]`. The re-key notices in the sync folder this device is halted on and the user has not ignored (`SyncEngine::pending_rekey_markers`, review H1). Empty for a non-folder backend | `locked`, transport codes | live |
| `sync_dismiss_rekey_marker` | `{ name, sha256 }` → `SyncOverview`. Remembers a notice by name and SHA-256 (like `sync_dismiss_account_file`); the halt clears once no other notice is left. The UI asks first ("Ignore notice" / "Keep"): a genuine notice means the account was re-keyed elsewhere and this device stays on the old keys until it adopts | `invalidInput`, transport codes | live |
| `sync_ignored_rekey_markers` | `null` → `{ name, sha256, toEpoch }[]`. The re-key notices the user ignored that the folder still shows (`SyncEngine::dismissed_rekey_markers`, ADR-0064). Settings > Password and keys lists them as "Ignored re-key notices" only when the list is not empty. Empty for a non-folder backend | `locked`, transport codes | live |
| `sync_restore_rekey_marker` | `{ name, sha256 }` → `SyncOverview`. Undoes an ignore (`SyncEngine::undismiss_rekey_marker`): the notice halts sync again and `rekeyPending` is set so the re-key can be adopted. No confirmation (restoring is safe). Dismissed unverifiable account files (`sync_dismiss_account_file`) are not listed anywhere in the desktop, so they have no restore here | `invalidInput`, transport codes | live |

`SyncOverview` gains `rekeyPending` (a verified `.mrek` marker names this device's epoch: sync is paused, status
`error` with `syncAccountRekeyed`, and the in-app notice offers `sync_recover_password` with the new Secret Key). A hosted device gets the same status on start when the server holds other keys than the device and states a higher account-key epoch (`GET /v1/account?epoch=1`, compared with `>` only; an equal epoch stays `syncPasswordChanged`, a lower one is no re-key): the settings then show the recovery form with the Secret Key required instead of the password card,
and the warning `unverifiedAccountFile { name, sha256, conflict }` (dismissible; `conflict` when the file names
this device's own epoch, two devices re-keyed at once: the only resolution is to re-key again from this device).
There is no equal-epoch adoption. Not built in T5: `vault_rotate_key` (a per-vault action, not in the T5 brief; M9 adds
it, see the M9 table above) and the silent recovery at the next password unlock of CRYPTO_SPEC §9.4 (it would keep the
typed password in memory past the unlock; the recovery card asks again; closed as accepted in M9, ADR-0076).

Event `rotationProgress { kind, phase, done, total }` after each phase change: read `rotation_status` again.

### Credential rotation and the breach check (M7b, CRYPTO_SPEC §7.4, §12, ADR-0064 to ADR-0070)

Not the key rotation above: the guided password change of one login, the rotate-now queue and the opt-in breach
check. The new password of an attempt is stored in the item (sealed, queued for sync) **before** `rotation_begin`
returns; the item's own password changes only on `rotation_close` with `succeeded`; no value is ever deleted
(CRYPTO_SPEC §7.4). Nothing here opens a page or makes a request except the breach scan: the change page is
returned as a link to copy (OD-M7b-4; the browser extension opens it). A viewer of a shared vault gets
`sharingRoleForbidden` from every command that changes an item. Attempt and history IDs are short ASCII key
suffixes (`attemptId` at most 64 characters, `historyId` at most 128; anything else is `invalidInput`).

| Command | Request → response | Errors (besides `locked`) | Status |
|---|---|---|---|
| `rotation_queue` | `null` → `{ entries: { vaultId, itemId, title \| null, username \| null, host \| null, kind: "guided" \| "manual", reasons: ("weak" \| "reused" \| "old" \| "breached" \| "unfinished")[], severity: "low" \| "medium" \| "high", attempts: { attemptId, stage: "prepared" \| "delivered", startedAt }[], snoozedUntil \| null }[], breach: { mode: "off" \| "manual" \| "weekly", consentAt \| null, lastFinishedAt \| null, running, lastError: "offline" \| "rateLimited" \| "protocol" \| "cancelled" \| null } }`. Items with a weak, reused, old (the item's reminder interval, else a year) or breached password (the cached answers of the last scan, only for the password they were made for) or an open attempt. Dismissed items are left out (an open attempt is never dismissed away); snoozed ones carry `snoozedUntil`. `kind` is `guided` for a login with an `https` URL in a vault this account may write, else `manual`. Most severe first, then unfinished ones, then by title | | live |
| `rotation_begin` | `{ vaultId, itemId, newPassword: SecretIn, rulesSource: "siteQuirks" \| null }` → `{ attemptId, changeUrl: { url, source: "stored" \| "quirks" \| "wellKnown" } \| null }`. One edit, one sealed record, one outbox entry; returns after the local commit | `itemNotFound`, `itemDeleted`, `vaultUnavailable`, `rotationNotSupported` (not a login, or no `https` URL), `rotationLimit` (four attempts are open), `invalidInput` (empty, or over 1024 characters), `sharingRoleForbidden` | live |
| `rotation_close` | `{ vaultId, itemId, attemptId, outcome: "succeeded" \| "failed" }` → `null`. `succeeded`: the attempt's value becomes the password, the old one goes to the history as `replaced`, every other open attempt as `unconfirmed`. `failed`: the password stays, the value goes to the history as `unconfirmed` | `rotationNotFound`, `invalidInput`, `sharingRoleForbidden` | live |
| `rotation_reveal` | `{ vaultId, itemId, attemptId }` → `{ value: SecretOut, hideAfterSeconds }` | `rotationNotFound`, `itemNotFound` | live |
| `rotation_copy` | `{ vaultId, itemId, attemptId }` → `{ clearsAt \| null, clearAfterSeconds \| null }` (as `copy_to_clipboard`: Rust writes the clipboard, history exclusion, auto-clear). The copy marks the attempt `delivered` (ADR-0066): the value left the app | `rotationNotFound`, `clipboardUnavailable` | live |
| `password_history_restore` | `{ vaultId, itemId, historyId }` → `null`. The rollback: the entry's value becomes the password and the current password goes to the history (as `replaced`) | `itemNotFound`, `historyNotFound`, `invalidInput`, `sharingRoleForbidden` | live |
| `rotation_snooze` | `{ vaultId, itemId, days: 1 \| 7 \| 30 }` → `null`. Local state (store v10), no item change | `itemNotFound`, `itemDeleted`, `invalidInput` | live |
| `rotation_undismiss` | `{ vaultId, itemId }` → `null`. Takes back a dismissal (the "Undo" of the dismiss toast); a no-op without one | `itemNotFound`, `itemDeleted` | live |
| `rotation_dismiss` | `{ vaultId, itemId }` → `null`. Until the password changes (stores the value mark) | `itemNotFound`, `itemDeleted`, `fieldEmpty` (no password) | live |
| `rotation_set_policy` | `{ vaultId, itemId, everyDays: 30 \| 90 \| 180 \| 365 \| null }` → `null`. The reminder interval; "old" in the insights uses it | `itemNotFound`, `invalidInput`, `sharingRoleForbidden` | live |
| `breach_scan_start` | `null` → `null`; returns at once. Needs `Settings.breachCheck` not `off` with `breachConsentAt` (the consent dialog writes both through `update_settings`). Hashes the passwords under the session lock, then runs on its own thread with a plan that holds only hash suffixes; the session is locked again for the one moment that stores the answers. `GET https://api.pwnedpasswords.com/range/{prefix}` directly (`mosaic_sync::http::pwned`, ADR-0065): header `Add-Padding: true`, `User-Agent: Mosaic`, TLS 1.3, no redirect, body over 256 KiB rejected, prefixes shuffled, 100 to 400 ms apart, answers older than 30 days asked again | `breachConsentRequired`, `breachRunning`, `internal` (the client or the thread could not start) | live |
| `breach_scan_cancel` | `null` → `null`. Locking the vault, turning the check off in `update_settings` and quitting do the same at once: each request runs on a helper thread that the scan waits on together with the cancel flag, so the scan ends, and its plan of hash suffixes is dropped, within about 50 ms even while a request is on the wire (the helper ends with the 10 s timeout and holds one 5-character prefix). The pause between requests also re-reads the setting. A failing random number generator ends the scan with `protocol` before the next request | | live |

`ItemDetail` gains `history: { historyId, index, reason: "replaced" \| "unconfirmed", changedAt }[]` (newest first,
`index` is what `reveal_secret` takes), `rotationAttempts: { attemptId, stage, startedAt }[]` (oldest first),
`rotationPolicy: 30 \| 90 \| 180 \| 365 \| null` and `changeUrl: { url, source } \| null` (a login with an `https` URL;
the stored URL is the text field `changePasswordUrl`, `TextFieldKey` gains it, `https` only; one that is not on a
registrable domain of the login's URLs is ignored when the page is resolved). No value of an attempt is in a detail,
a queue entry or an event. `security_report` gains `counts.breached` and the issue kind `breached` (severity `high`,
action `changePassword`). `Settings` gains `breachCheck: "off" \| "manual" \| "weekly"` (default `off`),
`breachConsentAt` (`null` exactly when `breachCheck` is `off`; a mode without it reads as `off`, turning the check off
withdraws it) and `rotationReminders` (default on, local). `breachCheck` and `breachConsentAt` are under the settings
MAC (ADR-0030): they count as protection settings (changing them needs the unlocked vault; the `settingsReset` notice
covers them), a file without a valid MAC reads them as `off`, and they enter the MAC bytes only while the check is on,
so the MAC of an install that never used it is unchanged. `weekly` starts a scan 60 seconds after an unlock when the
last one ended more than a week ago; nothing else starts one. New error codes: `rotationNotFound`, `rotationLimit`,
`rotationNotSupported`, `historyNotFound`, `originNotRelated` (the extension's add-origin: the address is not on the login's site), `breachConsentRequired`,
`breachRunning`, `rotationOpen` (`purge_item` of a login with an open change).

Events `breachScanProgress { done, total }` (the webview shows "n of m" and does not announce every step) and
`breachScanFinished { checked, found, error: "offline" \| "rateLimited" \| "protocol" \| "cancelled" \| null }`
(also after a cancel or a lock; the answers of a scan cut off by a lock are not stored).

### Sharing and emergency access (M6b, CRYPTO_SPEC §8, ADR-0048 to 0054)

Every command works with the Mosaic server only: with a sync folder, or with no sync, they answer `sharingNotHosted` (the UI says why: a folder cannot tell who may read a vault). They run on an engine of their own over the stored session token next to the sync thread, take the session one closure at a time, record the lock epoch first and drop their result after a lock. Safety codes are public values and travel as text (`XXXXX XXXXX XXXXX XXXXX XXXXXX`, comparison ignores case, spaces, hyphens and the Crockford aliases); no key, share or roster ever reaches the webview. Roles are `owner`, `manage`, `write`, `read`. **Verification before sharing is required:** every invite and every grant needs a contact whose code the user compared, and a key that changed blocks sharing until the new code was compared. **Rotation is owner-only:** a manager invites (with the current key) and removes (the server drops the membership at once); the owner's device then changes the vault key at its next run.

| Command | Request → response | Errors (besides `locked` and the transport codes) | State |
|---|---|---|---|
| `sharing_identity` | `null` → `{ safetyCode \| null (until the first run made the key), published }` | `sharingNotHosted` | live |
| `sharing_lookup` | `{ email }` → `{ accountId, email, safetyCode, status: "new" \| "pinned" \| "verified" \| "changed" \| "conflicting" }`. Looks the address up and pins the key the server serves (unverified) when it is new; an address nobody owns gets a made-up key (no promise that an account exists) | `invalidInput`, `syncRateLimited`, `sharingNotHosted` | live |
| `sharing_verify_contact` | `{ accountId, safetyCode }` → `null`. The code as shown; it must be the code of the key pinned now (the attestation is the user's click on "It matches") | `sharingContactUnverified` (no such contact), `sharingKeyChanged` (the pin moved, or the code is not its) | live |
| `sharing_accept_key_change` | `{ accountId, safetyCode }` → `null`. Looks the address up again; the served key's code must be the one the user compared; the pin is replaced, **unverified** (the UI follows with `sharing_verify_contact` for the same code) | `sharingKeyChanged`, `sharingNotHosted`, `invalidInput` | live |
| `sharing_contacts` | `null` → `{ contacts: { accountId, email, safetyCode, verified, changed (devices pinned different keys) }[] }` | | live |
| `vault_share` | `{ vaultId, accountId, role: "manage" \| "write" \| "read" }` → `{ inviteId, expiresAt }` | `sharingContactUnverified`, `sharingKeyChanged`, `sharingRoleForbidden`, `sharingInviteExists`, `sharingAlreadyMember`, `sharingReinviteBlocked`, `sharingInviteLimit`, `vaultUnavailable` (a hardware vault), `invalidInput` (role `owner`) | live |
| `vault_members` | `{ vaultId }` → `{ ownRole, members: { accountId, email \| null, role, verified, self, state: "member" \| "invited" \| "unconfirmed" \| "dropped" \| "removing", origin: "owner" \| "invite" \| "emergency" \| null }[], pendingRotation, unprotectedAttachments }`. `unprotectedAttachments` is a count (the attachments the last key change of this device could not re-seal because this device does not hold them; kept in memory) | `vaultNotFound`, `sharingRoleForbidden` (the server refuses this account the vault) | live |
| `vault_member_set_role` | `{ vaultId, accountId, role }` → `null` (never to or from `owner`) | `sharingRoleForbidden`, `invalidInput` | live |
| `vault_member_remove` | `{ vaultId, accountId }` → `{ unreachableAttachments }`. Rust asks in a **native** dialog (texts equal `sharing.members.remove.*`, a test checks): "{email} keeps whatever they already downloaded. Mosaic changes the vault key now, and they can't read anything new." for the owner, a variant for a manager (the owner's device changes the key at its next run, until then only the server keeps the person out) | `cancelled` (the user said no), `sharingRoleForbidden`, `invalidInput` (not a member) | live |
| `vault_invite_cancel` | `{ inviteId }` → `null` (a delivered invite queues a revocation on the owner's device) | `sharingInviteExpired` | live |
| `vault_leave` | `{ vaultId }` → `null` (native confirm with the buttons "Leave vault" and "Cancel", `sharing.members.leave.*`; the local copy stays readable) | `cancelled`, `sharingOwnerCannotLeave` | live |
| `invites_list` | `null` → `{ incoming: { inviteId, senderEmail, role, expiresAt }[], outgoing: { inviteId, vaultId, vaultName \| null, recipientEmail \| null, role, expiresAt }[] }` (outgoing: the ones still waiting) | | live |
| `invite_preview` | `{ inviteId }` → `{ vaultName \| null, role, senderAccountId, senderEmail, safetyCode, senderStatus, ownerAccountId, ownerEmail, ownerCode, ownerStatus }`. Opens the seal and runs every check of CRYPTO_SPEC §8.2; the key waits in the session for five minutes | `sharingInviteExpired` (not listed), `sharingInviteUnreadable`, `sharingInviteNotInRoster`, `sharingKeyChanged`, `sharingVaultKnown` (this device knows the vault already) | live |
| `invite_accept` | `{ inviteId, safetyCode \| null }` → `null`. `safetyCode` is the sender's code the user just compared (`null` when the sender is verified already). **A vault whose owner is not the sender needs the owner verified first** (`sharing_verify_contact` with `ownerCode`); `syncDataChanged` follows | `sharingContactUnverified` (sender or owner), `sharingKeyChanged`, `sharingInviteStale` (a key change since the preview, or the preview ran out: open it again), `sharingInviteExpired`, `sharingInviteNotInRoster` | live |
| `invite_decline` | `{ inviteId, block }` → `null` | `sharingInviteExpired` | live |
| `emergency_overview` | `null` → `{ granted: { granteeAccountId, email, waitDays, scope: { kind: "all" } \| { kind: "vaults", vaultIds }, state: "idle" \| "waiting" \| "approved" \| "released", requestedAt \| null, releaseAt \| null }[], received: { grantorAccountId, email, waitDays, state, requestedAt \| null, releaseAt \| null, cooldownUntil \| null }[] }` | `sharingNotHosted` | live |
| `emergency_grant` | `{ accountId, scope, waitDays: 1 \| 2 \| 7 \| 14 \| 30, password: SecretIn, code: SecretIn \| null }` → `null`. A fresh login with the master password (a code when the sync account has a second factor). **Vault keys only**, never the account; the wait is kept by the server | `wrongCredentials`, `syncTwoFactorRequired`, `sharingContactUnverified`, `sharingKeyChanged`, `sharingRoleForbidden`, `emergencyWaiting`, `invalidInput` (wait not listed, empty or over 64 vaults) | live |
| `emergency_revoke` | `{ accountId }` → `null` (any state; a released grant also changes the keys of the vaults it opened) | | live |
| `emergency_request` | `{ grantorAccountId }` → `{ releaseAt }` | `emergencyCooldown` (24 hours after a refusal), `emergencyNotGranted` | live |
| `emergency_respond` | `{ granteeAccountId, decision: "decline" \| "approve", password: SecretIn \| null, code: SecretIn \| null }` → `null` (approve needs the password: a fresh login) | `emptyPassword`, `wrongCredentials`, `syncTwoFactorRequired`, `emergencyNotGranted` | live |
| `emergency_claim` | `{ grantorAccountId }` → `{ vaults }` (read-only vaults; `syncDataChanged` follows) | `emergencyWaiting`, `emergencyNotGranted`, `sharingContactUnverified` (verify the grantor first), `sharingKeyChanged` | live |

Notes per area:

- **Sharing (M6b).** `SyncOverview` gains `sharingFindings`: `identityMismatch`,
  `rosterInvalid`, `shareUntrusted` (`{ vaultName, senderEmail }`), `memberUnverified`,
  `memberKeyMismatch` (`{ vaultId, vaultName, accountId, email }`: the server served a key that
  does not match the verified one, so the member was removed or the invite cancelled), `vaultHeld`
  (`{ vaultId, vaultName }`: nobody here confirms who owns the vault, so it waits for another
  device), `memberDropped` (`{ vaultId, vaultName, accountId, email }`) and `accessLost` (`{ vaultId,
  vaultName }`). A member finding is shown only while it is true (the member is still unverified,
  still on the roster); `accessLost` is a state and stays. `sync_acknowledge` also clears the
  stored findings. `SyncWarningView` gains `rotationBlocked` (`{ vaultName }`): this device owns a
  vault whose key change after a removal could not be done. `sync_disconnect` and a backend change
  answer `syncHasSharedVaults` while a shared vault is reachable (checked before anything is signed
  out or forgotten; the engine refuses it, too: CRYPTO_SPEC §10.2). Folder mode cannot share:
  every sharing command answers `sharingNotHosted`. The event `sharingChanged { invites,
  emergencyRequests }` follows a sync run whose counts differ from the last event; the UI
  reads `invites_list` and `emergency_overview` again. Emergency requests have no email: the
  grantor's app shows a banner until the request is refused, approved or released.
- **Sync (M6).** `SyncOverview` is `{ config: null \| { kind: "hosted", serverUrl, email,
  deviceName, totpEnabled } \| { kind: "folder", displayPath, deviceName }, state: { status,
  lastSyncedAt \| null, error \| null }, warnings: SyncWarningView[] }` (the spec calls it
  `SyncStatus`; that name is the `status` enum). `passwordChanged` is `true` while the server or the folder holds a master password this device does not know. `status` is `notConfigured`, `offline`,
  `syncing`, `upToDate`, `error` or `signedOut`. `SyncWarningView` is one of
  `freezeSuspected`, `missingFiles`, `noCommonView` (`{ deviceName \| null, since }`),
  `serverInconsistent`, `serverRollback` (`{ vaultName \| null }`), `rollbackDetected`,
  `identityDuplicated`, `undecryptable`, `hardwareVaultsLocal`, `attachmentsNotRepaired`
  (`{ count }` for the last three). While halted (`rollbackDetected`, `identityDuplicated`)
  the status is `error` with no code and the warning, and nothing retries until
  `sync_acknowledge`; the findings against the server stay until then too. The sync thread
  starts on unlock when sync is set up and stops on lock (every transport call then fails
  closed with `Locked`); sync work is not user activity for auto-lock. The hosted session
  token is kept in the keystore (`mosaic-sync-token-<account id>`), deleted on disconnect, on
  a 401 (`signedOut`) and when another device removes this one. The server allows one session
  per device: every fresh login (device removal, second-factor setup, password change)
  replaces this device's session, so Rust stops the sync thread, logs in, stores the token once
  its session is complete (a login that still waits for a missing or wrong second-factor code is
  kept in memory, and the next call with a code finishes it) and starts the thread again. Only
  `sync_totp_disable` reuses a fresh login (for four minutes, for its second code). A thread of an
  older generation writes nothing (status, warnings), and its 401 deletes the stored token only if
  it still is the one the thread ran with. A join stores the Secret Key in the keystore, writes the
  settings account block with `kitVerified = true` and leaves the app unlocked. Mutating
  item and vault commands poke the sync thread (debounced two seconds).

- **Items.** `ItemSummary` holds the vault, item, kind, title, subtitle (user name,
  cardholder, network name; never derived from a secret), host of the first URL,
  favorite, `hasOtp`, tags, `modifiedAt`, `trashedAt` and `needsReview`. `needsReview`
  covers sync conflicts, conflict copies and concurrent deletions, which CORE_API
  requires apps to show. `ItemDetail` adds the plain text fields, the names of the secret
  fields that hold a value, URLs, custom fields (hidden and OTP values left out),
  attachment metadata, the OTP configuration without its secret, the history count and
  the review state. `SecretRef` is
  `{ kind: "field", field } | { kind: "customField", elementId } | { kind: "history", index }`.
  `ItemDraft` is one edit, applied as one sync event. Its lists of text, secret, URL and
  custom-field changes go by element ID, so concurrent edits merge per element. Its
  `tags`, `otp` and `sshKey` take `null` for "unchanged". A custom-field `update` takes
  `value: null` to keep the stored value (a hidden or one-time-code field can be renamed
  without being revealed); that is only allowed when `kind` equals the stored kind,
  otherwise the result is `invalidInput`, as for an unknown element ID. The text, secret
  and custom-field changes the UI sends are limited to the item's kind (ADR-0033); the
  contract itself accepts any field on any item.
- **Passkeys.** `ItemDetail.passkey` is `null` or `{ rpId, userName, userDisplayName,
  credentialId (base64url), algorithm: "es256" \| "other", createdAt, signCount }`. The private
  key is part of the item's encrypted plaintext but not a `SecretFieldKey`: it appears in no
  summary, detail, event, reveal or copy (the red-team tests search every view for it), and a
  `passkey` item is edited only through its title, notes, tags and favorite in the app. For a
  passkey `ItemSummary.subtitle` is the user name (or the display name) and `host` the
  relying party ID. Passkeys are made and used through `browser_passkey_respond` (below).
- **SSH keys.** `ItemDraft.sshKey` is `null` or `{ op: "set", privateKey }`, for `sshKey`
  items only (`invalidInput` otherwise). Rust parses the pasted OpenSSH key without
  decrypting it, derives the public key and the SHA-256 fingerprint and stores all three
  as one field; text that is not an OpenSSH private key, or is longer than 16 KiB, is
  `sshKeyInvalid`. The passphrase is the item's `password` secret (what the agent reads).
  The material can be replaced, not removed. `ItemDetail.sshKey` is `null` or
  `{ publicKey, fingerprint, algorithm, encrypted, agentSupported }` (`agentSupported`
  is `false` for RSA, P-384 and other keys the agent cannot sign with). The private key
  never goes back to the webview: it is not a `SecretFieldKey`, there is no reveal, and
  `CopyField { kind: "sshPublicKey" }` copies the public key only (`fieldEmpty` without
  material).
- **OTP setup.** Rust captures the screen, reads the clipboard image or the chosen
  PNG/JPEG file, finds the QR code and parses the `otpauth://` URI (`mosaic-qr`,
  ADR-0031). The result stays in Rust as a pending slot (10 minutes, at most 4,
  cleared on lock); the webview gets `OtpPending` without the secret, the live code from
  `otp_pending_code` and saves with `OtpChange { op: "setPending", pendingId, issuer,
  accountName }`, which uses the slot up (a failed save keeps it). A pasted screenshot is
  read once and the clipboard is emptied afterwards if it still holds that image (`clipboardCleared`; also emptied, silently, when the code is a migration, several or invalid). `pendingId` is 32 hex characters, opaque. `app_status.
  otpScreenScan` is `true` on Windows and on Linux under X11 only. `OtpPending` is
  `{ pendingId, scheme, issuer, accountName, algorithm, digits, period, strength,
  expiresInSeconds }`; `strength` is `weak`, `belowRfc4226` or `rfc4226`.
- **Attachments** (ADR-0034). All file input and output happens in Rust, on paths that only a
  native dialog opened by Rust can produce; the webview gets no fs or dialog permission, and
  the four requests carry IDs only (`deny_unknown_fields`, so a `path` key is refused).
  - `AttachmentEntry` is `{ attachmentId, fileName, mime, size, addedAt \| null, available,
    previewable }`. `addedAt` comes from the time-ordered attachment ID. `available` says the
    encrypted file is in this device's store (a synced reference can arrive before its file).
    `previewable` is `available` and a PNG or JPEG. `mime` is a display hint (magic bytes
    first, then the extension) and decides nothing about security.
  - **Add.** The dialog has no filter. The size is checked before reading (more than 50 MiB
    is `attachmentTooLarge`; a file that grows during the read is refused too) and the file
    is read into a zeroizing buffer. A vault lock during the dialog or the read stores
    nothing (`locked`). The stored name has control and spoofing characters removed and is
    cut to 255 bytes. Hardware-protected vaults refuse files (`itemTypeNotAllowed`).
  - **Preview.** One call: decrypt, decode with `mosaic_qr::decode_image` (PNG and JPEG
    by magic bytes, at most 16384 px per side, 40 MP, 512 MiB of decoder memory), box
    downscale so that neither side exceeds 2048 px and the area stays at most 4194304
    pixels (at most 16 MiB plus the 8-byte header), return the pixels. No PDF or other
    format has a preview in this version: they get Save as only. A second preview while
    one runs is `busy`. The vault is checked again before the pixels are returned.
  - **Save as.** The dialog suggests the name with spoofing and control characters removed,
    `/ \ : * ? " < > |` replaced by `_`, trailing dots and spaces trimmed, a leading dot
    replaced, Windows device names (`CON`, `COM1`, ...) prefixed and a length of at most
    200 bytes with the extension kept. Rust decrypts only after the dialog returned a path
    and the vault is still unlocked with the same account. It writes a temporary file
    `.<16 hex>.mosaic-partial` in the same folder (created new, mode 0600 on Unix), syncs it
    and renames it over the target; on any error the temporary file is removed. On
    Windows the saved file gets the Mark of the Web (`ZoneId=3`, best effort). Overwriting
    relies on the native dialog's own confirmation.
  - **Remove.** The item and attachment must exist; the reference leaves the item and the
    encrypted file leaves the store.
  - `ErrorCode` gains `attachmentNotFound`, `attachmentUnavailable`,
    `attachmentFileUnreadable`, `attachmentSaveFailed`, `attachmentPreviewUnsupported` and
    `attachmentPreviewTooLarge`. `fixtures.rs` lists `attachment_preview` in
    `BINARY_RESPONSES`: it has a request sample and no JSON response sample, and the
    TypeScript parity test exempts it. In TypeScript its response is `ArrayBuffer`.
- **Clipboard.** Rust reads the value and writes the system clipboard itself. It marks
  the entry as concealed and excluded from clipboard history where the OS supports it.
  It clears the clipboard after `clipboardClearSeconds` (default 30 s) unless the user
  copied something else in the meantime, then emits `clipboardCleared`. The receipt
  drives the toast "Copied. Clears in 30 seconds." and its live region. `CopyField` is
  `username`, `password`, `totp`, `{ text, field }`, `{ secret, field }`,
  `{ customField, elementId }` or `sshPublicKey`.
- **Generator.** The generated value is shown in the generator, so it is returned.
  Rust also keeps it, so `{ kind: "generated" }` copies it without the webview sending
  it back. Rust drops it on the next generation and on lock.
- **Import.** Rust opens the native file dialog and reads the file. The webview gets
  an opaque `fileToken`, a preview with titles and duplicate hints, and the commit
  result. It never sees the export's plaintext. Parsed data is dropped on commit,
  cancel or lock. No dialog or fs permission is granted to the webview.
  Rust reads the file into a zeroizing buffer (refused above 256 MiB before reading) and parses it
  without holding the session. A new preview, commit, cancel or lock drops the parsed data. Entries
  with `duplicateOf` or `duplicateInFile` start unticked in the UI. `ImportEntryWarning` is
  `unmappedFields`, `otpUnreadable`, `attachmentTooLarge`, `attachmentMissing`, `tagsDropped`,
  `historyTruncated`, `passkeysNotImported`, `unknownCategory`, `fieldsDropped`, `archived` or
  `attachmentQuota`, `titleCleaned`, `passkeyUnreadable`, `passkeyUnsupported` or `passkeysLimit`.
  `ImportWarning` is
  `unmappedFields`, `attachmentsSkipped`, `entriesSkipped`, `otpUnreadable`, `passkeysNotImported`,
  `passkeyUnreadable`, `passkeyUnsupported` or `passkeysSkipped`. `ImportFormat` is `enpass`, `bitwarden`, `keepass`,
  `onePassword`, `cxf` or `csv`.
  Passkeys (M5, ADR-0041): Bitwarden, KeePassXC and CXF passkeys become `passkey` items of their own,
  one per credential, written right after their entry; `counts` has a `passkey` entry and
  `import_commit`'s `imported` counts them. An entry of passkeys only is listed with kind `passkey`
  and makes no other item. `passkeysNotImported` remains for 1PUX only (it holds no private
  keys); `passkeyUnreadable` means a passkey could not be decoded, `passkeyUnsupported` that its key
  is not ES256, `passkeysSkipped` / `passkeysLimit` that passkeys over 100 per entry or 10,000 per
  file were left out. A passkey already in the vault is skipped by `import_commit` and counted in
  `skipped`. No warning carries a value from the file.
- **Export** (ADR-0041). `export_passkeys_cxf` is the one command that writes private keys out of the
  vault. The webview sends a vault ID and the master password and gets a count back; Rust opens the
  save dialog, builds the file after the dialog returned a path, and writes it. The panel shows the
  warning "The file contains your passkeys unencrypted. Anyone with the file can sign in as you.
  Delete it after importing." before the password is asked for.
- **SSH agent.** Off by default, and it runs only while unlocked. Each sign request arrives as
  `sshApprovalRequested`, carrying:
  - the key item and its fingerprint;
  - the client process (`processName` and `pid`: both `null` on Windows, `pid` only on macOS);
  - `requestedAt` and `expiresAt`;
  - `canRemember`: when it is `false`, the UI hides "Allow for 15 minutes".

  The UI answers with `ssh_respond`. An unanswered request is refused on expiry, on lock, or when
  4 prompts are already open (`sshApprovalClosed`). A request that fails after approval arrives as
  `sshSignFailed`. `SSH_AUTH_SOCK` is `endpoint`.

- **Browser extension.** The extension reaches the desktop through the host binary and the
  local endpoint (`docs/NATIVE_MESSAGING.md`, ADR-0035 to ADR-0037); the webview sees only
  the paired browsers and two prompts.
  - **Pairing.** A host's `pairRequest` is shown while the vault is unlocked as
    `browserPairingRequested`: the six-symbol `code` (Crockford base32, shown `XXX-XXX`, the
    extension shows the same), the browser family the host verified and the `hint` the
    extension gave (display only). The user names the browser and answers with
    `browser_pairing_respond`; the desktop then draws the pairing ID and `K_pair` itself and
    hands them to the host over the endpoint. One request is open at a time: a second one
    closes both as `conflict`. An unanswered request ends after 120 s (`expired`), on lock
    (`locked`) or when the host hangs up (`disconnected`). Three denials within ten minutes
    block pairing requests for ten minutes; at most three requests a minute; at most 16
    pairings.
  - **Save.** `proposeSave` from a paired browser opens `browserSaveRequested` with the host,
    the captured user name, whether it is a `new` login or an `update` (the logins with the
    same user name are the `candidates`), a suggested title and `insecure` (the page is plain
    HTTP, not loopback). **The request carries no password**: it stays in Rust as a
    `SecretString` and is dropped on cancel, expiry (300 s), lock, disconnect of the asking
    session and removal of its pairing; the dialog
    shows a fixed mask. `saveNew` adds a login (user name, password, the page's origin as an
    exact-origin URL; `allowHttp` is honoured only when `insecure`); `update` changes the
    password only (the model moves the old one into the history). Only a candidate can be
    updated. Nothing is written without this answer.
  - **Passkeys.** `passkeyCreate` and `passkeyGet` (ADR-0039, ADR-0040) open
    `browserPasskeyRequested` after Rust checked the origin against the relying party ID and
    decoded the inputs: `kind: "create" \| "get"`, the browser's `origin`, the accepted `rpId`,
    for a create the sanitized `userName` and `userDisplayName` (else `null`), for a sign-in the
    `candidates` (`{ vaultId, itemId, vaultName, userName, userDisplayName }`, never empty: with
    no passkey the request is answered `passkeyNotFound` and no dialog opens), `excluded` (a
    create that lists a passkey Mosaic holds: the dialog only informs), `needsPassword` (the
    site asked for user verification and none happened in the last 300 s), `userVerification`
    and the times. **The request carries no challenge and no key material.** The user answers
    with `browser_passkey_respond`: the 1 s Allow guard applies to `create`, `assert` and `acknowledge`;
    `password` (a `SecretIn`; the user may send one whenever typed, it is checked only when
    verification is needed at that moment) is checked as `reauthenticate` does, so wrong
    passwords count towards the five-failure lock; a right one sets UV for 300 s but does not
    open the two-minute re-authentication grant that other commands spend. When the window
    runs out while the dialog is open, `reauthRequired` means "verification is needed now" and
    the dialog switches to password mode. The `requestId` of the event and of
    `browser_passkey_respond` is a handle the desktop makes for each dialog; the extension's own
    ID is kept only to match its `passkeyCancel` (a page that reuses an ID cannot steer an answer
    meant for another dialog). The answer is applied before the site is told: a create writes a new
    `passkey` item (title: the relying party ID) and an assertion with an imported counter above
    0 stores the raised counter first. One passkey request is open at a time; it ends on
    cancel, expiry (300 s), lock (the site is told `cancelled`), the asking session closing, its
    pairing being removed, or the page aborting (`passkeyCancel`, reason `abortedByPage`).
    User verification: `last_user_verification` is set at every unlock (password, quick unlock)
    and every right `reauthenticate`, cleared on lock; the UV flag is set only when the site did
    not say `discouraged` and the user was verified within 300 s.
  - **Unreadable list.** If `browser-pairings.json` exists but cannot be read (damaged,
    oversized, locked, wrong version), `browser_status` reports `registry: "unreadable"` with
    an empty list. The list is then read-only: no pairing request is shown, and
    `browser_remove_pairing` fails with `internal`. The file is never overwritten, so a
    transient error cannot cost every pairing. A missing file is `registry: "ok"`.
  - **Guards.** An allow less than 1 s after the prompt appeared is refused
    (`browserAllowTooSoon`); the UI also keeps its button disabled for 1 s, and again after the
    window regains focus. Labels and titles are checked with `is_spoofing_char`.
  - **Activity.** Only `fill` and `copy` from the extension, and a **completed** passkey create or
    sign-in, count as user activity (auto-lock); a passkey request that only opens a dialog, is
    cancelled, answered "another device" or fails does not, so a script cannot keep the vault
    open by asking. The webview's own answers to these prompts do.
  - **Events.** `browserPairingsChanged` follows every pairing, removal, connect and
    disconnect: read `browser_status` again. `browserPairingClosed`, `browserSaveClosed` and
    `browserPasskeyClosed` close the dialog with the reason `answered`, `expired`, `conflict`,
    `locked`, `disconnected` or (passkey only) `abortedByPage`.
  - `ErrorCode` gains `browserRequestExpired`, `browserAllowTooSoon` and
    `browserPairingNotFound`. Fixtures list the four commands and the five events; the
    contract module imports only `serde`, `mosaic-core` ids and contract types, because the
    desktop fuzz crate includes it.

## 4. Events (`AppEvent`, tag `type`)

| Event | Payload | Meaning |
|---|---|---|
| `locked` | `{ reason: "manual" \| "idle" \| "sleep" \| "screenLock" }` | The vault locked. Drop revealed secrets and show the unlock screen. |
| `syncStateChanged` | `{ state: { status, lastSyncedAt \| null, error \| null } }` | Status bar. `status` is `notConfigured`, `offline`, `syncing`, `upToDate`, `error` or `signedOut`. Read `sync_status` for the warnings. |
| `syncDataChanged` | `{ vaultIds }` | A sync run changed these vaults: read their items again. |
| `sharingChanged` | `{ invites, emergencyRequests }` | The invites that wait for this account or the emergency requests that wait for its answer changed after a sync run. Read `invites_list` / `emergency_overview` again; the grantor's banner and the sidebar count follow. |
| `breachScanProgress` | `{ done, total }` | A breach scan answered another prefix (M7b). Show "n of m"; do not announce it. |
| `breachScanFinished` | `{ checked, found, error \| null }` | A breach scan ended (M7b), also when cancelled or the vault locked. Read `rotation_queue` and `security_report` again. |
| `rotationProgress` | `{ kind, phase, done, total }` | A key change moved to another phase or counted an attachment (M7): read `rotation_status` again. The live region of the UI names the phase only; the counts are plain text. |
| `vaultMetaDropped` | `{ vaultId, fields: ("name" \| "icon" \| "description")[] }` | A merge of a vault's name, icon or description was too long to keep whole and the longer value won (CRYPTO_SPEC §7.1; from `SyncReport.meta_dropped`): show a notice the user can dismiss that names the fields in plain words. Field names only, never the values. |
| `syncAttention` | `{ conflicts, restored, replacedUnreadable }` | A run merged or repaired something (records merged with a local change, fields written back after a delete raced an edit, unreadable records replaced): show a short toast. |
| `clipboardCleared` | none | Rust cleared what Mosaic copied. |
| `sshApprovalRequested` | `{ request: SshApprovalRequest }` | Show the approval dialog (D18). |
| `sshApprovalClosed` | `{ requestId, reason: "answered" \| "expired" \| "locked" }` | Close it. |
| `sshSignFailed` | `{ requestId, vaultId, itemId, reason: "passphraseRequired" \| "wrongPassphrase" \| "keyMalformed" \| "kdfTooExpensive" \| "publicKeyMismatch" \| "unsupported" \| "internal" }` | A sign request failed after approval; show `ssh.signFailed.<reason>`. No secret. |
| `quickSearchRequested` | none | The global shortcut was pressed while unlocked; open the command palette. |
| `hardwareVaultClosed` | `{ vaultId }` | A hardware vault closed itself (timer or lock). Drop revealed codes and show the closed panel. |
| `browserPairingRequested` | `{ request: { requestId, browser, hint, code, requestedAt, expiresAt } }` | A browser asks to be paired: show the dialog (code, name field, Deny, Allow after 1 s). |
| `browserPairingClosed` | `{ requestId, reason: "answered" \| "expired" \| "conflict" \| "locked" \| "disconnected" }` | Close it. |
| `browserSaveRequested` | `{ request: { requestId, pairingLabel, origin, host, username \| null, kind: "new" \| "update", candidates, suggestedTitle, insecure, requestedAt, expiresAt } }` | Show the save dialog. No password. |
| `browserSaveClosed` | `{ requestId, reason }` | Close it. |
| `browserPasskeyRequested` | `{ request: { requestId, pairingLabel, kind: "create" \| "get", origin, rpId, userName \| null, userDisplayName \| null, candidates: { vaultId, itemId, vaultName, userName, userDisplayName }[], excluded, needsPassword, userVerification: "required" \| "preferred" \| "discouraged", requestedAt, expiresAt } }` | Show the passkey dialog (account list or vault choice, password field when `needsPassword`, "Use another device or security key", Cancel). No challenge, no key. |
| `browserPasskeyClosed` | `{ requestId, reason }` | Close it (`reason` may be `abortedByPage`). |
| `browserRotationRequested` | `{ request: { requestId, pairingLabel, kind: "begin" \| "result" \| "addOrigin", vaultId, itemId, title \| null, host, unicodeHost \| null, outcome: "succeeded" \| "failed" \| null, newOrigin \| null, attempt: { startedAt, stage: "prepared" \| "delivered", otherOpen } \| null, requestedAt, expiresAt } }` | Show the password-change dialog (M7b): a `begin` shows the generated password masked (Show, Generate another, Cancel, Use this password after 1 s), a `result` asks "Did {host} accept the new password?" (It worked after 1 s, Keep old password, Cancel) and says which attempt it closes (`attempt`, `result` only: when it started, how far it got, how many other changes of the login are open; no value; a capture of the pending password on the site opens it too, see `NATIVE_MESSAGING.md`), an `addOrigin` asks "Add {host} to {title}?" with an IDN warning. `host` is the page's host, for `addOrigin` the host to add. **No password**: fetch it with `browser_rotation_reveal`. |
| `browserRotationClosed` | `{ requestId, reason }` | Close it. |
| `browserPairingsChanged` | none | A browser was paired, removed, connected or disconnected: read `browser_status` again. |
| `settingsReset` | none | The first unlock after the start found weaker protection settings in `settings.json` without a valid integrity MAC (edited outside Mosaic, or from before the MAC) and reset them to the defaults (ADR-0030). Show the toast `toast.settingsReset` once. Sent at most once per app start. |

The latest `subscribe_events` call wins (one main window). After subscribing, the
webview re-reads `session_state`, since events are notifications and not a log.

## 5. Errors

`notImplemented`, `noAccount`, `accountExists`, `locked`, `alreadyUnlocked`,
`wrongCredentials`, `secretKeyRequired`, `secretKeyMalformed`, `secretKeyMistyped`,
`emptyPassword`, `quickUnlockUnavailable`, `storeInUse`, `storeUnavailable`,
`vaultNotFound`, `vaultUnavailable`, `itemNotFound`, `itemDeleted`, `sameVault`,
`noOtp`, `fieldEmpty`, `invalidInput`, `sshKeyInvalid`, `reauthRequired`, `attachmentTooLarge`,
`attachmentQuota`, `attachmentNotFound`, `attachmentUnavailable`,
`attachmentFileUnreadable`, `attachmentSaveFailed`, `attachmentPreviewUnsupported`,
`attachmentPreviewTooLarge`, `clipboardUnavailable`, `importFileUnreadable`,
`importFormatInvalid`, `importPreviewExpired`, `importTooLarge`, `importUnsupported`,
`importEncrypted`, `sshRequestExpired`, `sshAgentUnavailable`, `sshAgentEndpointInUse`, `unlockDelayed`, `tooManyAttempts`, `sshAllowTooSoon`,
`browserRequestExpired`, `browserAllowTooSoon`, `browserPairingNotFound`, `busy`,
`internal`, `cancelled`, `quickUnlockExpired`, `quickUnlockInvalidated`, `passwordRequired`,
`hardwareKeyUnsupported`, `hardwareKeyNotFound`, `hardwareKeyTimeout`,
`hardwareKeyPinRequired`, `hardwareKeyPinInvalid`, `hardwareKeyPinBlocked`,
`hardwareKeyNoPin`, `hardwareKeyNotRecognized`, `hardwareKeyLimit`, `hardwareKeyLast`,
`itemTypeNotAllowed`, `otpScanUnsupported`, `otpQrNotFound`, `otpQrNotOtp`,
`otpQrMigration`, `otpQrMultiple`, `otpQrInvalid`, `otpUriInvalid`, `otpKeyInvalid`,
`otpImageUnreadable`, `otpImageTooLarge`, `otpClipboardNoImage`, `otpPendingExpired`,
`sharingNotHosted`, `sharingContactUnverified`, `sharingKeyChanged`, `sharingRoleForbidden`, `sharingInviteExists`, `sharingInviteStale`, `sharingInviteExpired`, `sharingInviteUnreadable`, `sharingInviteNotInRoster`, `sharingOwnerCannotLeave`, `sharingAlreadyMember`, `sharingReinviteBlocked`, `sharingInviteLimit`, `sharingVaultKnown`, `syncHasSharedVaults`, `emergencyWaiting`, `emergencyCooldown`, `emergencyNotGranted`,
`syncNotConfigured`, `syncAlreadyConfigured`, `syncServerUnreachable`, `syncServerInvalid`, `syncServerInsecure`, `syncServerMismatch`, `syncServerIncompatible`, `syncRegistrationClosed`, `syncInviteInvalid`, `syncAccountExists`, `syncAccountMismatch`, `syncTwoFactorRequired`, `syncTwoFactorInvalid`, `syncSignedOut`, `syncRateLimited`, `syncQuotaExceeded`, `syncFolderUnavailable`, `syncFolderNotEmpty`, `syncFolderNoAccount`, `syncFolderDamaged`, `syncPasswordNeedsServer`, `syncPasswordOutcomeUnknown`, `syncDeviceIsCurrent`, `syncChoiceExpired`, `syncFreshLoginExpired`, `syncPasswordChanged`, `syncPasswordRollback`,
`syncWebauthnUnavailable`, `syncWebauthnLimit`, `syncHandoffExpired`, `syncHandoffFailed` (five wrong codes or failed checks on the page; start again), `shortcutUnavailable`, `shortcutUnsupported` (M9, live in T6),
`kitNotVerified`, `kitGroupMismatch`, `rotationInProgress`, `rotationNone`, `rotationTooFar`, `rotationJumpTooLarge` (a folder adoption or re-key target more than 16 epochs above this device's: it has to be set up again, remove the account here and join with the Emergency Kit; no "re-key again" is offered, THREAT_MODEL R96), `rotationNotApplied`, `secretKeyUnchanged`, `passwordUnchanged`, `profileDowngrade`, `profileTooSlow`, `rekeyNeedsCleanSync`, `rekeyMemberKeyMismatch`, `rekeyBusy`, `rekeyOutcomeUnknown`, `rekeyNeedsServer`, `rekeyNoAccountFile`, `syncAccountRekeyed`, `syncEpochRollback`, `syncRekeyRefused`.

The UI message for each code is `error.<code>` in `packages/i18n`. Rust maps
`mosaic-vault` and `mosaic-core` errors to codes in `contract/errors.rs`. Anything
without its own code becomes `internal`, which leaks nothing.

## 6. Secrets

| Direction | Type | Where |
|---|---|---|
| webview → Rust | `SecretIn` (deserializes into a zeroizing `SecretString`) | `password` and `secretKey` of `create_account` and `unlock`, the `password` of `export_passkeys_cxf`, `lastGroup`, secret field values, custom-field values and the SSH private key in `ItemDraft`, the `otpauth://` URI, the `uri` and `secret` of `otp_parse`, the KeePass file password, the re-authentication password in `update_settings`, the master password in `save_emergency_kit`, the `password` of `browser_passkey_respond`, the security-key `pin` of `unlock_quick` and the hardware-key and hardware-vault commands, the `password` of `enroll_biometric` and `enroll_hardware_key`, and (M6) the master password, Secret Key, second-factor codes and invite code of the sync and join commands and of `change_master_password`, and (M7) the passwords, the retyped last Secret Key group, the Secret Key typed from the newest kit and the codes of the account-key commands, and (M9) the `password` and `code` of `sync_webauthn_add_begin`, the `password` of `sync_webauthn_confirm_begin` and the `code` of `sync_webauthn_remove` |
| Rust → webview | `SecretOut` | `create_account` (the Secret Key, once), `reveal_secret`, `generate_password`, `totp_code`, `otp_pending_code`, `sync_totp_begin` (the sync account's second-factor setup key and link: the fifth), `rotation_reveal` (the new password of an open change attempt, M7b; `rotation_begin` takes it as a `SecretIn`) |

Preview pixels are a second Rust → webview flow of decrypted content. They are not a
`SecretOut` (they are binary, not JSON, and are not a string the UI can hide by clearing
state). `attachment_preview` answers with one `tauri::ipc::Response`, which arrives as an
`ArrayBuffer`: `[width u32 LE][height u32 LE][RGBA]`, downscaled in Rust, at most 2048 x 2048
and 4194304 pixels. The file itself never crosses the boundary, and nothing is encoded as
base64 or put in JSON. Rust keeps nothing between calls: the plaintext, the decoded image and
the downscaled pixels are zeroizing buffers dropped before the call returns, a failed decode
included (the response `Vec` that moves into Tauri is ordinary memory, and so is the scratch
memory of the PNG and JPEG decoders: the inflate window and the coefficient planes). The webview paints the pixels into a
`<canvas>` with `putImageData` (no `blob:`, `data:` or frame in the CSP), overwrites the
`ArrayBuffer` with zeros right after, and treats the dialog like a revealed secret: it closes
on lock and when the window loses focus, and clears the canvas and sets its size to 0 when it
goes. An answer that arrives after the dialog closed is zeroed and dropped. The frame carries
`data-secret` without `data-mask`, so the reveal snapshot never paints it.

`SecretOut` is the only contract type that serializes a secret, and a fixture test fails
if a secret sample shows up in any other response. Debug output of both types is
redacted. Neither type is `Clone`.

Residual exposure, accepted (THREAT_MODEL, desktop):
- Text the user types lives in the webview's JavaScript heap. JavaScript cannot zeroize
  it. The UI clears its state after submitting and on lock.
- The IPC request body that Tauri parses is a normal heap buffer and is not zeroized.
  The secret is moved into a zeroizing type at deserialization.
- The generated password of a browser password-change dialog stays in Rust (the event has no
  secret); "Show" fetches it with `browser_rotation_reveal` and it is hidden again after 15 s.
- A revealed secret, a generated password and a one-time code live in the webview until
  the UI hides them: after `hideAfterSeconds` (60), on blur, or on lock.
- Preview pixels of an image attachment live in Tauri's response buffer and the webview
  heap, and in the canvas until it is cleared. The webview wipes its copies as described
  above; the response `Vec` and any copy the webview engine keeps are not under Mosaic's
  control (THREAT_MODEL R35).

## 7. Mock IPC and development

`apps/desktop/src/ipc/mock.ts` implements the contract in memory, with handlers per area
in `src/ipc/mock/`. A command without a handler fails with `notImplemented`, like its
Rust stub. The mock is selected by build mode (`--mode mock`). Non-mock builds replace
`mock.ts` with a stub, so mock code and sample data never ship.

- `pnpm --filter @mosaic/desktop dev:mock`: the UI in a browser at
  http://localhost:1420. Options: `?mock=noAccount|locked|unlocked` and `&latency=300`.
  The sample account's master password is `mosaic`. Quick unlock:
  `&quick=biometric,hardwareKey` seeds an enrolled Windows Hello and one security key (and
  turns on key support, which the real backend still reports as unsupported), plus one
  closed hardware vault "Codes (security key)" with two codes; `&bioPlatform=windowsHello|touchId|none`,
  `&quickFail=<errorCode>` fails `unlock_quick` (expiry codes also drop the method),
  `&touchMs=<ms>` delays a key touch (cancel with `hardware_key_cancel`) and
  `&hwOpenSeconds=<s>` sets how long a hardware vault stays open (default 300). A security
  key needs the PIN `1234`. One-time code setup (ADR-0031): `&otpScan=<errorCode>|cancel`
  makes `otp_scan` fail with that code (or cancel the file dialog) and `&screenScan=0`
  reports screen scanning as unsupported. Attachments (ADR-0034): `&attachAdd=<errorCode>|cancel`
  and `&attachSave=<errorCode>|cancel` make `attachment_add` and `attachment_save` fail with that
  code or cancel the dialog; by default an add stores a 1.2 MB `scan-2.png`, and the "Passport scan"
  document starts with a PNG (previewable) and a PDF. The mock preview is a 64 x 48 gradient.
  Passkeys (ADR-0040): `&browserPasskey=get|create|excluded|uv|uvExpires` makes a browser ask 500 ms after
  subscribing to sign in (`uv`: the site wants user verification, the master password is `mosaic`;
  `uvExpires`: the first click is answered `reauthRequired`, as when the 300 s window ran out),
  to create a passkey, or to create one that the vault already holds; the sign-in variants add two
  passkeys for example.com (one per vault) to the sample items.
  Sharing (M6b): `&sharing=basic|empty|invites|emergency|request|findings|keyChanged|manager|viewer|rotation`
  seeds the sharing state and, unless `&sync=` says otherwise, a hosted sync account (sharing needs the
  Mosaic server; `&sync=folder` or `&sync=none` shows why it is off). `basic` is the owner of "Work" with
  Sam (verified, editor) and Kim (invited, not verified), plus Kim's invite waiting; `empty` has nobody;
  `invites` has three invites (Sam's from the vault's owner, Kim's as a manager of Sam's vault, so the
  owner's code is compared too, and one that cannot be opened); `emergency` has grants given and
  received in every state; `request` shows the grantor's banner for a waiting request; `findings` shows
  every sharing finding and member state; `keyChanged` makes Sam's key differ from the pinned one;
  `manager`, `viewer` and `rotation` seed those roles and a removal waiting for the owner's device with two
  files that could not be protected. The mock's people are sam, kim and lee at example.com; an
  address nobody owns gets a made-up key. The master password is `mosaic` and the one-time code `123456`.
  Account keys (M7, Settings > Password and keys): `&rotation=none|shared|resumePrepared|resumeAccepted|resumeAttachments|resumeSecretKey|rekeyElsewhere|rekeyPending|unknownFile|conflict|slowDevice`
  seeds the journal and the sync state (a hosted account, or a folder for the folder variants, unless `&sync=` says otherwise). `shared` gives the re-key review shared vaults, members,
  emergency contacts and security keys; `resume*` leave a journal row for "Continue key rotation" (`resumeAccepted` and `resumeSecretKey` ask for the last group of the new Secret Key
  `M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ3`, i.e. `8VJ3`, and the new master password); `rekeyElsewhere` is a hosted account signed out after a re-key on another device and
  `rekeyPending` a folder with a verified marker: both adopt with that Secret Key and the password `new correct horse battery staple`; a Secret Key the mock device used is refused
  (`secretKeyUnchanged`); `unknownFile` and `conflict` show the folder finding; `slowDevice` answers `profileTooSlow` until forced. `&rotationFail=<errorCode>` fails the commits and
  `&rotationStepMs=<ms>` sets the pause between the phases of a run (default 450).
  Credential rotation (M7b, Security > Rotate now): `&credRotation=queue|empty|pending|breachRunning|breachOffline|consent`
  and, for the browser's password-change dialogs, `browserBegin|browserResult|addOrigin` (one request 500 ms after the app subscribes: the
  generated password `q7#Vt2!mZp9@Lx4$Nw6&`, "Did github.com accept the new password?", "Add gist.github.com to GitHub?")
  (the spec's `?rotation=` is taken by the key rotation above; `&credRotationStepMs=<ms>` paces a scan, default 250).
  `queue` seeds weak, reused, old and breached items (GitHub is old and was found by the last scan, Figma and Jira share a
  weak password, the Wi-Fi password has no https page: "Edit item") with the breach check on (`manual`); `empty` has
  nothing to change; `pending` leaves an open change on GitHub (delivered) and a not-confirmed history entry;
  `breachRunning` shows a running weekly scan; `breachOffline` an offline last scan; `consent` the queue with the
  check off, so "Check now" asks for the consent. The mock's breach check finds `password`, `123456`, `hunter2`,
  `Sommer2024!` and `letmein`. The browser dialogs of the change (`browserBegin`, `browserResult`, `addOrigin`) belong to
  the extension track.
- `pnpm --filter @mosaic/desktop tauri dev`: the real app, running Vite on port 1420
  and Rust in debug mode (devtools available in debug builds only).
- `pnpm --filter @mosaic/desktop tauri build`: release bundles. The release build has no
  devtools and serves the frontend through the custom protocol under the strict CSP.
  A navigation guard (`navigation.rs`, ADR-0019 amendment) keeps every webview on the
  bundled frontend; only `tauri dev` builds may also load the Vite dev server.
- `pnpm test` runs component tests (Vitest, jsdom) against the mock.
  `pnpm test:e2e` runs Playwright and axe against a mock-mode production build, which
  the preview server serves with the production CSP from `tauri.conf.json`.

## 8. Changing the contract

1. Change the Rust type in `src-tauri/src/contract/` and its samples. For a new
   command, also add its name to `commands/names.rs`, its handler to
   `commands::handler`, and its permission to `capabilities/main.json`.
2. Run `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`.
3. Change the TypeScript type in `src/ipc/contract/`, and for a new command, the
   `Commands` map and `commandSet`.
4. `pnpm typecheck` checks every Rust sample against the TypeScript types.
   `pnpm test` compares the command list and every string enum in both directions.
   `cargo test -p mosaic-desktop` checks the handler, the capability, and that each
   request sample deserializes in Rust.
5. Update this file. Adding a field, variant or command is fine without an ADR.
   Removing or renaming one is a contract change for every track and needs a note in
   PROGRESS.md.
6. The desktop fuzz crate includes `src/contract/` by path. Contract modules must not import the
   feature crates (`mosaic-import`, `mosaic-insights`, `mosaic-ssh-agent`). A new request type also
   goes into `fuzz/fuzz_targets/ipc_requests.rs`, with a seed.
