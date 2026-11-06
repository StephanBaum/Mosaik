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
| `save_emergency_kit` | `{ labels: { title, intro, email, accountId, signInUrl, signInUrlValue, secretKey, masterPassword, masterPasswordHint, footer }, password \| null }` → `{ kind: "saved" } \| { kind: "cancelled" }` | `reauthRequired`, `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `secretKeyRequired`, `invalidInput` | live (Rust shows the save dialog, then renders and writes the PDF; no password needed from `create_account` until the kit is verified or the vault locks) |
| `verify_emergency_kit` | `{ lastGroup }` → `{ matches }` | `secretKeyRequired`, `tooManyAttempts` (the fifth wrong answer since the last unlock locks the vault and requires the password for the next unlock) | live (needs unlocked; a match sets `kitVerified`) |
| `unlock` | `{ password, secretKey \| null }` → `SessionState` | `wrongCredentials` (never says which secret; from the third in a row, `unlockDelayed` answers the next attempts for 1 s, 2 s, 4 s ... up to 30 s before any key work, nothing sleeps and nothing else is blocked; the UI shows `error.unlockDelayed` and the unlock button can retry after a few seconds; the wait lives in memory and resets on a right password or an app restart), `unlockDelayed`, `secretKeyRequired`, `secretKeyMalformed`, `secretKeyMistyped`, `alreadyUnlocked`, `noAccount` | live |
| `unlock_quick` | `{ method: "biometric" \| "hardwareKey", pin \| null }` → `SessionState` | `quickUnlockUnavailable`, `quickUnlockExpired`, `quickUnlockInvalidated`, `passwordRequired`, `cancelled`, `alreadyUnlocked`, `busy`, `hardwareKey*` codes | live (OS prompt; `pin` only for `hardwareKey`; without it a key that needs one fails with `hardwareKeyPinRequired`) |
| `lock` | `null` → `SessionState` | none; locking a locked vault is a no-op | live |
| `reauthenticate` | `{ password }` → `null` | `locked`, `secretKeyRequired`, `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `emptyPassword` | live (a right password opens a 2-minute window in which `enroll_*`, `add_hardware_vault_key` and `move_item` out of a hardware vault accept `password: null`; the UI sends those first without a password and calls this on `reauthRequired`, then retries) |
| `subscribe_events` | a `Channel<AppEvent>` → `null` | none | live |
| `app_status` | `null` → `{ shortcut: "registered" \| "conflict" \| "unavailable", screenLockDetection, otpScreenScan }` | none | live |
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
| `trash_item`, `restore_item`, `purge_item` | `{ vaultId, itemId }` → `null` | `itemNotFound`, `itemDeleted` | live |
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
| `update_settings` | `{ settings, password \| null }` → `Settings` | `reauthRequired` (turning off auto-lock, a lock trigger or clipboard clearing, or raising `quickUnlockMaxAgeDays`), `wrongCredentials` (five in a row lock the vault and require the password for the next unlock), `invalidInput` (no lock trigger this system can deliver is on, CRYPTO_SPEC §11; screen lock counts only where `screenLockDetection`), `locked` (a protection setting changes while locked; theme and language can; the protection settings carry a MAC that needs the unlocked vault, ADR-0030) | live |
| **Security insights** | | | |
| `security_report` | `null` → `{ generatedAt, itemsChecked, counts: { weak, reused, reusedGroups, old, missingTotp, insecureUrl, high, medium, low }, findings: { vaultId, itemId, issues: { kind, severity, action }[], reusedWith: { vaultId, itemId }[], totpSite: { name, documentation \| null } \| null, insecureHost \| null }[] }` | | live (mosaic-insights, ADR-0022 to ADR-0024) |
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
| `browser_passkey_respond` | `{ requestId (the dialog's desktop-generated handle, from the event), decision: { kind: "create", vaultId } \| { kind: "assert", vaultId, itemId } \| { kind: "useAnotherDevice" } \| { kind: "cancel" } \| { kind: "acknowledge" }, password: SecretIn \| null }` → `null` | `browserRequestExpired` (also for the extension's own request ID or the handle of an earlier dialog), `browserAllowTooSoon` (a create, an assert or an acknowledge less than 1 s after the dialog appeared; a cancel and `useAnotherDevice` are always taken), `reauthRequired` (verification is needed now and no password came: the dialog asked for it with `needsPassword`, or the 300 s window ran out while the dialog was open, and the dialog then asks for the password and sends it with the next click), `wrongCredentials` (counts towards the five-failure lock of `reauthenticate`; a right password verifies the user for passkeys but earns no re-authentication grant for other commands), `secretKeyRequired` (the Secret Key is not on this device, so the password cannot be checked), store errors (`storeUnavailable` and the other storage codes of the vault commands), `internal`, `vaultNotFound`, `vaultUnavailable` (closed, or hardware-protected), `itemNotFound` (an assert of a passkey that is not one of the request's candidates, or no longer an answer), `invalidInput` (an answer that does not fit the request: `create` for a sign-in, `acknowledge` when not `excluded`, `create` or `useAnotherDevice` for an exclusion), `locked`. The request stays open after all of them except the first | live |

Notes per area:

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
| `syncStateChanged` | `{ state: { status, lastSyncedAt \| null, error \| null } }` | Status bar. `status` is `notConfigured`, `offline`, `syncing`, `upToDate` or `error`. Sync itself is M6. |
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
`otpImageUnreadable`, `otpImageTooLarge`, `otpClipboardNoImage`, `otpPendingExpired`.

The UI message for each code is `error.<code>` in `packages/i18n`. Rust maps
`mosaic-vault` and `mosaic-core` errors to codes in `contract/errors.rs`. Anything
without its own code becomes `internal`, which leaks nothing.

## 6. Secrets

| Direction | Type | Where |
|---|---|---|
| webview → Rust | `SecretIn` (deserializes into a zeroizing `SecretString`) | `password` and `secretKey` of `create_account` and `unlock`, the `password` of `export_passkeys_cxf`, `lastGroup`, secret field values, custom-field values and the SSH private key in `ItemDraft`, the `otpauth://` URI, the `uri` and `secret` of `otp_parse`, the KeePass file password, the re-authentication password in `update_settings`, the master password in `save_emergency_kit`, the `password` of `browser_passkey_respond`, the security-key `pin` of `unlock_quick` and the hardware-key and hardware-vault commands, the `password` of `enroll_biometric` and `enroll_hardware_key` |
| Rust → webview | `SecretOut` | `create_account` (the Secret Key, once), `reveal_secret`, `generate_password`, `totp_code`, `otp_pending_code` |

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
