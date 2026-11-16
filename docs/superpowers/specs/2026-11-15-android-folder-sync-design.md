# Android folder sync: the user's own cloud folder on the phone

Status: draft for the owner, 2026-11-15. Replaces the default of OD-M10-11 for Android ("SAF later or never").
Owner decision (2026-11-15): Android folder sync is launch scope. The free plan syncs through the user's own cloud
folder, and the phone apps support hosted accounts only (`folderAccountUnsupported`), so a free user cannot use
the Android app today.

## 1. Intent

A free user who already syncs Mosaic between computers through Dropbox, OneDrive or Nextcloud can add their
Android phone in about a minute: scan the desktop's setup code, sign in to the cloud provider once, type the
master password. After that the phone syncs like a hosted account, with the same folder format, the same
integrity rules and no new cryptography. The desktop keeps writing into its local synced folder, the phone talks
to the same files in the cloud, and the provider still sees only ciphertext.

## 2. Goals, non-goals, anti-goals

**Goals**
- Join an existing folder account from Android, and sync it (pull, push, attachments, housekeeping, re-key
  following), for **Dropbox**, **OneDrive** (personal and work/school) and **Nextcloud / any WebDAV server**.
- The desktop's folder backend keeps its format and behaviour (CRYPTO_SPEC §10.1). The phone is one more
  device of the folder, bound by the same single-writer rule.
- A desktop setup code for folder accounts, so the phone needs no typing besides the master password.

**Non-goals (launch)**
- Google Drive and iCloud Drive on Android (§4: not feasible at launch without a cost or an ADR change;
  the owner decides, §10).
- Creating a folder account on the phone (the phone does not create hosted accounts either).
- Background sync while the phone is locked: sync needs the unlocked session, as for hosted accounts.
- Provider APIs on the desktop. The desktop keeps using the local folder.
- iOS. OD-M10-11 keeps iOS (iCloud Drive and File Provider) as its own track; the Rust parts here are reusable.

