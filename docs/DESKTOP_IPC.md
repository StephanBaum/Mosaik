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
`{ state: "unlocked", accountId, email }`. Every session command returns the new state.
The store can also be unavailable, for example when another process holds it. In that
case every command fails with `storeInUse` or `storeUnavailable`, and the UI shows that
sentence.

Account creation returns the Emergency Kit data once. The UI keeps the vault UI hidden
until the user has typed back the last group of the Secret Key (`verify_emergency_kit`,
UX_GUIDELINES §8). The Secret Key lives in a `SecretKeyStore`: the OS keystore (unlock
track), and in memory only until then. Without a stored key, the locked state reports
`secretKeyStored: false`, and `unlock` takes the key as typed from the kit. A typed key
that unlocks is stored.

## 3. Commands

Status: **live** means wired to `mosaic-vault` now. **stub** means the contract is fixed
and the command fails with `notImplemented` until its track lands (track in brackets).
The mock implements most commands for UI work (§7).

| Command | Request → response | Errors besides `locked` / `internal` | Status |
|---|---|---|---|
| **Session** | | | |
| `session_state` | `null` → `SessionState` | `storeInUse`, `storeUnavailable` | live |
| `create_account` | `{ email, password, defaultVaultName }` → `{ emergencyKit: { accountId, email, secretKey }, session }` | `accountExists`, `invalidInput`, `emptyPassword` | live (Argon2 calibrated on the device) |
| `verify_emergency_kit` | `{ lastGroup }` → `{ matches }` | `secretKeyRequired` | live |
| `unlock` | `{ password, secretKey \| null }` → `SessionState` | `wrongCredentials` (never says which secret), `secretKeyRequired`, `secretKeyMalformed`, `secretKeyMistyped`, `alreadyUnlocked`, `noAccount` | live |
| `unlock_quick` | `{ method: "biometric" \| "hardwareKey" }` → `SessionState` | `quickUnlockUnavailable` | stub (unlock) |
| `lock` | `null` → `SessionState` | none; locking a locked vault is a no-op | live |
| `subscribe_events` | a `Channel<AppEvent>` → `null` | none | live |
| **Vaults** | | | |
| `list_vaults` | `null` → `VaultSummary[]` (`vaultId`, `name \| null`, `itemCount`, `available`) | | live |
| `create_vault` | `{ name }` → `VaultSummary` | `invalidInput` | live |
| `rename_vault` | `{ vaultId, name }` → `VaultSummary` | `invalidInput`, `vaultNotFound`, `vaultUnavailable` | live |
| **Items** | | | |
| `list_items` | `{ vaultId \| null, trash: "exclude" \| "only" \| "include", favoritesOnly, query \| null }` → `ItemSummary[]` | | live |
| `get_item` | `{ vaultId, itemId }` → `ItemDetail` (secrets as field names only) | `vaultNotFound`, `itemNotFound`, `itemDeleted` | live |
| `reveal_secret` | `{ vaultId, itemId, secret: SecretRef }` → `{ value, hideAfterSeconds }` | `itemNotFound`, `fieldEmpty` | live |
| `add_item` | `{ vaultId, kind, draft: ItemDraft }` → `ItemDetail` | `invalidInput`, `vaultUnavailable` | stub (vault UI) |
| `update_item` | `{ vaultId, itemId, draft }` → `ItemDetail` | `invalidInput`, `itemNotFound` | stub (vault UI) |
| `set_favorite` | `{ vaultId, itemId, favorite }` → `ItemSummary` | `itemNotFound` | live |
| `trash_item`, `restore_item`, `purge_item` | `{ vaultId, itemId }` → `null` | `itemNotFound`, `itemDeleted` | live |
| `move_item` | `{ vaultId, itemId, targetVaultId }` → `{ vaultId, itemId }` (a new ID) | `sameVault`, `vaultNotFound`, `itemNotFound` | live |
| **Clipboard** | | | |
| `copy_to_clipboard` | `{ source: { kind: "item", vaultId, itemId, field: CopyField } \| { kind: "generated" } }` → `{ clearsAt \| null, clearAfterSeconds \| null }` | `fieldEmpty`, `noOtp`, `clipboardUnavailable` | stub (vault UI) |
| `clear_clipboard` | `null` → `null` | `clipboardUnavailable` | stub (vault UI) |
| **Generator, codes** | | | |
| `generate_password` | `{ kind: "password", length, lowercase, uppercase, digits, symbols, excludeAmbiguous } \| { kind: "passphrase", words, separator, capitalize, appendDigit } \| { kind: "forSite", host }` → `{ value, entropyBits, strength }` | `invalidInput` | live (works while locked) |
| `totp_code` | `{ vaultId, itemId }` → `{ code, secondsRemaining \| null, period \| null }` | `noOtp`, `vaultUnavailable` | stub (TOTP) |
| **Settings** | | | |
| `get_settings` | `null` → `Settings` | | live (in memory) |
| `update_settings` | `{ settings, password \| null }` → `Settings` | `reauthRequired` (turning auto-lock or clipboard clearing off), `invalidInput` | stub (settings) |
| **Security insights** | | | |
| `security_report` | `null` → `{ generatedAt, itemsChecked, counts, findings: { vaultId, itemId, kinds, reusedWith }[] }` | | stub (insights) |
| **Import** | | | |
| `import_choose_file` | `{ format }` → `{ kind: "chosen", fileToken, fileName } \| { kind: "cancelled" }` | | stub (import) |
| `import_preview` | `{ fileToken, format, password \| null }` → `{ previewId, entries, warnings }` | `importFileUnreadable`, `importFormatInvalid`, `wrongCredentials` | stub (import) |
| `import_commit` | `{ previewId, targetVaultId, skip }` → `{ imported, skipped }` | `importPreviewExpired`, `vaultNotFound` | stub (import) |
| `import_cancel` | `{ previewId }` → `null` | | stub (import) |
| **SSH agent** | | | |
| `ssh_agent_status` | `null` → `{ enabled, running, endpoint \| null, keyCount }` | | live (reports "off") |
| `ssh_agent_set_enabled` | `{ enabled }` → `SshAgentStatus` | `sshAgentUnavailable` | stub (SSH agent) |
| `ssh_respond` | `{ requestId, decision: "allowOnce" \| "allowUntilLock" \| "deny" }` → `null` | `sshRequestExpired` | stub (SSH agent) |

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
  `tags` and `otp` take `null` for "unchanged".