**Anti-goals** (failure even if it works)
- Any provider SDK in the app (Dropbox SDK, MSAL, Google Play services): they bring telemetry or proprietary
  code, and the Android app ships without Play services (ADR on mobile dependencies, DECISIONS "no Firebase, no
  Google Play services").
- Anything that leaves plaintext with the provider, or a Mosaic server in the middle of the OAuth flow or the
  file traffic.
- A provider token that can reach files outside the chosen Mosaic folder through Mosaic's code.
- A false "rollback" halt or silent data loss because a cloud listing lags.

## 3. Constraints

- **Never invent crypto.** The folder format, MACs, keys and join rules stay as in CRYPTO_SPEC §10.1. Nothing in
  this design adds a primitive. The new setup-code form (§7) is a spec change first, with an ADR.
- **The provider sees only §12.1.** API access adds metadata (§9), which goes into §12.1 before the code.
- **No telemetry, no Mosaic relay.** The phone talks straight to the provider over HTTPS. OAuth uses PKCE
  public clients, so no client secret ships and no Mosaic server takes part.
- **Secrets in zeroizing types.** Refresh tokens and WebDAV app passwords are secrets like the session token:
  stored through `SecureStorage` (Keystore AEAD, `noBackupFilesDir`, CRYPTO_SPEC §6.9.1), held in a non-`Clone`,
  non-`Debug` zeroizing type, never logged, never in an error. Access tokens live in memory only and are wiped on
  lock.
- **TLS** as today: the existing `reqwest`/`rustls` client, system anchors, TLS 1.3 only (ADR-0091).
- **i18n:** every string in `packages/i18n`, DE informal "du", EN. Error codes follow the `error.<code>` rule.
- **Rust owns the policy** (MOBILE.md §1). Kotlin opens the browser for sign-in and renders records.

## 4. Per-provider findings

How the desktop and the phone coexist: the desktop writes into the local folder that the provider's desktop client
syncs (for example `~/Dropbox/Mosaic`). The phone reads and writes **the same cloud path** through the provider.
The desktop client then downloads the phone's files. The single-writer rule (§10.1) means the two never write the
same path, so there's nothing to lock.

| Provider | SAF tree access on Android | Verdict | Mechanism at launch | Registration and review | Owner actions |
|---|---|---|---|---|---|
| **Dropbox** | No tree-capable provider: the app exposes no folder to other apps. This comes from Dropbox forum threads and CommonsWare. I found no official statement and did not test it on a 2026 build. | API needed | Dropbox API v2, OAuth code flow with PKCE (S256), `token_access_type=offline` (a refresh token, short-lived access tokens). Upload `mode=add, autorename=false, strict_conflict=true` for write-once names and `mode=overwrite` for replace, one call up to 150 MiB. `list_folder` (recursive) plus `continue` cursors for listing and changes. | App Console app, scoped access, **Full Dropbox** (§8). Scopes: `files.metadata.read`, `files.content.read`, `files.content.write`, `account_info.read` (the team root namespace). A development app can link up to 500 users. From 50 linked users there are two weeks to get production approval, or linking freezes. Review checks branding and least privilege. | Create the app, set the redirect URI and icon, apply for production at 50 users with the justification in §8. |
| **OneDrive** | The app's provider opens single files. Tree access and write-back are unreliable: KeePassium (2021) and KeePassDX users reported failed saves. Not re-verified on 2026. | API needed | Microsoft Graph, public client with PKCE, `Files.ReadWrite offline_access`. Root stored as `drive-id` plus `item-id` (survives a rename). `PUT …:/content` (simple upload, 250 MB max; the builder uses upload sessions above 4 MiB) with `conflictBehavior=fail` or `replace`. Children listing per directory. `delta` on a non-root folder works on personal OneDrive only (known limit, recheck), so work accounts list per directory. | Entra app registration, free, audience "personal and organisational accounts", mobile redirect URI. Publisher verification is free but needs a Microsoft AI Cloud Partner Program account, a work-tenant registration and a verified domain. Without it, tenants with risk-based step-up consent block user consent. Personal accounts are unaffected. | Register the app (ideally in a work tenant). Publisher verification once a domain exists (OD-M9-6, OD-M10-12). |
| **Nextcloud** | **Works in the source code.** The root sets `FLAG_SUPPORTS_IS_CHILD`, and the provider implements create, rename, move and delete. A read checks the server ETag and downloads. A write uploads **asynchronously after close** with overwrite, and listings come from a cache with background reload. So reads can be stale and a rename can race a pending upload. There is also an old report of a failed create. | SAF works but is weak; use the API | WebDAV in Rust: `PROPFIND Depth:1`, `PUT` to `.<hex>.mtmp`, then `MOVE` with `Overwrite: F` (write-once) or `T` (replace). This is the same temp-and-rename pattern as the desktop. Sign-in with Nextcloud Login Flow v2 (browser login, poll, an app password). Works for ownCloud and other WebDAV servers with a typed app password. | None. Login Flow v2 needs no client registration. | None. |
| **Google Drive** | The Drive provider does not support `ACTION_OPEN_DOCUMENT_TREE`, only single files. This comes from community reports (2019 to 2023); I found no official statement. | Not feasible at launch | `drive.file` (non-sensitive) covers only files the app created or the user picked. The desktop's files are written by Drive for desktop, so the phone would most likely not see them. That is community evidence; folder-picker inheritance is unverified. Full `drive` is a **restricted** scope: OAuth verification, then an annual security assessment (CASA). A third-party source cites about **US$540 per app per year** for Tier 2 through TAC Security; that is unverified for 2026. Google also disabled custom URI schemes and loopback redirects for Android clients (verified on its native-app OAuth page), so sign-in on Android needs the Play services authorization API, which conflicts with the no-Play-services rule. | Restricted-scope verification plus annual CASA, or a desktop Google Drive API backend plus Play services. | Decide (§10, question 1). |
| **iCloud Drive** | No Android app and no public iCloud Drive REST API. | Not feasible | None | None | Tell users (hosted, or another provider). |
| *Syncthing, FolderSync and other local-sync apps* | A real local folder reached through the system's `ExternalStorageProvider` tree. This is ordinary file semantics and reliable. | SAF ok (phase 2) | SAF store implemented in Kotlin through a UniFFI callback trait (§5.3) | None | None |

## 5. Architecture

### 5.1 One storage trait under the folder transport

`FolderTransport` today calls `std::fs` through `folder/fsutil.rs` and two direct calls in `writer.rs`. The
operations it needs are small and already funnel through `fsutil`:

- `list(dir)` and `list_capped(dir, limit, keep)`: name, is-dir, length, modification time. A missing directory
  has no entries. At most `MAX_SCANNED` (4096) items are looked at.
- `read_capped(path, max)`: a regular file up to `max` bytes (cap plus one); `None` if it is missing.
- `write_atomic(dir, name, bytes, replace)`: the file appears whole or not at all. With `replace == false` the
  name must be absent.
- `remove(path)` (a missing file is fine), `remove_empty_dir`, `remove_squatter` (file, directory or link on a
  reserved name), `exists`, `is_real_dir`, `create_dir_all`.
- Change detection: `change_digest` (path, size and mtime of every foreign file), polled by `wait_for_change`.

**Decision:** a `FolderStore` trait in `crates/sync/src/folder/store.rs` with exactly these operations. Paths are a
validated `RelPath` type built only from `Layout` components and canonical `folder_names`: no `..`, no
separators inside a component, always below the root. `Entry` gains an optional `version: Option<Vec<u8>>`
(Dropbox `rev`, Graph `eTag`, WebDAV `getetag`). The segment scan cache keys on it when present, otherwise on
(length, mtime) as today. There are two optional hooks:
- `begin_run()`, which lets a remote store fetch one recursive listing per run and answer `list` from it, with its
  own writes merged in;
- `stat(path)`, which is always authoritative (a direct metadata request) and is used for the rollback check
  (§6).

`init`, `inspect`, `list_accounts`, `has_account` and `join_from_folder` take `&mut dyn FolderStore` instead of
`&Path`.

- `LocalStore` is today's `fsutil`, moved behind the trait without changing behaviour: symlink refusal, the
  regular-file check before and after open, `O_NONBLOCK` on Unix, temp file plus `sync_all` plus rename. The
  desktop and CLI use it, and the existing folder tests run against it unchanged.
- Remote stores live in `folder/remote/{dropbox,graph,webdav}.rs` behind a new feature `folder-remote` (which
  needs `http`). Only the mobile crate enables it at launch.

### 5.2 Remote stores in Rust over HTTP, not Kotlin and SAF

The stores are Rust HTTP for the API providers, and **not** Kotlin callbacks over SAF. Why:
- SAF is not available for Dropbox, OneDrive or Google Drive (§4), so the API path is needed anyway.
- MOBILE.md §1 says "Rust owns sync". In Rust the provider rules (size caps, name validation, redirect handling,
  rate limits) are written once, host-tested and reusable on iOS later.
- No new HTTP stack: the existing blocking `reqwest` over `rustls` (ADR-0046, ADR-0091). WebDAV `PROPFIND`
  needs an XML reader: `quick-xml`, already in `Cargo.lock` through `plist`. Adding it directly to `mosaic-sync`
  needs a one-line justification in the commit. It is not a crypto dependency.

Rules every remote store follows:
- **Confinement.** Every request path is `root + RelPath`. A listing entry whose name fails `folder_names::parse`
  is dropped, as on disk. Graph `remoteItem` and `package` entries and Dropbox deleted entries are not entries.
- **Caps.** Listing responses are capped in bytes and entries. Downloads check the announced size against `max`
  and read through a cap of `max + 1`.
- **Redirects.** Graph `/content` answers 302 to a pre-authenticated download host. The store follows exactly one
  `https` redirect **without** the `Authorization` header. The existing client has redirects off; it is handled by
  hand.
- **Atomicity.** A provider upload is atomic per file: Dropbox and Graph commit the whole file, and WebDAV uses
  temp plus `MOVE`. Write-once uses the provider's conflict mode, never a check-then-write. A conflict maps to
  the same `Io` result as an existing name on disk.
- **Errors.** 401 after a refresh attempt becomes `folderProviderSignedOut`. 429 or 503 with `Retry-After`
  becomes `syncRateLimited`. A full quota (Dropbox `insufficient_space`, Graph 507) becomes `folderStorageFull`.
  Network errors become `syncServerUnreachable`. No error carries a path, a name or a provider message.
- **Tokens.** A `ProviderCredential` (refresh token or app password) lives in `Zeroizing`, is not `Clone`, has
  a redacted `Debug` and is stored under `folder-credential/<account>` in `SecureStorage`. Microsoft rotates
  refresh tokens, so the new one is stored before the access token is used. The access token lives in memory,
  inside the store, and goes when the session locks (the store is dropped with the transport).
- **Keep the transport alive.** `mosaic_mobile::Core` keeps the `FolderTransport` between runs while unlocked,
  so the scan cache survives and a run does not re-download every segment. It drops it on lock.

### 5.3 SAF store (phase 2, optional)

This is for Syncthing and other apps that keep a real local copy. A UniFFI callback trait `SafStore` (list,
read, write, rename, delete, query) is implemented in Kotlin with `DocumentsContract` on a persisted tree
permission. Writes go to a temp document followed by `renameDocument`. `createDocument` renames silently on a
collision ("name (1)"), so the store checks the returned name and deletes and fails on a mismatch. It is meant
only for local providers. The UI does not offer it for the Nextcloud app's provider, because of its asynchronous
upload.

### 5.4 FFI and storage on the phone

- `SyncConfig` keeps `kind = Folder`. A new device-local `FolderLocation { provider, root, server }` (a store
  migration) replaces `folder_path` on the phone. It is never synced.
- New `Core` calls, following the pattern of the WebAuthn handoff:
  - `folder_link_begin(provider, server?) -> LinkInfo { url, kind }` makes the PKCE verifier, `state` and
    expiry in Rust (a pending slot of five minutes, dropped on lock);
  - `folder_link_finish(callback_url)` checks `state` and exchanges the code;
  - `folder_link_status()` polls Login Flow v2;
  - `folder_locate(hint) -> Vec<FolderCandidate>` and `folder_browse(dir)` find the folder.
- `sync_once` and `recover_password` dispatch on the kind instead of refusing a folder account.
  `folderAccountUnsupported` stays only for a provider the phone cannot reach, renamed
  `folderProviderUnsupported`.
- Redirects: a custom scheme (`io.github.stephanbaum.mosaic.oauth:/cb`) with PKCE and `state` (RFC 8252 §7.1),
  opened in Custom Tabs, never in a WebView. Once a domain exists, verified App Links (`https`) replace it.
- New error codes (DE/EN): `folderProviderSignedOut`, `folderNotFound`, `folderStorageFull`,
  `folderLinkExpired`, `folderLinkInvalid`, `folderProviderUnsupported`.

## 6. Consistency and conflicts with provider delays

- **No locks are needed.** The single-writer rule holds: the phone writes only its own `d/<phone>/`,
  `log/<phone>/`, its own `.macct` and `.mdel`, and write-once blobs. A desktop client cannot make a conflict copy
  of a phone file, because no desktop writes it.
- **Lag between desktop and cloud.** The phone sees a desktop's change only after the desktop client has uploaded
  it. Files may arrive out of order or seconds apart, for example a segment before the key that opens it. The
  engine already handles this: an unverifiable file is retried, not marked read, and an unexplained object blocks
  a purge. The 24-hour lag finding stays as it is.
- **Listing staleness.** Graph `delta` and the Nextcloud cache can lag. Two rules prevent false alarms:
  - the store merges this run's own writes into its listings;
  - the **own-log rollback check** (§10.1) asks `stat()` for its expected names before it halts. Only a
    `stat` that says "missing" counts, never a missing entry in a listing.
  A file that cannot be read at the moment still counts as present, as on disk.
- **Atomic replace.** `head.mck` and `info.mdev` are replaced in one provider call, so a reader sees the old file
  or the new one.
- **Times.** Remote mtimes are the provider's server times. The engine uses them only as cache keys (a `version`
  token is preferred), as "newest" ordering of checkpoint candidates and as junk ages. Server time serves all
  three.
- **When the phone syncs.** As for hosted accounts: on unlock, after an edit, and on pull-to-refresh. It does not
  long-poll in the background. A run on a 3-device, 5-vault account does 1 or 2 listing requests on Dropbox and
  personal OneDrive (recursive or delta), about 25 `PROPFIND`s on WebDAV, plus the new segments.

## 7. Joining a folder account on the phone

**Setup code for folder accounts** (a spec change of CRYPTO_SPEC §6.9.7, with an ADR). The desktop shows it after
re-authentication, for 60 s, as for hosted accounts:
```
"mosaic-setup:2:" || base64url_unpadded({"v":2,"kind":"folder","account":"<uuid>","provider":"dropbox|onedrive|webdav|other","server":"<canonical origin, webdav only>","path":"<path hint below the provider root>","sk":"<SK display form>"})
```
- It carries no new secret: the account ID and the path are already visible to the provider (§12.1).
- The parser is strict, like v1.
- The hint comes from the desktop's provider client configuration: Dropbox `info.json`, the `OneDrive*`
  environment variables, the Nextcloud client config. This is best effort; with `other` the phone asks the user
  to choose.

**Flow**
1. The phone: "Use an existing account", then scan. It shows "This account syncs through your Dropbox folder."
2. "Sign in to Dropbox" opens a Custom Tab with OAuth PKCE and returns to the app. For WebDAV, the server
   address comes from the code or is typed, then Login Flow v2 runs.