- **Clipboard.** Rust reads the value and writes the system clipboard itself. It marks
  the entry as concealed and excluded from clipboard history where the OS supports it.
  It clears the clipboard after `clipboardClearSeconds` (default 30 s) unless the user
  copied something else in the meantime, then emits `clipboardCleared`. The receipt
  drives the toast "Copied. Clears in 30 seconds." and its live region. `CopyField` is
  `username`, `password`, `totp`, `{ text, field }`, `{ secret, field }` or
  `{ customField, elementId }`.
- **Generator.** The generated value is shown in the generator, so it is returned.
  Rust also keeps it, so `{ kind: "generated" }` copies it without the webview sending
  it back. Rust drops it on the next generation and on lock.
- **Import.** Rust opens the native file dialog and reads the file. The webview gets
  an opaque `fileToken`, a preview with titles and duplicate hints, and the commit
  result. It never sees the export's plaintext. Parsed data is dropped on commit,
  cancel or lock. No dialog or fs permission is granted to the webview.
- **SSH agent.** Each sign request arrives as `sshApprovalRequested`, carrying the key
  item, its fingerprint, the client process and an expiry. The UI answers with
  `ssh_respond`. An unanswered request is refused on expiry or lock
  (`sshApprovalClosed`).

## 4. Events (`AppEvent`, tag `type`)

| Event | Payload | Meaning |
|---|---|---|
| `locked` | `{ reason: "manual" \| "idle" \| "sleep" \| "screenLock" }` | The vault locked. Drop revealed secrets and show the unlock screen. |
| `syncStateChanged` | `{ state: { status, lastSyncedAt \| null, error \| null } }` | Status bar. `status` is `notConfigured`, `offline`, `syncing`, `upToDate` or `error`. Sync itself is M6. |
| `clipboardCleared` | none | Rust cleared what Mosaic copied. |
| `sshApprovalRequested` | `{ request: SshApprovalRequest }` | Show the approval dialog (D18). |
| `sshApprovalClosed` | `{ requestId, reason: "answered" \| "expired" \| "locked" }` | Close it. |

The latest `subscribe_events` call wins (one main window). After subscribing, the
webview re-reads `session_state`, since events are notifications and not a log.

## 5. Errors

`notImplemented`, `noAccount`, `accountExists`, `locked`, `alreadyUnlocked`,
`wrongCredentials`, `secretKeyRequired`, `secretKeyMalformed`, `secretKeyMistyped`,
`emptyPassword`, `quickUnlockUnavailable`, `storeInUse`, `storeUnavailable`,
`vaultNotFound`, `vaultUnavailable`, `itemNotFound`, `itemDeleted`, `sameVault`,
`noOtp`, `fieldEmpty`, `invalidInput`, `reauthRequired`, `attachmentTooLarge`,
`attachmentQuota`, `clipboardUnavailable`, `importFileUnreadable`,
`importFormatInvalid`, `importPreviewExpired`, `sshRequestExpired`,
`sshAgentUnavailable`, `busy`, `internal`.

The UI message for each code is `error.<code>` in `packages/i18n`. Rust maps
`mosaic-vault` and `mosaic-core` errors to codes in `contract/errors.rs`. Anything
without its own code becomes `internal`, which leaks nothing.

## 6. Secrets

| Direction | Type | Where |
|---|---|---|
| webview → Rust | `SecretIn` (deserializes into a zeroizing `SecretString`) | `password` and `secretKey` of `create_account` and `unlock`, `lastGroup`, secret field values and custom-field values in `ItemDraft`, the `otpauth://` URI, the KeePass file password, the re-authentication password in `update_settings` |
| Rust → webview | `SecretOut` | `create_account` (the Secret Key, once), `reveal_secret`, `generate_password`, `totp_code` |

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

## 7. Mock IPC and development

`apps/desktop/src/ipc/mock.ts` implements the contract in memory, with handlers per area
in `src/ipc/mock/`. A command without a handler fails with `notImplemented`, like its
Rust stub. The mock is selected by build mode (`--mode mock`). Non-mock builds replace
`mock.ts` with a stub, so mock code and sample data never ship.

- `pnpm --filter @mosaic/desktop dev:mock`: the UI in a browser at
  http://localhost:1420. Options: `?mock=noAccount|locked|unlocked` and `&latency=300`.
  The sample account's master password is `mosaic`.
- `pnpm --filter @mosaic/desktop tauri dev`: the real app, running Vite on port 1420
  and Rust in debug mode (devtools available in debug builds only).
- `pnpm --filter @mosaic/desktop tauri build`: release bundles. The release build has no
  devtools and serves the frontend through the custom protocol under the strict CSP.
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