3. `folder_locate(hint)` checks `mosaic-sync.json` and `a/<account>/` at the hinted path. If either is missing,
   the user browses to the folder. No full-drive search.
4. The user types the master password. `join_from_folder` runs over the remote store with the same candidate
   rules, MAC checks and `WrongCredentials`/`FolderDamaged` outcomes. Then comes the first pull.
5. Without a desktop: the user types the Secret Key, picks the provider, signs in and browses.

A wrong or hostile folder gains nothing: the join opens only an account file that the user's password and Secret
Key unwrap, and the version rule refuses a re-planted old wrapping (§10.1 "Joining").

## 8. Security analysis

- **Provider token scope.** Full Dropbox and Graph `Files.ReadWrite` reach every file of the user's cloud. They
  are needed because the desktop's folder already exists at an arbitrary path, and Dropbox team folders are
  out of reach of an App folder (owner direction on business clients). Mitigations:
  - Mosaic's code reaches only `root + RelPath`;
  - the token is Keystore-bound, excluded from backups and wiped from memory on lock;
  - on unlink or "Set up this phone again", the phone revokes what can be revoked: Dropbox
    `/2/auth/token/revoke`, Nextcloud `DELETE /ocs/v2.php/core/apppassword`. Microsoft has no per-app
    revocation for consumer refresh tokens that I could verify; the user removes the app on the account page.
  - Residual risk: a thief who breaks the Keystore gets cloud access beyond Mosaic. This is a new THREAT_MODEL
    entry. The narrower App-folder mode is question 2 in §10.
- **Malicious provider or malicious API answers.** This is the same model as a hostile folder (R54): the
  provider can delete, withhold, roll back or plant files. The MAC checks, the canonical-name parser, the caps,
  the own-log rollback halt and the checkpoint freeze and rollback rules apply unchanged. The API adds only
  bounded parsing (JSON and XML, capped) and the redirect rule of §5.2.
- **Rollback through version history.** Dropbox and OneDrive keep old versions. Restoring them is a rollback,
  which is already detected (own-log halt, checkpoints) and already in R54.
- **New metadata for §12.1.** The provider learns that the "Mosaic" API app wrote the phone's files, the phone's
  IP address, and the time and size of every sync run. A Nextcloud admin sees the app password's name. All of it
  goes into §12.1 before the code.
- **OAuth flow.** PKCE S256 with a fresh verifier per attempt, and `state` checked in constant time. A pending
  slot lives five minutes and is dropped on lock. Custom Tabs only. There is no client secret anywhere. A
  custom-scheme redirect can be claimed by another app, but PKCE makes an intercepted code useless.
- **No Mosaic server sees anything.** Neither tokens nor files pass through Mosaic infrastructure. This also
  keeps the app out of Google's "data on servers" assessment trigger, if Google Drive is ever added.
- **TLS 1.3 only** is kept. A self-hosted WebDAV server that offers only TLS 1.2 is refused, with a clear message.

## 9. What the owner must do

1. **Dropbox:** create the app (scoped, Full Dropbox, the four scopes, redirect URI, icon, branding). Apply for
   production when 50 users have linked, or earlier through "request early review".
2. **Microsoft:** create the Entra registration (free) for personal and organisational accounts with a mobile
   redirect and public client. Later, the free publisher verification (a Partner Program account, a verified
   domain).
3. **Google Drive:** decide (question 1).
4. **Privacy policy and Play data-safety form:** the phone connects to the provider the user chooses; Mosaic
   collects nothing.
5. **THREAT_MODEL, CRYPTO_SPEC §6.9.7 and §12.1, the ADR:** review them when the builder drafts them.

## 10. Open questions for the owner

1. **Google Drive:** (a) skip it at launch and tell Drive users to choose hosted or another provider; or (b) pay
   for restricted-scope verification plus the annual CASA (about US$540 a year, unverified) **and** accept
   Google Play services in the Android app; or (c) build a Google Drive API backend for the desktop as well
   (`drive.file`), which also needs Play services and a spike on `drive.file` visibility. I recommend (a).
2. **Scope breadth:** full-access scopes (any existing folder, team folders), or App-folder scopes (the Mosaic
   folder must sit in `Apps/Mosaic`; this is safer if a phone is compromised, but an existing setup has to
   move)? I chose full access with confinement.
3. **Self-hosted Nextcloud on TLS 1.2:** keep refusing it, or allow TLS 1.2 for WebDAV only?
4. **Launch order:** WebDAV and Nextcloud first (no registration, it can ship at once), then Dropbox and
   OneDrive as their registrations clear?

## 11. Calls I made (please check)

1. **Rust HTTP stores for the providers, not SAF.** SAF only as an optional phase-2 store for local-sync apps.
2. **No provider SDKs.** No Dropbox SDK, no MSAL, no Play services. Plain HTTPS on the existing client;
   `quick-xml` (already in the lockfile) for WebDAV.
3. **Full-access scopes,** confined in Rust to the chosen root (question 2).
4. **Google Drive and iCloud Drive are not on Android at launch** (question 1).
5. **A v2 setup code for folder accounts,** with the account ID and a path hint and no new secret. Spec and ADR
   come first.
6. **The desktop is unchanged** except for the folder setup code and the hint detection; it gets no API backends.
7. **The phone syncs only while unlocked,** with no background long-poll.
8. **Custom-scheme redirect with PKCE** until a domain allows App Links.
9. **TLS 1.3 only stays** (question 3).
10. **The own-log rollback check uses an authoritative `stat`,** never a possibly stale listing.

## 12. Not verified (spike before building)

- The current (2026) SAF behaviour of the Dropbox, OneDrive and Google Drive Android apps (no tree access is
  assumed). Only Nextcloud was checked, in its source.
- Whether a folder picked with the Google Picker under `drive.file` grants access to the files inside it, and
  whether `drive.file` grants are per OAuth client or per Cloud project.
- The CASA price and tier names for 2026.
- Graph `conflictBehavior` on the simple `PUT` (documented for upload sessions), `delta` on non-root folders for
  work accounts, and per-app revocation of consumer refresh tokens.
- `If-None-Match: *` on WebDAV `PUT`. The design does not rely on it; it uses `MOVE` with `Overwrite: F`, which
  Nextcloud documents.
- Dropbox's behaviour when a team-space user lists through the root namespace (`Dropbox-API-Path-Root`).

## 13. Left to the builder

- The trait refactor with no behaviour change on the desktop (the existing folder tests prove it).
- A `cloudsim`-based remote fake (`crates/sync/src/testutil/cloudsim.rs`) with lag, stale listings, 429s and
  partial visibility. The folder engine's chaos suite runs over both stores.
- Mock-server tests per provider: caps, redirects, conflict modes and token rotation.
- Screens and the DE/EN wording within the design system.

## Owner answers (2026-11-15)

1. **Google Drive:** skipped at launch (owner). Drive users are pointed to Dropbox, OneDrive, Nextcloud or
   hosted sync; revisit when revenue pays for Google's assessment.
2. **Permission breadth:** full access, limited to the chosen folder by our code (owner). Works with existing
   desktop folders and Dropbox team folders; the permission screen says "all files", so the in-app text explains
   that Mosaic only touches the folder you pick.
3. **TLS 1.2 Nextcloud servers:** still refused (TLS 1.3 only, as everywhere else). My call.
4. **Build order:** WebDAV/Nextcloud first (no registration), then Dropbox, then OneDrive. My call.
