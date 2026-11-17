# Android Folder Sync Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A free user joins and syncs an existing folder account on Android through Nextcloud/WebDAV, Dropbox or OneDrive, with the folder format, MACs and join rules of CRYPTO_SPEC §10.1 unchanged.

**Architecture:** A `FolderStore` trait goes under `FolderTransport` (today's `fsutil` becomes `LocalStore`, behaviour unchanged). Three remote stores (WebDAV, Dropbox API v2, Microsoft Graph) implement it in Rust over the existing blocking `reqwest`/`rustls` client (TLS 1.3 only), behind the new `mosaic-sync` feature `folder-remote`. `mosaic-mobile` links the provider in the system browser (PKCE, custom-scheme redirect), keeps the credential in `SecureStorage` and the `FolderTransport` alive while unlocked. A v2 setup code carries the folder account to the phone.

**Tech Stack:** Rust 2024 (`mosaic-core`, `mosaic-sync`, `mosaic-vault`, `mosaic-mobile`, `mosaic-mobile-ffi`, `mosaic-desktop`), `reqwest` blocking + `rustls` ring, `quick-xml` 0.42 (already in `Cargo.lock`), UniFFI, Kotlin/Compose, `packages/i18n`.

**Spec:** `docs/superpowers/specs/2026-11-15-android-folder-sync-design.md` (with the owner answers at its end). Executors read both.

## Global Constraints

- **Numbers.** ADR-0154…0160 are taken by pending branches: this plan writes **ADR-0161** (folder setup code v2) and **ADR-0162** (folder sync through provider APIs). THREAT_MODEL residual rows **R166–R170**. Store schema: **LATEST_VERSION + 1 on `main`** when Task 16 starts (v13 today; v14 if agent access merged first). Before writing any of them, check `main` (`grep -n "^## ADR-" docs/DECISIONS.md | tail -1`, `grep -o "^| R[0-9]*" docs/THREAT_MODEL.md | sort -t R -k2 -n | tail -1`, `LATEST_VERSION`) and take the next free ones; use the same numbers everywhere.
- **Build order (owner):** WebDAV/Nextcloud (Tasks 7–11), then Dropbox (12–13), then OneDrive (14–15). Google Drive and iCloud Drive are not built.
- **Scopes (owner):** Dropbox Full Dropbox with `files.metadata.read files.content.read files.content.write account_info.read`; Graph `Files.ReadWrite offline_access`. Mosaic's code reaches only `root + RelPath`.
- **TLS 1.3 only** (ADR-0046/0091), also for self-hosted WebDAV: a TLS 1.2 server gets `folderServerTlsTooOld`.
- **No provider SDK, no Play services, no `androidx.browser`.** The sign-in page opens with `Intent.ACTION_VIEW` in the system browser, as the WebAuthn handoff does (RFC 8252 external user agent). New Rust dependency edges: `quick-xml` and `subtle` (both already in the lock file) for `mosaic-sync` under `folder-remote`; each gets a one-line justification in its commit. Nothing else new.
- **Caps:** listing response ≤ `MAX_LISTING_BYTES = 4 * 1024 * 1024`; entries per directory ≤ `MAX_SCANNED` (4096); Dropbox recursive run listing ≤ `MAX_RUN_ENTRIES = 50_000` entries (more → `TransportError::Protocol`, the run fails closed); token, login-flow and metadata responses ≤ `MAX_SMALL_JSON = 64 * 1024`; downloads: announced size > `max` → `Protocol`, body read through `max + 1`; at most `MAX_PAGES = 32` continuation pages per listing.
- **Timeouts:** `CONNECT_TIMEOUT` 10 s, `REQUEST_TIMEOUT` 60 s, transfers `ATTACHMENT_TIMEOUT` 600 s (the existing constants in `crates/sync/src/http/client.rs`).
- **Secrets:** refresh tokens, app passwords, access tokens, PKCE verifiers and auth codes live in `Zeroizing`, are never `Clone`, have a redacted `Debug`, never appear in an error, a log or a path. `TransportError` carries no data from a provider. `Authorization` header values are `set_sensitive(true)`.
- **Errors never carry a path, a name or a provider message.**
- **i18n:** every UI string in `packages/i18n` (EN, DE informal "du"); error text is `error.<code>` (or a `mobile.error.<code>` override); `pnpm --filter @mosaic/i18n export:mobile` regenerates Android/iOS string files, which are committed.
- **Rust verification:** always `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo …` (written `$CARGO` below) with `-p <crate>` and a test filter. One heavy build at a time. No full-workspace run before Task 23.
- **Kotlin builds only in CI** (`.github/workflows/mobile.yml`): locally run only the i18n export and `git diff --exit-code -- apps/mobile` after it; push and let CI compile. Do not wait in a loop for CI.
- **Agent guard:** no searches outside the repo, `node_modules` or the cargo registry; writes only inside the worktree and the system temp dir; no `git add -f` of `.claude/worktrees`; more than 3 full-workspace runs per hour are denied (`FULL_RUN_OK=1` only for the final gate).
- Conventional Commits; `#![forbid(unsafe_code)]`; no `unwrap`/`expect`/`panic`/`println!` outside tests; TypeScript strict, no `any`, no `console.*`.

## Review Focus

1. **Slow provider sync.** The desktop's client uploads seconds to minutes late, out of order (a segment before the key that opens it), and Nextcloud/Graph listings lag behind writes. Expect: no `RollbackDetected`, no data loss, convergence once files arrive. Pinned by `phone_waits_for_late_desktop_files_without_halting` and `stale_own_listing_does_not_halt` (Task 11).
2. **Two devices writing at once.** Desktop and phone edit the same item, and both write segments in the same minute. Expect: last-writer rules as on two desktops, no conflict copy of a phone-owned file, both converge. Pinned by `desktop_and_phone_edit_the_same_item_and_converge` (Task 11).
3. **Token expiry mid-sync.** The access token runs out between two requests of one run (Dropbox about 4 h, Graph about 1 h), or the refresh token was revoked. Expect: one refresh and retry, then the run finishes. A revoked grant ends the run with `folderProviderSignedOut` and leaves no half-written final name. Pinned by `expired_access_token_mid_run_refreshes_once` and `revoked_refresh_token_ends_run_signed_out` (Task 13), `rotated_refresh_token_is_stored_before_use` (Task 15).
4. **Provider rate limits.** A 429 or 503 with `Retry-After` comes in the middle of a run. Expect: `syncRateLimited` with the clamped wait, no retry storm, and the next run completes without duplicate or skipped sequence numbers. Pinned by `rate_limit_mid_run_then_next_run_completes` (Task 11) and `dropbox_429_maps_to_rate_limited` (Task 12).
5. **The user picks the wrong folder.** It has no marker, belongs to another account, is empty, or is a parent of the real one. Expect: `syncFolderNoAccount`/`folderNotFound`, **nothing written** (the phone never creates or initialises a folder), and the user can browse again. Pinned by `choosing_a_wrong_folder_writes_nothing` (Task 18).

---

### Task 1: Spec change, ADRs, threat rows

**Files:**
- Modify: `docs/CRYPTO_SPEC.md` (§6.9.1 table, §6.9.7, §10.1 new subsection "Remote stores", §12.1 table)
- Modify: `docs/DECISIONS.md` (append ADR-0161, ADR-0162; one line under the M10 folder deferral at ~line 5114: "Android: resolved by ADR-0162")
- Modify: `docs/THREAT_MODEL.md` (folder section row that names R54: add "a provider API answer is a hostile folder"; §5 rows R166–R170)

- [ ] **Step 1: §6.9.7.** Add the folder form below the v1 block:
  ```
  setup_code = "mosaic-setup:2:" || base64url_unpadded(JSON)
  JSON       = {"v":2,"kind":"folder","account":"<32 lowercase hex>","provider":"dropbox|onedrive|webdav|other","server":"<canonical origin or empty>","path":"<path hint or empty>","sk":"<SK display form>"}
  ```
  The rules are: these seven members in this order, and the code is at most 1 024 bytes. `server` is non-empty exactly when `provider == "webdav"`, and then it passes `canonical_origin` with `https`. `path` is empty, or it starts with `/`, has no empty, `.` or `..` component and no trailing `/`, is at most 512 bytes and contains no `"`, `\` or control character. The parser is as strict as for v1. Replace the last bullet with: "It shows the code for hosted and folder accounts (ADR-0161)." Add: "The folder code carries no new secret: the account ID and the path are visible to the provider (§12.1)."
- [ ] **Step 2: §6.9.1 row.** `folder-credential/<account_id hex32>` | the encoded provider credential (Task 8 format) | after a provider sign-in | `reset_device`, a new sign-in, revocation.
- [ ] **Step 3: §10.1 "Remote stores (ADR-0162)".** Six bullets:
  - The phone reaches the same folder through a provider API. The format is unchanged.
  - Write-once names use the provider's conflict mode (Dropbox `add` with `strict_conflict`, Graph `conflictBehavior=fail`, WebDAV `MOVE` with `Overwrite: F`), never a check-then-write.
  - `head.mck` and `info.mdev` are replaced in one provider call.
  - The own-log rollback check counts a name as missing only when an authoritative `stat` says it is missing.
  - A remote `remove` never deletes a directory, and it is conditional on the listed version.
  - Listing entries outside the root, or with names that fail `folder_names::parse`, are dropped.
- [ ] **Step 4: §12.1 rows.**
  - The provider sees which API app reads and writes the files (the Dropbox app key, the Entra client ID).
  - It sees the phone's IP address, and the time, count and size of every request of a sync run.
  - It sees the OAuth grant and its scopes on the user's account page.
  - Nextcloud only: the admin sees the app password's name `Mosaic (Android)` and its last use.
- [ ] **Step 5: ADR-0161 "Folder setup code v2".**
  - Context: spec §7. The decision is Step 1.
  - Alternatives:
    - an optional member in v1 (rejected: it would break the strict v1 parser on old phones);
    - carrying an OAuth token (rejected: a new secret in the QR code).
  - Consequences: the desktop shows the code for folder accounts; the hint is best effort.
- [ ] **Step 6: ADR-0162 "Android folder sync through provider APIs".** Record each decision in one line:
  - Rust HTTP stores, not SAF (SAF store is phase 2 and not in this plan).
  - Full-access scopes, confined in code (owner).
  - No SDKs; the system browser through `ACTION_VIEW`, not Custom Tabs (no `androidx.browser` dependency).
  - Redirect `io.github.stephanbaum.mosaic.oauth:/cb` with PKCE S256 (a fresh 32-byte verifier per attempt) and a 32-byte `state` compared in constant time with `subtle`. PKCE is RFC 7636 over SHA-256 and `random(32)` from §2, not a new construction.
  - The pending link lives 5 min and follows the join slot's lifetime.
  - The client IDs are compile-time `option_env!` values; an empty one means `folderProviderUnsupported`.
  - Graph lists per directory at launch (no `delta`).
  - Revocation: Dropbox `token/revoke`, Nextcloud `DELETE apppassword`, Microsoft none.
  - TLS 1.3 only (owner).
  - Google Drive is skipped (owner).
  - `quick-xml` for `PROPFIND`.
- [ ] **Step 7: Threat rows.**
  - R166: a thief who breaks the Keystore gets cloud access beyond the Mosaic folder (full-access scope). Accepted, owner answer 2.
  - R167: another app can claim the custom redirect scheme and receive the code. It is useless without the PKCE verifier; a forged callback fails the `state` check.
  - R168: Microsoft consumer refresh tokens cannot be revoked by the app. The user removes the app on the account page; MOBILE.md says how.
  - R169: the provider sees API metadata (§12.1).
  - R170: a hostile provider answer is the R54 hostile folder, plus bounded JSON/XML parsing.
- [ ] **Step 7b: Ask the crypto-reviewer agent** (read-only) to review Steps 1, 2, 5 and 6 against CRYPTO_SPEC §2/§6.9. Fix its findings before Task 2.
- [ ] **Step 8: Commit** `docs(crypto): folder setup code v2 and provider-API folder stores (ADR-0161, ADR-0162)`

### Task 2: Setup code v2 in core and sync

**Files:**
- Modify: `crates/core/src/setup_code.rs`
- Modify: `crates/sync/src/setup_code.rs`
- Test: in-file `#[cfg(test)]` modules of both

**Interfaces:**
- Produces (core):
  - `pub const SETUP_CODE_PREFIX_V2: &str = "mosaic-setup:2:"`;
  - `pub enum FolderProviderHint { Dropbox, OneDrive, WebDav, Other }` with `as_wire(self) -> &'static str` and `from_wire(&str) -> Option<Self>`;
  - `pub struct FolderSetupCode { pub account: [u8; 16], pub provider: FolderProviderHint, pub server: String, pub path: String, pub secret_key: SecretKey }` (redacting `Debug`, not `Clone`);
  - `pub enum ParsedSetupCode { Hosted(SetupCode), Folder(FolderSetupCode) }`;
  - `pub fn build_folder(code: &FolderSetupCode) -> Result<Zeroizing<String>>`;
  - `pub fn parse_any(bytes: &[u8], canonical: impl Fn(&str, &str) -> Option<(String, String)>, canonical_server: impl Fn(&str) -> Option<String>) -> Result<ParsedSetupCode>`.
- Produces (sync): `pub fn parse_any(bytes: &[u8]) -> mosaic_core::Result<ParsedSetupCode>` (https-only canonical origin for `webdav`), plus re-exports.

- [ ] **Step 1: Failing tests (core).**
  - `folder_code_round_trips`: build → parse_any gives `Folder` with equal fields.
  - `folder_code_golden`: the account `00..0f`, `Dropbox`, server `""`, path `/Mosaic` and the golden SK build exactly `"mosaic-setup:2:" + base64url({"v":2,"kind":"folder","account":"000102030405060708090a0b0c0d0e0f","provider":"dropbox","server":"","path":"/Mosaic","sk":"<golden display>"})`.
  - `folder_code_rejects_every_deviation`: each gives `Err(Error::Malformed)`:
    - members reordered, a member missing, an extra member;
    - `"v":1` with the v2 prefix;
    - uppercase hex, 31 hex characters;
    - `provider:"gdrive"`;
    - `server` non-empty for `dropbox`, `server` empty for `webdav`;
    - path `Mosaic` (no leading `/`), `/a//b`, `/a/../b`, `/a/`;
    - a 513-byte path, a code over 1 024 bytes.
  - `v1_code_still_parses_through_parse_any`: gives `Hosted`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-core --lib setup_code`. Expect FAIL (not defined).
- [ ] **Step 3: Implement.** Use the same canonical-member parsing style as v1 (prefix, then a fixed member sequence, `plain`). Build in `Zeroizing`.
- [ ] **Step 4: Failing test (sync).** `folder_setup_code_uses_https_origin`: `webdav` with `https://Cloud.example.com` → `Malformed`; with `http://cloud.example.com` → `Malformed`; with `https://cloud.example.com` → `Ok`.
- [ ] **Step 5: Run** `$CARGO test -p mosaic-core --lib setup_code` and `$CARGO test -p mosaic-sync --lib setup_code`. Expect PASS.
- [ ] **Step 6: Commit** `feat(core,sync): folder setup code v2 (ADR-0161)`

### Task 3: Folder hint detection

**Files:**
- Create: `crates/sync/src/folder/hint.rs` (pure: no file access)
- Modify: `crates/sync/src/folder/mod.rs` (`pub mod hint;`)

**Interfaces:**
- Produces:
  - `pub struct HintSources { pub dropbox_info_json: Option<Vec<u8>>, pub onedrive_roots: Vec<PathBuf>, pub nextcloud_cfg: Option<Vec<u8>> }`;
  - `pub struct FolderHint { pub provider: FolderProviderHint, pub server: String, pub path: String }`;
  - `pub fn detect_hint(folder: &Path, sources: &HintSources) -> FolderHint`.
- The result always satisfies Task 2's path and server rules. Anything unknown gives `Other`, `""`, `""`.

- [ ] **Step 1: Failing tests.**
  - `dropbox_personal_path_is_relative_with_slashes`: info.json `{"personal":{"path":"C:\\Users\\a\\Dropbox"}}`, folder `C:\Users\a\Dropbox\Apps\Mosaic` → `Dropbox`, `/Apps/Mosaic`.
  - `dropbox_business_path_is_used_when_the_folder_is_under_it`.
  - `onedrive_root_from_env_list`: roots `[C:\Users\a\OneDrive - Firma]`, folder below → `OneDrive`, relative path.
  - `nextcloud_cfg_maps_local_to_target_path`: cfg lines `0\url=https://cloud.example.com`, `0\Folders\1\localPath=C:/Users/a/Nextcloud/`, `0\Folders\1\targetPath=/Sync` → `WebDav`, server `https://cloud.example.com`, path `/Sync/Mosaic`.
  - `unknown_or_hostile_sources_give_other`: these all give `Other`:
    - malformed JSON;
    - a folder outside every root;
    - a path with `"` in it;
    - a cfg url `http://…`;
    - a cfg over 1 MiB (cap `MAX_HINT_SOURCE = 1024 * 1024`).
  - Path comparison is case-insensitive on Windows only (`cfg!(windows)`).
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --lib folder::hint`. Expect FAIL.
- [ ] **Step 3: Implement.** `serde_json` reads info.json; a line scanner reads the INI (`[Accounts]` keys `<n>\url`, `<n>\Folders\<m>\localPath|targetPath`). The provider is whichever root is the longest prefix of the folder.
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): provider path hint for the folder setup code`

### Task 4: Desktop shows the setup code for folder accounts

**Files:**
- Modify: `apps/desktop/src-tauri/src/commands/setup_code.rs`
- Create: `apps/desktop/src-tauri/src/sync/hint_sources.rs` (reads the three sources)
- Modify: `apps/desktop/src-tauri/src/setup_code_tests.rs`
- Modify: `apps/desktop/src/screens/sync/SetupPhone.tsx`, `SetupPhone.test.tsx`
- Modify: `packages/i18n/src/catalog/{en,de}/sync.ts`, `packages/i18n/src/catalog/{en,de}/errors.ts` (`error.setupCodeNeedsServer` → "…an account that syncs" wording)

**Interfaces:**
- Consumes: `build_folder`, `detect_hint`, `HintSources` (Tasks 2–3).
- Produces: `pub(crate) fn gather_hint_sources() -> HintSources`. It reads:
  - Windows: `%APPDATA%\Dropbox\info.json`, then `%LOCALAPPDATA%\Dropbox\info.json`, then env `OneDrive`, `OneDriveConsumer`, `OneDriveCommercial`, then `%APPDATA%\Nextcloud\nextcloud.cfg`;
  - macOS/Linux: `~/.dropbox/info.json`, `~/Library/CloudStorage/OneDrive-*`, `~/.config/Nextcloud/nextcloud.cfg`, `~/Library/Preferences/Nextcloud/nextcloud.cfg`.

  Each source is read with a size cap of `MAX_HINT_SOURCE`, and any read error counts as absent.

- [ ] **Step 1: Failing tests.**
  - Rename `setup_code_refused_for_folder_account` to `setup_code_for_folder_account_carries_account_and_hint`. It decodes the modules and checks that the payload starts with `mosaic-setup:2:`, that `parse_any` gives `Folder` with this account, and that the provider is `Other` in the test environment.
  - `setup_code_folder_still_requires_reauth`: a wrong password → `wrongCredentials`.
  - SetupPhone vitest `shows the code button for a folder account`: no `settings.sync.phone.folder` text, and the button is there.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-desktop --lib setup_code` and `pnpm --filter @mosaic/desktop test -- SetupPhone`. Expect FAIL.
- [ ] **Step 3: Implement.** For `SyncKind::Folder`, `detect_hint(folder_path, &gather_hint_sources())` → `FolderSetupCode` → `build_folder`. The reauth, 60 s lifetime and QR rendering stay as they are. The UI drops the folder branch and adds the hint text `settings.sync.phone.folderHint`:
  - EN: "Your phone signs in to your cloud storage and opens this same folder."
  - DE: "Dein Handy meldet sich bei deinem Cloud-Speicher an und öffnet denselben Ordner."
- [ ] **Step 4: Run again,** plus `pnpm --filter @mosaic/i18n test`. Expect PASS.
- [ ] **Step 5: Commit** `feat(desktop): setup code for folder accounts with a provider path hint`

### Task 5: `FolderStore` trait, `RelPath`, `LocalStore`

**Files:**
- Create: `crates/sync/src/folder/store.rs`
- Modify: `crates/sync/src/folder/fsutil.rs` (its functions become `LocalStore`'s private helpers; its tests move into `store.rs` unchanged in substance)

**Interfaces:**
- Produces (all `pub`, re-exported from `folder`):
  ```rust
  pub enum EntryKind { File, Dir, Link }
  #[non_exhaustive]
  pub struct Entry { pub name: OsString, pub kind: EntryKind, pub len: u64,
                     pub modified: Option<SystemTime>, pub version: Option<Box<[u8]>> }
  impl Entry { pub fn is_dir(&self) -> bool; pub fn modified_ms(&self) -> u64; }
  pub struct RelPath(/* components */);
  impl RelPath {
      pub fn root() -> Self;
      pub fn child(&self, name: &str) -> Option<Self>;   // validated
      pub fn parent(&self) -> Option<Self>;
      pub fn components(&self) -> impl Iterator<Item = &str>;
      pub(crate) fn known(&self, name: &str) -> Self;   // only constants and folder_names builders
  }
  pub trait FolderStore: Send {
      fn begin_run(&self) -> Result<(), TransportError> { Ok(()) }
      fn list(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError>;          // no links
      fn list_with_links(&self, dir: &RelPath) -> Result<Vec<Entry>, TransportError> { self.list(dir) }
      fn list_capped(&self, dir: &RelPath, limit: usize, keep: &dyn Fn(&Entry) -> bool) -> Result<Vec<Entry>, TransportError>;
      fn read_capped(&self, path: &RelPath, max: u64) -> Result<Option<Vec<u8>>, TransportError>;
      fn write_atomic(&self, dir: &RelPath, name: &str, bytes: &[u8], replace: bool) -> Result<(), TransportError>;
      fn remove(&self, path: &RelPath) -> Result<(), TransportError>;
      fn remove_empty_dir(&self, path: &RelPath);
      fn remove_squatter(&self, path: &RelPath);
      fn stat(&self, path: &RelPath) -> Result<Option<Entry>, TransportError>;   // authoritative
      fn exists(&self, path: &RelPath) -> bool { matches!(self.stat(path), Ok(Some(_))) }
      fn is_real_dir(&self, path: &RelPath) -> bool { matches!(self.stat(path), Ok(Some(e)) if e.is_dir()) }
      fn create_dir_all(&self, dir: &RelPath) -> Result<(), TransportError>;
  }
  pub struct LocalStore { root: PathBuf }
  impl LocalStore { pub fn new(root: &Path) -> Self; }
  ```
- The rules for a valid component: non-empty, at most 255 bytes, not `.` or `..`, and no `/`, `\`, NUL, `:` or control character. `known` is not public; a unit test proves that every `folder_names` builder output and every layout constant is valid.
- `LocalStore` keeps the old behaviour exactly:
  - `exists` keeps today's `try_exists`, which follows links;
  - `is_real_dir` keeps `symlink_metadata`;
  - `list_with_links` returns links as `EntryKind::Link` (needed by `clear_above_reserved`);
  - `version` is always `None`.

- [ ] **Step 1: Failing tests** in `store.rs`:
  - The four moved `fsutil` tests, now through `LocalStore`, with the same assertions: `list_capped_stops_at_the_limit_and_filters`, `list_capped_scans_a_bounded_number_of_items`, `is_real_dir_is_true_for_directories_only`, `read_capped_reads_only_small_regular_files`, plus the unix symlink test.
  - `child_refuses_unsafe_components`: `""`, `"."`, `".."`, `"a/b"`, `"a\\b"`, `"a\0"`, `"c:"`, a 256-byte name → `None`; `"head.mck"` → `Some`.
  - `every_builder_output_is_a_valid_component`.
  - `list_with_links_reports_links` (unix).
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --lib folder::store`. Expect FAIL.
- [ ] **Step 3: Implement** by moving the code, not rewriting it.
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `refactor(sync): FolderStore trait with LocalStore (no behaviour change)`

### Task 6: `FolderTransport` and the folder functions over the store

**Files:**
- Modify: `crates/sync/src/folder/{mod,layout,account,blobs,devices,init,maintain,reader,rekey,writer}.rs`
- Modify: `crates/sync/src/transport.rs` (variant `TlsVersionUnsupported`, used from Task 7)

**Interfaces:**
- Consumes: Task 5.
- Produces:
  - `Layout` methods return `RelPath` (built with `known`).
  - `pub struct FolderParts<C> { pub account_id, pub install_id, pub crypto: C, pub clock: Arc<dyn Clock>, pub device: DeviceInfo }`.
  - `FolderTransport::open_with_store(store: Box<dyn FolderStore>, parts: FolderParts<C>) -> Result<Self, TransportError>`. `open(FolderConfig)` stays unchanged and calls it with `LocalStore::new(&root)`.
  - `pub fn begin_run(&mut self) -> Result<(), TransportError>`, which forwards to the store.
  - `pub fn init_in(&dyn FolderStore)`, `inspect_in`, `list_accounts_in`, `has_account_in(&dyn FolderStore, &AccountId)`, `join_from_folder_in(&dyn FolderStore, …)` and `current_account_file_in`. The existing `&Path` functions become one-line wrappers with unchanged signatures, so desktop, CLI and tests do not change.
  - `inspect_in` replaces `root.is_dir()` with `store.is_real_dir(&RelPath::root())`. For `LocalStore` this is the same answer for an existing directory. The `Missing` case for a symlinked root is kept through a `LocalStore`-specific `root.is_dir()` check in the `&Path` wrapper.
  - `SegmentFile` and `ScanEntry` gain `version: Option<Box<[u8]>>`. The cache is valid when both versions are `Some` and equal, or else when `len` and `modified` are equal (as today).
  - `own_log_reaches`: when no listed file at or above `written` counts as present, it calls `store.stat(log_dir/segment_name(written))`. A `Some` that probes Good, or cannot be read now, counts as present. `None` counts as missing.
  - `clear_above_reserved` uses `list_with_links`: a `Link` or `Dir` gets `remove_squatter`, as today.

- [ ] **Step 1: Failing test** `crates/sync/tests/it/folder_transport.rs::own_log_rollback_check_asks_stat_before_halting`. It uses a test store (`testutil::StatOverride`, new in `testutil/mod.rs`, which wraps `LocalStore` and hides one name from `list` but not from `stat`). After a write, hiding the newest own segment from the listing gives `own_logs_rolled_back == Ok(false)`. Deleting it on disk gives `Ok(true)`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util --test it own_log_rollback_check_asks_stat`. Expect FAIL.
- [ ] **Step 3: Implement** the interfaces above. Replace every `fsutil::` and `std::fs::` call in `folder/` with `self.store.` / `store.`.
- [ ] **Step 4: Prove no behaviour change.** Run:
  - `$CARGO test -p mosaic-sync --features test-util --lib folder`;
  - `$CARGO test -p mosaic-sync --features test-util --test it -- folder_transport join_folder sim_folder malicious_folder rotation_folder chaos owned_session`;
  - `$CARGO test -p mosaic-cli --features test-util --locked -- --test-threads=2 transfer`.

  All must PASS with no test file edited except the new test.
- [ ] **Step 5: Commit** `refactor(sync): folder transport over FolderStore; authoritative stat for the own-log rollback check`

### Task 7: Remote HTTP client, fake HTTP server, date parsing

**Files:**
- Modify: `crates/sync/Cargo.toml`:
  - feature `folder-remote = ["http", "dep:quick-xml", "dep:subtle"]`;
  - `quick-xml = { version = "=0.42.0", default-features = false, optional = true }` (WebDAV `PROPFIND`, already in the lock file through `plist`);
  - `subtle = { workspace = true, optional = true }` (constant-time `state` compare). Add `subtle` to `[workspace.dependencies]` if it is not there yet, with the version from `Cargo.lock`.
- Modify: `crates/sync/src/http/{client,tls,mod}.rs` (`pub(crate)` for `tls::config`, `read_capped`, `parse_retry_after`, `map_reqwest`)
- Create: `crates/sync/src/folder/remote/{mod,http,time}.rs`
- Create: `crates/sync/src/testutil/fakehttp.rs` (`#[cfg(any(test, feature = "test-util"))]`)

**Interfaces:**
- Produces:
  - `pub struct RemoteHttp`, with:
    - `fn new(allow_loopback: bool) -> Result<Self, TransportError>`: redirects off, the TLS 1.3 config, `https_only` unless loopback;
    - `fn send(&self, req: RemoteRequest<'_>) -> Result<RemoteReply, TransportError>`;
    - `fn get_following_one_redirect(&self, url: &str, auth: Option<&HeaderValue>, timeout: Duration, cap: u64) -> Result<RemoteReply, TransportError>`.
  - `pub struct RemoteRequest<'a> { pub method: Method, pub url: &'a str, pub headers: &'a [(HeaderName, HeaderValue)], pub body: RemoteBody<'a>, pub timeout: Duration, pub cap: u64 }`.
  - `pub enum RemoteBody<'a> { None, Bytes(&'a [u8], &'static str), Form(Zeroizing<Vec<u8>>) }`.
  - `pub struct RemoteReply { pub status: u16, pub retry_after_s: Option<u32>, headers: Vec<(String, String)>, pub body: Vec<u8> }` with `header(&self, name) -> Option<&str>`. At most 64 headers are kept, each at most 8 KiB.
  - `pub(crate) fn check_url(url: &str, allow_loopback: bool) -> Result<(), TransportError>`: `https://` only (`http://127.0.0.1:` and `http://[::1]:` when loopback is allowed), no userinfo, no fragment, at most 8 KiB.
  - `pub(crate) fn status_error(reply: &RemoteReply) -> TransportError`: 401 `Unauthorized`, 403 `Forbidden`, 404 `NotFound`, 408 `Timeout`, 429 `RateLimited`, 503 with `Retry-After` `RateLimited`, 503 without it `Unreachable`, 507 `QuotaExceeded`, 502/504 `Unreachable`, other 5xx `Io`, the rest `Protocol`.
  - TLS: when the source chain holds `rustls::Error::PeerIncompatible(_)` or `AlertReceived(ProtocolVersion)`, the result is `TransportError::TlsVersionUnsupported`.
  - `time::parse_rfc3339(&str) -> Option<SystemTime>` and `time::parse_http_date(&str) -> Option<SystemTime>` (IMF-fixdate only).
  - testutil: `pub struct FakeHttp`, with:
    - `fn start(handler: impl Fn(&FakeRequest) -> FakeResponse + Send + Sync + 'static) -> Self`;
    - `fn origin(&self) -> &str`;
    - `fn requests(&self) -> Vec<FakeRequest>`.

    `FakeRequest { method, path_and_query, headers: Vec<(String,String)>, body: Vec<u8> }` and `FakeResponse { status, headers, body }`. The server reads `Content-Length` bodies up to 64 MiB, uses one thread per connection and `Connection: close`, follows the `RawServer` pattern in `tests/it/http_transport.rs`, and stops on drop.

- [ ] **Step 1: Failing tests** (`folder/remote/http.rs` and `time.rs` test modules, over `FakeHttp`):
  - `check_url_refuses_http_userinfo_and_fragments`.
  - `success_body_over_cap_is_protocol`: `Content-Length` over the cap and a chunked body over the cap.
  - `redirect_is_followed_once_without_authorization`: the first reply is 302 to the second fake's `/dl`. The second fake records that the `Authorization` header is absent. A 302 to a 302 gives `Protocol`, and a 302 to `ftp://` gives `Protocol`.
  - `status_mapping_table` (the table above), and `retry_after_is_clamped` (`0` → 1, `999999` → 86 400).
  - `tls_version_error_is_classified` (built `rustls::Error` values through a pure `classify_tls(&dyn Error)`).
  - `parse_dates`: `2026-11-15T10:20:30Z`, `Sun, 15 Nov 2026 10:20:30 GMT`, garbage → `None`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --lib folder::remote`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): remote HTTP layer for provider folder stores` (body: the two dependency justification lines)

### Task 8: Credentials, PKCE, callback parsing, folder location

**Files:**
- Create: `crates/sync/src/folder/remote/{credential,oauth,location}.rs`

**Interfaces:**
- Produces:
  - `pub enum Provider { Dropbox, OneDrive, WebDav }`.
  - `pub enum ProviderCredential { WebDav { login: String, app_password: Zeroizing<String> }, Dropbox { refresh_token: Zeroizing<String> }, Graph { refresh_token: Zeroizing<String> } }`. No `Clone`. `Debug` prints `ProviderCredential::<Kind>([REDACTED])`. It has:
    - `fn encode(&self) -> Zeroizing<Vec<u8>>`: byte 0 = version `0x01`, byte 1 = kind (`1` WebDAV, `2` Dropbox, `3` Graph), then `u16`-BE length-prefixed UTF-8 fields (WebDAV: login, password; others: token), each at most 4 096 bytes, nothing trailing;
    - `fn decode(&[u8]) -> Result<Self, TransportError>` (`Protocol` on any deviation);
    - `fn provider(&self) -> Provider`.
  - `pub trait CredentialSink: Send + Sync { fn rotated(&self, credential: &ProviderCredential) -> Result<(), TransportError>; }`.
  - `pub struct Pkce` (32 random bytes, verifier = base64url without padding, 43 characters, `Zeroizing`) with `fn new() -> Result<Self, TransportError>`, `fn challenge(&self) -> String` (base64url(SHA-256(verifier))) and `fn verifier(&self) -> &str`.
  - `pub struct AuthState([u8; 32])` with `fn new()`, `fn as_param(&self) -> String` and `fn matches(&self, other: &str) -> bool` (decode, then `subtle::ConstantTimeEq`).
  - `pub struct Callback { pub code: Zeroizing<String>, pub state: String }` and `pub enum CallbackError { Invalid, Denied }`.
  - `pub fn parse_callback(url: &str, redirect: &str) -> Result<Callback, CallbackError>`. The URL must start with exactly `redirect + "?"` and be at most 4 096 bytes. Keys are percent-decoded, and a duplicate key is `Invalid`. `error=` with any value is `Denied`. A missing `code` or `state` is `Invalid`. Unknown keys are ignored.
  - `pub(crate) fn form(pairs: &[(&str, &str)]) -> Zeroizing<Vec<u8>>`, with `application/x-www-form-urlencoded` encoding that keeps only the unreserved characters.
  - `pub(crate) fn path_segment(s: &str) -> String`, which percent-encodes everything except unreserved characters.
  - `pub struct FolderLocation { pub provider: Provider, pub root: String, pub server: Option<String>, pub drive_id: Option<String>, pub item_id: Option<String>, pub path_root: Option<String> }`. It has serde JSON `encode() -> String` (at most 4 096 bytes) and `decode(&str) -> Result<Self, TransportError>`, with `deny_unknown_fields`. It holds no secret.
  - `pub const REDIRECT_URI: &str = "io.github.stephanbaum.mosaic.oauth:/cb";`
  - `pub struct Endpoints { pub dropbox_auth, pub dropbox_api, pub dropbox_content, pub ms_login, pub graph: String }` with `Endpoints::production()` and `Endpoints::loopback(origin: &str)` (`#[cfg(any(test, feature = "test-util"))]`).

- [ ] **Step 1: Failing tests.**
  - `credential_round_trip_and_strict_decode`: trailing byte, version 2, kind 4 and an over-long field → `Err(Protocol)`.
  - `credential_debug_is_redacted`: `format!("{c:?}")` does not contain the token.
  - `pkce_rfc7636_appendix_b`: verifier `dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk` → challenge `E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM`. This uses a `Pkce::from_verifier_for_test` behind `cfg(test)`.
  - `state_matches_only_itself`.
  - `parse_callback_table`:
    - an exact redirect with code and state → `Ok`;
    - another scheme, `…:/cb2?`, a duplicate `state`, a missing `code`, or a length over 4 096 → `Invalid`;
    - `error=access_denied&state=x` → `Denied`.
  - `location_json_round_trip_and_unknown_field_refused`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features folder-remote --lib folder::remote`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): provider credentials, PKCE and callback parsing`

### Task 9: Fake cloud model and WebDAV store

**Files:**
- Create: `crates/sync/src/folder/remote/{webdav,davxml}.rs`
- Create: `crates/sync/src/testutil/fake_cloud/{mod,dav}.rs` (`#[cfg(all(feature = "test-util", feature = "folder-remote"))]`)

**Interfaces:**
- Produces:
  - `pub struct WebDavStore` with `fn new(http: RemoteHttp, base: &str, root: &str, credential: ProviderCredential /* WebDav */) -> Result<Self, TransportError>`, which implements `FolderStore`. `base` is the collection URL `https://host/remote.php/dav/files/<login>` (or a typed WebDAV URL), canonical, with no trailing `/`.
  - Request URL: `base + "/" + root components + "/" + rel components`, each encoded with `path_segment`.
  - `list`: `PROPFIND Depth: 1` with body `<d:propfind xmlns:d="DAV:"><d:prop><d:getetag/><d:getcontentlength/><d:getlastmodified/><d:resourcetype/></d:prop></d:propfind>`, `MAX_LISTING_BYTES`. A 404 means an empty list.
  - `stat`: `PROPFIND Depth: 0`.
  - `read_capped`: `GET`; a `Content-Length` over `max` is `Protocol`.
  - `write_atomic`: `MKCOL` of the parents (405 or 409-then-exists is fine), `PUT` to `.<16 hex>.mtmp`, then `MOVE` with `Destination: <absolute URL>` and `Overwrite: F` (write-once; 412 → `Io`) or `T`. When the `MOVE` fails, a best-effort `DELETE` of the temp file follows.
  - `remove`: `DELETE` with `If-Match: <etag>` from the last listing or `stat`. It is refused locally when the target is a collection. 404 is `Ok`, 412 is `Io`.
  - `remove_squatter` and `remove_empty_dir`: `DELETE` of a collection, the latter only after a `Depth: 1` listing shows it is empty.
  - Basic auth from the credential, as a sensitive header value built in `Zeroizing`.
  - `begin_run` clears a per-run listing cache; this run's own writes are merged into later `list` answers of the same run.
  - `davxml::parse_multistatus(body: &[u8], collection_href: &str) -> Result<Vec<Entry>, TransportError>`. It uses `quick-xml` with namespaces and drops the collection itself. Each `href` is percent-decoded and must start with the decoded collection href; the remainder must be one valid component (`RelPath::child`), and everything else is dropped. A depth over 16 or over 4 096 entries → `Protocol`.
  - testutil `FakeDav`:
    - `fn serve(dir: PathBuf, login: &str, password: &str) -> Self`, which serves the directory as a WebDAV tree: `PROPFIND`, `GET`, `PUT`, `MKCOL`, `MOVE` with `Overwrite`, `DELETE` with `If-Match`, ETag = hex of SHA-256(len‖mtime);
    - `Faults { stale_listing: bool, fail_nth: Option<(usize, u16, Option<u32>)>, outside_href: bool }`, set by `fn set_faults(&self, Faults)`;
    - `fn origin()` and `fn writes(&self) -> usize` (counts `PUT`, `MKCOL`, `MOVE`, `DELETE`).

- [ ] **Step 1: Failing tests** (`crates/sync/tests/it/remote_dav.rs`, add `mod remote_dav;` to `main.rs` under `cfg(feature = "folder-remote")`):
  - `dav_store_lists_reads_and_writes_like_local`. It runs the store contract (`fn store_contract(store: &dyn FolderStore)` in `tests/it/common/mod.rs`, also run over `LocalStore` in `local_store_passes_the_store_contract`):
    - list sorted;
    - a missing directory → empty;
    - `read_capped` with `max` → `Protocol`;
    - write-once twice → second `Err(Io)`;
    - replace → new bytes;
    - `remove` of a missing file → `Ok`;
    - `stat` after write → `Some`.
  - `listing_entry_outside_the_root_is_dropped`: `outside_href` makes the fake answer one `href` of `/remote.php/dav/files/alice/Other/x.mseg` and one with `%2F` in the name. Neither is listed, and the rest is.
  - `remove_never_deletes_a_collection`: a directory planted on a file name; `remove` → `Ok(())` and the directory and its content are still there.
  - `write_once_conflict_is_io_and_leaves_no_temp`.
  - `propfind_over_cap_is_protocol`.
  - `password_never_in_debug`: `format!("{store:?}")` does not contain the password.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_dav`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again,** plus `--lib folder::remote::davxml`. Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): WebDAV folder store`

### Task 10: Nextcloud Login Flow v2

**Files:**
- Create: `crates/sync/src/folder/remote/nextcloud.rs`
- Modify: `crates/sync/src/testutil/fake_cloud/dav.rs` (login routes)

**Interfaces:**
- Produces:
  - `pub struct LoginFlowStart { pub login_url: String, pub poll: LoginPoll }` and `pub struct LoginPoll { endpoint: String, token: Zeroizing<String>, origin: String }` (redacted `Debug`).
  - `pub fn login_flow_begin(http: &RemoteHttp, origin: &str, user_agent: &str) -> Result<LoginFlowStart, TransportError>`: `POST {origin}/index.php/login/v2` with `User-Agent: Mosaic (Android)`, `MAX_SMALL_JSON`.
  - `pub fn login_flow_poll(http: &RemoteHttp, poll: &LoginPoll) -> Result<Option<(ProviderCredential, String /* dav base */)>, TransportError>`: `POST endpoint` with form `token=…`; 404 → `Ok(None)`; 200 → `{server, loginName, appPassword}`.
  - `pub fn revoke_app_password(http: &RemoteHttp, origin: &str, credential: &ProviderCredential) -> Result<(), TransportError>`: `DELETE {origin}/ocs/v2.php/core/apppassword` with `OCS-APIRequest: true`.
  - Origin rules: `login_url` and `poll.endpoint` must have exactly the canonical `origin`; the returned `server` must canonicalise to `origin`; `loginName` must be a valid component. Anything else → `Protocol`. The DAV base is `{origin}/remote.php/dav/files/{path_segment(loginName)}`.

- [ ] **Step 1: Failing tests** (`tests/it/remote_dav.rs`):
  - `login_flow_happy_path`: begin, poll → `None`, mark the fake as done, poll → `Some` with `WebDav` login `alice`.
  - `login_flow_refuses_foreign_poll_endpoint`: endpoint `http://127.0.0.2:…` → `Protocol`.
  - `login_flow_refuses_other_server_in_result`.
  - `revoke_sends_ocs_header_and_basic_auth`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_dav::login`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): Nextcloud Login Flow v2 and app-password revocation`

### Task 11: Folder engine over WebDAV with a lagging cloud

**Files:**
- Create: `crates/sync/tests/it/remote_engine.rs`
- Modify: `crates/sync/tests/it/common/mod.rs` (`open_remote_transport(access, store: Box<dyn FolderStore>, clock, name)`)

**Interfaces:**
- Consumes: `FolderTransport::open_with_store`, `begin_run`, `WebDavStore`, `FakeDav`, `CloudSim`.
- The setup: replica 0 is the desktop (`LocalStore`). Replica 1 is served by `FakeDav`, and the phone opens it through `WebDavStore`. `CloudSim` moves files between the replicas.

- [ ] **Step 1: Write the tests.**
  - `phone_joins_and_pulls_desktop_items`: `join_from_folder_in` over the `WebDavStore` → the phone sees the desktop's item after one run.
  - `phone_waits_for_late_desktop_files_without_halting` (Review Focus 1): `SimConfig::FLAKY` plus `withhold` of the desktop's key segment for 3 rounds. No run reports `RollbackDetected` or halts, and after `settle()` both devices hold all items.
  - `stale_own_listing_does_not_halt` (Review Focus 1): `Faults { stale_listing: true }` for the phone's own log directory after a write → the next run has no `RollbackDetected`.
  - `desktop_and_phone_edit_the_same_item_and_converge` (Review Focus 2): both set the password in the same round. After `settle()`, both have the same value, and no `conflicted`/` (1)` file exists under `d/<phone>/` or `log/<phone>/`.
  - `rate_limit_mid_run_then_next_run_completes` (Review Focus 4): `fail_nth: Some((5, 429, Some(30)))` → `SyncError::Transport(RateLimited { retry_after_s: 30 })`. Next, with the faults cleared, the run is `Ok`, and the phone's own log sequence numbers are contiguous or have only reserved gaps (no duplicate seq file).
  - `malicious_folder_suite_over_dav`: run the planted-file scenarios of `malicious_folder.rs` (the junk segment, a symlink stand-in as a collection, a conflict copy of `head.mck`) against the phone over WebDAV → the same outcomes as on disk. Reuse the scenario functions by giving them a store parameter.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_engine`. Fix the store, not the tests, until they PASS.
- [ ] **Step 3: Ask the sync-chaos agent** to add one adversarial schedule over `FakeDav`: a phone and two desktops, a rollback through version history. Keep what it writes if it passes; a finding gets a fix in this task.
- [ ] **Step 4: Commit** `test(sync): folder engine over WebDAV with lag, stale listings and rate limits`

### Task 12: Dropbox store

**Files:**
- Create: `crates/sync/src/folder/remote/dropbox.rs` (store part)
- Create: `crates/sync/src/testutil/fake_cloud/dropbox.rs`
- Test: `crates/sync/tests/it/remote_dropbox.rs`

**Interfaces:**
- Produces:
  - `pub struct DropboxStore` with `fn new(http: RemoteHttp, endpoints: &Endpoints, location: &FolderLocation, auth: DropboxAuth) -> Result<Self, TransportError>`, which implements `FolderStore`. This task defines `pub struct DropboxAuth` with `fn bearer(&self) -> Result<HeaderValue, TransportError>` and `fn fixed_for_test(token: &str) -> Self` (`#[cfg(any(test, feature = "test-util"))]`). Task 13 adds the refresh. The Graph tasks split `GraphAuth` the same way.
  - `begin_run`: `POST {api}/2/files/list_folder` `{"path":root,"recursive":true,"include_deleted":false,"limit":2000}`, then `/list_folder/continue` while `has_more`, up to `MAX_PAGES` and `MAX_RUN_ENTRIES`. This builds a map from `RelPath` to `Entry`. `list` answers from it (plus this run's writes), `version` = `rev`, `modified` = `server_modified`. Outside a run, `list` does a non-recursive `list_folder`.
  - Confinement: an entry must have `path_lower` starting with `lowercase(root) + "/"`; the remainder splits into valid components and its last component equals `name` case-insensitively. Otherwise it is dropped. `deleted` entries are skipped.
  - `stat`: `/2/files/get_metadata`; 409 `path/not_found` → `None`.
  - `read_capped`: `POST {content}/2/files/download` with `Dropbox-API-Arg: {"path":…}`. The `size` in the `Dropbox-API-Result` header is checked against `max`.
  - `write_atomic`: `POST {content}/2/files/upload` with `Dropbox-API-Arg` `{"path":…,"mode":"add","autorename":false,"strict_conflict":true,"mute":true}` (write-once; 409 `path/conflict` → `Io`) or `"mode":"overwrite"`. Parents are created by the upload, so `create_dir_all` is `create_folder_v2` with conflict ignored.
  - `remove`: `/2/files/delete_v2` with `parent_rev` = the listed `rev`. Without a known rev, `get_metadata` first, and a `folder` result is refused (no call).
  - `Dropbox-API-Arg` JSON escapes every non-ASCII character as `\uXXXX`.
  - Team space: when `location.path_root` is `Some(ns)`, every request carries `Dropbox-API-Path-Root: {".tag":"root","root":"<ns>"}`.
  - Errors: 409 `insufficient_space` → `QuotaExceeded`; 429 → `RateLimited`; others through `status_error`.
  - testutil `FakeDropbox::serve(dir: PathBuf, token: &str) -> Self` with the same `Faults` as `FakeDav` plus `outside_path: bool`.

- [ ] **Step 1: Failing tests.**
  - `dropbox_store_passes_the_store_contract`.
  - `listing_entry_outside_the_root_is_dropped`: `outside_path` adds `/Other/a/…` and `/mosaic-evil/x` entries (a root-prefix trick on `/mosaic`); neither is listed.
  - `upload_args_are_ascii_and_write_once`: a root `/Möbius/Mosaic` → the header value is ASCII and contains `\u00f6`, and `mode:add` + `strict_conflict:true` are present.
  - `remove_without_rev_refuses_a_folder`.
  - `dropbox_429_maps_to_rate_limited` (Review Focus 4).
  - `run_listing_over_max_entries_is_protocol`.
  - `path_root_header_sent_for_team_space`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_dropbox`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again,** plus `remote_engine` (add `phone_joins_and_pulls_desktop_items_dropbox` there, over `FakeDropbox`). Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): Dropbox folder store`

### Task 13: Dropbox OAuth, refresh, revocation

**Files:**
- Modify: `crates/sync/src/folder/remote/dropbox.rs`
- Modify: `crates/sync/src/testutil/fake_cloud/dropbox.rs` (token routes, expiry knob)

**Interfaces:**
- Produces:
  - `pub fn dropbox_authorize_url(endpoints: &Endpoints, app_key: &str, pkce: &Pkce, state: &AuthState) -> String`: `{auth}/oauth2/authorize?client_id=…&response_type=code&code_challenge=…&code_challenge_method=S256&token_access_type=offline&redirect_uri=<REDIRECT_URI encoded>&state=…&scope=files.metadata.read%20files.content.read%20files.content.write%20account_info.read`.
  - `pub fn dropbox_exchange(http, endpoints, app_key, code: &str, pkce: &Pkce) -> Result<(ProviderCredential, Option<String> /* team root ns */), TransportError>`. It is a form `POST {api}/oauth2/token` with `grant_type=authorization_code`. Then `POST {api}/2/users/get_current_account`; `root_info.root_namespace_id != root_info.home_namespace_id` gives `Some(ns)`.
  - `pub struct DropboxAuth` (refresh credential plus an in-memory access token in `Zeroizing`, plus `Arc<dyn CredentialSink>`):
    - `fn new(app_key: String, credential: ProviderCredential, sink: Arc<dyn CredentialSink>) -> Self`;
    - `fn bearer(&self) -> Result<HeaderValue, TransportError>`.
  - The refresh is `grant_type=refresh_token` and runs when there is no token yet or after a 401. Each request retries **once** after a refresh. A failed refresh (400 `invalid_grant`, or 401) → `Unauthorized`.
  - `pub fn dropbox_revoke(http, endpoints, auth: &DropboxAuth) -> Result<(), TransportError>`: `POST {api}/2/auth/token/revoke`.

- [ ] **Step 1: Failing tests.**
  - `authorize_url_has_pkce_offline_and_exact_redirect`.
  - `exchange_returns_refresh_credential_and_team_namespace`.
  - `expired_access_token_mid_run_refreshes_once` (Review Focus 3): the fake expires the token after the 3rd request. The run finishes `Ok`, the fake saw exactly one refresh, and no request was sent twice except the one 401 retry.
  - `revoked_refresh_token_ends_run_signed_out` (Review Focus 3): the fake revokes the refresh token mid-run → `SyncError::Transport(Unauthorized)`. `FakeDropbox` lists no final name without its full bytes and no `.mtmp` outside own dirs.
  - `token_never_in_debug_or_error`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_dropbox`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): Dropbox OAuth with PKCE, refresh and revocation`

### Task 14: Graph store

**Files:**
- Create: `crates/sync/src/folder/remote/graph.rs` (store part)
- Create: `crates/sync/src/testutil/fake_cloud/graph.rs`
- Test: `crates/sync/tests/it/remote_graph.rs`

**Interfaces:**
- Produces:
  - `pub struct GraphStore` with `fn new(http, endpoints: &Endpoints, location: &FolderLocation /* drive_id, item_id */, auth: GraphAuth) -> Result<Self, TransportError>`, which implements `FolderStore`. In this task the test uses `GraphAuth::fixed_for_test`.
  - Addressing: `{graph}/v1.0/drives/{drive}/items/{root}:/{encoded rel}:` (the root itself is `items/{root}`).
  - `list`: `…:/children?$select=name,size,lastModifiedDateTime,eTag,file,folder,package,remoteItem,deleted&$top=200`.
    - `@odata.nextLink` is followed only when it starts with `{graph}/v1.0/drives/{drive}/`, up to `MAX_PAGES`; anything else → `Protocol`.
    - Entries with `package`, `remoteItem` or `deleted`, and names that fail `RelPath::child`, are dropped.
    - 404 → empty.
  - `stat`: `GET …:` (404 → `None`).
  - `read_capped`: `GET …:/content` → 302 → `get_following_one_redirect` without `Authorization`; `size` from the metadata is checked first.
  - `write_atomic`:
    - up to 4 MiB: `PUT …:/content?@microsoft.graph.conflictBehavior=fail|replace` (409 `nameAlreadyExists` → `Io`);
    - above 4 MiB: `POST …:/createUploadSession` with `{"item":{"@microsoft.graph.conflictBehavior":"fail|replace"}}`, then chunks of `GRAPH_CHUNK = 10 * 320 * 1024` bytes to the returned `uploadUrl`. The `uploadUrl` must be `https`; it is sent without `Authorization`.
  - `create_dir_all`: `POST …parent:/children` with `{"name":…,"folder":{},"@microsoft.graph.conflictBehavior":"fail"}`; 409 is fine.
  - `remove`: `DELETE` with `If-Match: <eTag>`; a folder is refused locally.
  - Errors: 507 or `quotaLimitReached` → `QuotaExceeded`; 429/503 with `Retry-After` → `RateLimited`.
  - testutil `FakeGraph::serve(dir, token)` with `Faults` plus `foreign_next_link: bool` and a download host on a second `FakeHttp`.

- [ ] **Step 1: Failing tests.**
  - `graph_store_passes_the_store_contract`.
  - `listing_entry_outside_the_root_is_dropped`: a `remoteItem` entry, a name `a/b`, and a `foreign_next_link` → the first two are dropped, the third is `Protocol`.
  - `graph_download_redirect_drops_authorization`.
  - `large_blob_uses_upload_session_with_320k_multiple_chunks`: a 6 MiB blob → 2 chunks of 3 276 800 bytes and 1 of the rest; `conflictBehavior` is `fail` for write-once.
  - `graph_507_is_quota_exceeded`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_graph`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again,** plus `remote_engine::phone_joins_and_pulls_desktop_items_graph`. Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): Microsoft Graph folder store`

### Task 15: Graph OAuth with rotating refresh tokens

**Files:**
- Modify: `crates/sync/src/folder/remote/graph.rs`, `testutil/fake_cloud/graph.rs`

**Interfaces:**
- Produces:
  - `pub fn graph_authorize_url(endpoints, client_id: &str, pkce, state) -> String`: `{ms_login}/common/oauth2/v2.0/authorize?client_id=…&response_type=code&redirect_uri=…&response_mode=query&scope=Files.ReadWrite%20offline_access&state=…&code_challenge=…&code_challenge_method=S256`.
  - `pub fn graph_exchange(http, endpoints, client_id, code, pkce) -> Result<ProviderCredential, TransportError>`: `POST {ms_login}/common/oauth2/v2.0/token`.
  - `pub fn graph_resolve_root(http, endpoints, auth: &GraphAuth, path: &str) -> Result<FolderLocation, TransportError>`: `GET {graph}/v1.0/me/drive/root:/{encoded path}:` → `id` and `parentReference.driveId`. A non-folder → `FolderUnavailable`.
  - `pub fn graph_browse(http, endpoints, auth, drive_id: Option<&str>, item_id: Option<&str>) -> Result<Vec<(String, String /* item id */)>, TransportError>` (folders only, capped at 200).
  - `pub struct GraphAuth` is like `DropboxAuth`. **Rotation:** when a token response carries a new `refresh_token`, `sink.rotated(&new)` must return `Ok` **before** the new access token is used. If the sink fails, the request fails with `Io`, and the old credential stays in place.
  - There is no revoke function (ADR-0162).

- [ ] **Step 1: Failing tests.**
  - `authorize_url_has_pkce_and_scopes`.
  - `rotated_refresh_token_is_stored_before_use` (Review Focus 3): a recording sink and the fake check the order: sink call index < first `GET` with the new access token.
  - `sink_failure_keeps_old_credential_and_fails_io`.
  - `resolve_root_by_path_returns_drive_and_item_ids`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-sync --features test-util,folder-remote --test it remote_graph`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(sync): Graph OAuth with PKCE and rotating refresh tokens`

### Task 16: Store schema: the device-local remote folder location

**Files:**
- Modify: `crates/vault/src/storage/schema.rs` (new migration `V<n>` = LATEST + 1), `crates/vault/src/storage/sync_state.rs`, `crates/vault/src/session/*.rs` (pass-throughs)

**Interfaces:**
- Produces:
  - Migration: `CREATE TABLE folder_remote (id INTEGER PRIMARY KEY CHECK (id = 1), location TEXT NOT NULL CHECK (length(location) BETWEEN 2 AND 4096), updated_at INTEGER NOT NULL) STRICT;`. The table is additive. `sync_config` and `SyncConfig` are untouched. On the phone, `folder_path` holds the provider root path for display.
  - `Store::folder_remote(&self) -> Result<Option<String>>` and `Store::set_folder_remote(&mut self, location: Option<&str>) -> Result<()>`. `None` deletes the row. Setting it while `sync_config.kind != 2` → `Error::InvalidInput`.
  - `LockedSession::folder_remote()`, `UnlockedSession::folder_remote()` and `UnlockedSession::set_folder_remote(Option<&str>)`.
  - The row is cleartext local metadata (CRYPTO_SPEC §11) and never synced. Add one line to §11 in this task.

- [ ] **Step 1: Failing tests** in `sync_state.rs`:
  - `folder_remote_round_trip_and_delete`;
  - `folder_remote_requires_folder_kind`;
  - `migration_from_previous_version_keeps_sync_config` (the existing migration-test pattern of the previous version).
- [ ] **Step 2: Run** `$CARGO test -p mosaic-vault --lib storage::sync_state`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `$CARGO test -p mosaic-vault --lib storage`. Expect PASS.
- [ ] **Step 5: Commit** `feat(vault): store v<n>, device-local remote folder location`

### Task 17: Mobile: provider link flow and error codes

**Files:**
- Modify: `crates/mobile/Cargo.toml` (`mosaic-sync` features `["http", "folder-remote"]`; dev-dependency `mosaic-sync` with `["http", "folder-remote", "test-util"]` for the fakes)
- Create: `crates/mobile/src/folder.rs`
- Modify: `crates/mobile/src/{lib,state,storage,error,test_hooks,setup}.rs`
- Test: `crates/mobile/tests/it/folder.rs` (+ `mod folder;` in `main.rs`)

**Interfaces:**
- Consumes: Tasks 8, 10, 13 and 15.
- Produces:
  - `pub enum FolderProvider { Dropbox, OneDrive, WebDav }`.
  - `pub struct LinkInfo { pub url: String, pub kind: LinkKind }` with `pub enum LinkKind { Redirect, Poll }`, and `pub enum LinkStatus { Waiting, Linked, Expired }`.
  - `pub struct FolderSetupInfo { pub account: String /* hex32 */, pub provider: Option<FolderProvider> /* None = other */, pub server: Option<String>, pub path: Option<String> }`. `SetupCode` gains `pub folder: Option<FolderSetupInfo>`; for a folder code, `server` and `email` are `""`. Both `parse_setup_code` and `scan_setup_qr` use `parse_any`.
  - `Core` methods:
    - `folder_link_begin(provider, server: Option<String>) -> Result<LinkInfo>`;
    - `folder_link_finish(callback_url: String) -> Result<()>`;
    - `folder_link_status() -> Result<LinkStatus>`;
    - `folder_link_password(dav_url: String, login: String, password: Vec<u8>) -> Result<()>` (typed WebDAV: canonical https URL, then a `PROPFIND Depth: 0` check);
    - `folder_link_cancel()`.
  - `pub(crate) struct PendingLink { provider, pkce: Option<Pkce>, state: Option<AuthState>, poll: Option<LoginPoll>, server: Option<String>, credential: Option<ProviderCredential>, expires_at_ms: u64, relink: bool }` in `Slots.link`. It is dropped by `clear_on_lock` unless `keep_join` (the same rule as the join), and dropped on expiry (`LINK_TTL_MS = 300_000`). A `state` mismatch drops it and gives `folderLinkInvalid`. `CallbackError::Denied` gives `folderLinkInvalid`. An expired slot gives `folderLinkExpired`.
  - `pub fn folder_credential_name(account: &AccountId) -> String` (`folder-credential/<hex32>`) in `storage.rs`.
  - `struct SecureSink { storage: Arc<dyn SecureStorage>, name: String }` implements `CredentialSink` with `put(name, encode())`.
  - Client IDs: `DROPBOX_APP_KEY = option_env!("MOSAIC_DROPBOX_APP_KEY")` and `MS_CLIENT_ID = option_env!("MOSAIC_MS_CLIENT_ID")`. A missing or empty ID → `folderProviderUnsupported`. `TestHooks::set_folder_endpoints(origin: &str, dropbox_key: &str, ms_client: &str)` overrides them in tests.
  - Error codes (in `error.rs`, `mobileShared.ts`, FFI in Task 20):
    - new: `folderProviderSignedOut`, `folderNotFound`, `folderStorageFull`, `folderLinkExpired`, `folderLinkInvalid`, `folderServerTlsTooOld`;
    - renamed: `folderAccountUnsupported` → `folderProviderUnsupported`;
    - reused from the desktop catalog: `syncFolderNoAccount`, `syncFolderDamaged`, `syncPasswordRollback`.
  - `pub(crate) fn folder_error(e: TransportError) -> MobileError` maps:
    - `Unauthorized` → `FolderProviderSignedOut`;
    - `QuotaExceeded` → `FolderStorageFull`;
    - `FolderUnavailable` and `NotFound` → `FolderNotFound`;
    - `TlsVersionUnsupported` → `FolderServerTlsTooOld`;
    - the rest as `From<TransportError>`.

- [ ] **Step 1: Failing tests** (`tests/it/folder.rs`, over `FakeDav`/`FakeDropbox`/`FakeGraph` through `TestHooks`):
  - `dropbox_link_begin_finish_stores_nothing_before_join`: begin → the URL carries `code_challenge_method=S256` and the `state`. Finishing with the fake's code and a matching `state` → `Ok`, and `storage.contains("folder-credential/…")` is `false` (no account yet).
  - `forged_callback_state_is_refused_and_drops_the_link`: wrong `state` → `FolderLinkInvalid`; the correct callback afterwards → `FolderLinkInvalid` (slot gone).
  - `link_expires_after_five_minutes` (clock offset +300 001 ms → `FolderLinkExpired`).
  - `lock_drops_link_close_keeps_it`.
  - `nextcloud_poll_waits_then_links`.
  - `provider_without_client_id_is_unsupported`.
  - `folder_setup_code_parses_with_folder_info`.
  - Rename `folder_setup_code_or_config_is_folder_account_unsupported` to `copied_desktop_folder_store_is_provider_unsupported`, asserting `FolderProviderUnsupported`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-mobile --test it folder`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again,** plus `--test it join` (the renamed test). Expect PASS.
- [ ] **Step 5: Commit** `feat(mobile): provider sign-in for folder accounts (PKCE, Login Flow v2)`

### Task 18: Mobile: locate, browse, join a folder account

**Files:**
- Modify: `crates/mobile/src/folder.rs`, `crates/mobile/src/join.rs`
- Test: `crates/mobile/tests/it/folder.rs`

**Interfaces:**
- Consumes: Task 17's `PendingLink.credential`, plus `inspect_in`, `has_account_in` and `join_from_folder_in` (Task 6).
- Produces:
  - `pub struct FolderCandidate { pub path: String, pub accounts: Vec<String> /* hex32, ≤ 64 */ }` and `pub struct FolderDirEntry { pub name: String, pub path: String }`.
  - `folder_locate(path_hint: Option<String>) -> Result<Vec<FolderCandidate>>`. It checks only the hinted path (no drive search): `inspect_in` on a store opened at that root. A `Mosaic` result gives one candidate; anything else gives an empty list.
  - `folder_browse(dir: String) -> Result<Vec<FolderDirEntry>>`. It lists the subdirectories of `dir` (`""` is the provider root), at most 200, sorted by name, built through `RelPath::child` (invalid names are skipped). Graph browses by `graph_browse`.
  - `pub struct JoinFolderRequest { pub path: String, pub account: String, pub password: Vec<u8>, pub secret_key: Vec<u8>, pub device_name: Option<String> }`.
  - `join_folder(JoinFolderRequest) -> Result<SessionState>`. It needs a linked `PendingLink` (else `NotFound`) and an empty store (else `AccountExists`). Then:
    1. open the store at `path` (Graph: `graph_resolve_root`);
    2. `join_from_folder_in`;
    3. `set_sync_config(Folder, folder_path = path)` and `set_folder_remote(location.encode())`;
    4. `put` the Secret Key, then `folder-credential/<account>`;
    5. finish unlocked;
    6. run the first pull (its failure is reported in `last_error`, and the join still counts).

    A failure after step 2 cleans up as `finish_join` does. Error mapping: `JoinError::NoAccountFile` → `SyncFolderNoAccount`, `FolderDamaged` → `SyncFolderDamaged`, `WrongCredentials` → `WrongCredentials`, `Folder(_)` → `FolderNotFound`.
  - **The phone never calls `init_in` and never creates the root.**

- [ ] **Step 1: Failing tests.**
  - `join_folder_over_dav_then_items_visible`: a desktop account made with the sync test helpers in a FakeDav-served dir → join → `items()` shows the desktop's item.
  - `choosing_a_wrong_folder_writes_nothing` (Review Focus 5). Each case is `FakeDav::writes() == 0` before and after:
    - an empty dir → `folder_locate` gives `[]` and `join_folder` gives `FolderNotFound`;
    - a Mosaic folder with another account → `SyncFolderNoAccount`;
    - the parent of the real folder → `FolderNotFound`;
    - a wrong password → `WrongCredentials`, and the link stays usable for a retry.
  - `browse_skips_invalid_names_and_caps_at_200`.
  - `join_folder_without_link_is_not_found`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-mobile --test it folder`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again.** Expect PASS.
- [ ] **Step 5: Commit** `feat(mobile): join a folder account from a provider folder`

### Task 19: Mobile: folder sync, recovery, overview, reset, sign in again

**Files:**
- Modify: `crates/mobile/src/{sync,lib,state,folder}.rs`
- Test: `crates/mobile/tests/it/folder.rs`

**Interfaces:**
- Consumes: Tasks 6, 12–18.
- Produces:
  - `Inner.folder_engine: Option<SyncEngine<FolderTransport<SessionCrypto<OwnedSession>>>>`. It is built on the first folder run after unlock, and dropped by `lock_with` and `reset_device`. `sync_once` takes it out of the mutex for the run and puts it back only when the lock generation is unchanged. Each run calls `engine.transport_mut().begin_run()` before `engine.sync_once`.
  - `sync_once` dispatches on `config.kind`. For `Folder` with `folder_remote` present: read the credential (missing → `FolderProviderSignedOut`), build the store for `FolderLocation.provider`, then map errors with `folder_error`. `Folder` without `folder_remote` → `FolderProviderUnsupported`.
  - `recover_password` for folder accounts mirrors the desktop's `recover_folder`: `verified_account_file`, then the version check against `folder_account_version` (`<=` → `SyncPasswordRollback`), then `derive_join_keys`, `adopt_remote_password` and `raise_folder_account_version`.
  - `SyncOverview` gains:
    - `pub folder: Option<FolderOverview { provider: FolderProvider, path: String }>`;
    - `pub reset_phrase: String`: the email for hosted accounts, the first 8 hex characters of the account ID for folder accounts.

    `configured` is true for both kinds. `signed_in` for folder accounts = `contains(folder-credential/…)`. `reset_device` compares `typed_email` with `reset_phrase`. After the secrets are deleted it revokes best effort: Dropbox `dropbox_revoke`, WebDAV `revoke_app_password` from a Login Flow credential, Graph nothing. A failed revocation does not stop the reset.
  - Sign in again: `folder_link_begin` while unlocked on a folder account sets `relink = true` and refuses another provider (`InvalidInput`). When `finish`/`status`/`password` complete a relink, they check that `has_account_in` holds at the stored location (else `SyncFolderNoAccount`, nothing stored), then `put` the credential and drop `folder_engine`.

- [ ] **Step 1: Failing tests.**
  - `folder_sync_round_trip_dav_dropbox_graph`: for each fake, desktop item → phone; phone edit → desktop sees it after a `CloudSim` round.
  - `engine_kept_between_runs_does_not_redownload_segments`: the second run makes 0 `GET`s of segments the first run read.
  - `lock_drops_the_kept_engine_and_access_token`.
  - `signed_out_provider_reports_and_relink_restores` (revoked refresh → `FolderProviderSignedOut`; relink → next sync `Ok`).
  - `relink_to_folder_without_account_stores_nothing`.
  - `folder_password_recovery_adopts_newer_file_and_refuses_rollback`.
  - `folder_reset_uses_short_id_phrase_and_revokes_dropbox`.
  - `storage_full_maps_to_folder_storage_full`.
  - `tls12_only_dav_server_is_tls_too_old`: a unit test of `folder_error`.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-mobile --test it folder`. Expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run again,** plus `$CARGO test -p mosaic-mobile --test it -- sync join lifecycle` (hosted unchanged). Expect PASS.
- [ ] **Step 5: Commit** `feat(mobile): folder account sync, recovery, reset and sign in again`

### Task 20: FFI

**Files:**
- Modify: `crates/mobile-ffi/src/{lib,types,convert}.rs`

**Interfaces:**
- Produces:
  - UniFFI enums `FolderProvider`, `LinkKind` and `LinkStatus`;
  - records `LinkInfo`, `FolderSetupInfo`, `FolderCandidate`, `FolderDirEntry`, `JoinFolderRequest` and `FolderOverview`;
  - `SetupCode.folder: Option<FolderSetupInfo>`, and `SyncOverview.folder` plus `reset_phrase`;
  - the exported methods `folder_link_begin`, `folder_link_finish`, `folder_link_status`, `folder_link_password`, `folder_link_cancel`, `folder_locate`, `folder_browse` and `join_folder` (signatures as in Tasks 17–18, wrapped with `lift`);
  - the error variants of Task 17 (rename included).

- [ ] **Step 1: Failing tests:** add round trips to the existing `rt::<f::X, m::X>` test for every new record and enum, and the error-variant list test.
- [ ] **Step 2: Run** `$CARGO test -p mosaic-mobile-ffi` and `bash scripts/mobile/check-ffi-crate.sh`. Expect FAIL, then PASS after Step 3.
- [ ] **Step 3: Implement** with the `record!`/`record_out!` macros.
- [ ] **Step 4: Commit** `feat(mobile-ffi): folder account calls`

### Task 21: Android: redirect, controller, onboarding, strings

**Files:**
- Create: `apps/mobile/android/app/src/main/kotlin/io/github/stephanbaum/mosaic/session/OAuthCallback.kt` (pure helper), `…/ui/OAuthRedirectActivity.kt`, `…/ui/onboarding/FolderSteps.kt`
- Modify: `AndroidManifest.xml`, `…/session/SessionController.kt`, `…/ui/onboarding/OnboardingFlow.kt`, `…/scan/ScanScreen.kt` (pass `SetupCode.folder` through)
- Modify: `packages/i18n/src/catalog/{en,de}/mobile.ts`, `packages/i18n/src/catalog/mobileShared.ts`
- Test: `apps/mobile/android/app/src/test/kotlin/io/github/stephanbaum/mosaic/session/OAuthCallbackTest.kt`, `…/ui/L10nTest.kt` (the new keys exist)

**Interfaces:**
- Consumes: Task 20's FFI.
- Produces:
  - `object OAuthCallback { const val SCHEME = "io.github.stephanbaum.mosaic.oauth"; fun accepts(text: String): Boolean }`. It is true only for `SCHEME + ":/cb?"` + at most 4 096 characters in total.
  - Manifest: `OAuthRedirectActivity` with `exported="true"`, `noHistory="true"`, `excludeFromRecents="true"` and a translucent theme. Its intent-filter has `VIEW` + `DEFAULT` + `BROWSABLE`, `<data android:scheme="io.github.stephanbaum.mosaic.oauth" android:path="/cb"/>`. `onCreate` does this:
    1. if `accepts`, call `session.folderLinkFinish(text)`;
    2. drop the string;
    3. start `MainActivity` with `FLAG_ACTIVITY_CLEAR_TOP | FLAG_ACTIVITY_SINGLE_TOP`;
    4. `finish()`.

    It never logs, and it never keeps the URI in saved state.
  - `SessionController`:
    - `folderLinkBegin`, `folderLinkFinish`, `folderLinkStatus`, `folderLinkPassword`, `folderLinkCancel`, `folderLocate`, `folderBrowse`, `joinFolder`;
    - `val folderLink: StateFlow<FolderLinkEvent?>` (`Linked`, `Failed(code)`), set by `folderLinkFinish`.
  - Onboarding steps:
    1. `FolderIntro`: "This account syncs through your {provider} folder".
    2. `FolderSignIn`: the system browser opens with `ACTION_VIEW`. Only `https` URLs are opened (as in `openInBrowser`). Nextcloud polls `folderLinkStatus` every 2 s in the foreground, up to `Expired`. Typed WebDAV fields go to `folderLinkPassword`.
    3. `FolderLocate`: `folderLocate(hint)`, which falls through to `FolderBrowse` (directory list, up, "Use this folder").
    4. The existing `Password` step calls `joinFolder`.

    "Use an existing account" without a code shows the provider choice (Dropbox, OneDrive, Nextcloud/WebDAV), the Secret Key field, and then the same steps.
  - Strings (`mobile.folder.*`, EN / DE):

    | Key | EN | DE |
    |---|---|---|
    | `intro` | This account syncs through your {provider} folder. | Dieses Konto synchronisiert über deinen {provider}-Ordner. |
    | `signIn` | Sign in to {provider} | Bei {provider} anmelden |
    | `scope` | {provider} will ask for access to all your files. Mosaic only opens the folder you choose and never touches anything else. | {provider} fragt nach Zugriff auf alle deine Dateien. Mosaic öffnet nur den Ordner, den du auswählst, und rührt sonst nichts an. |
    | `waiting` | Finish signing in in your browser, then come back here. | Schließ die Anmeldung im Browser ab und komm dann hierher zurück. |
    | `serverLabel` | Nextcloud or WebDAV address | Nextcloud- oder WebDAV-Adresse |
    | `locating` | Looking for your Mosaic folder… | Suche deinen Mosaic-Ordner … |
    | `browseTitle` | Choose your Mosaic folder | Wähle deinen Mosaic-Ordner |
    | `browseHint` | It's the folder your computer syncs, with a file named mosaic-sync.json. | Es ist der Ordner, den dein Computer synchronisiert, mit einer Datei namens mosaic-sync.json. |
    | `useThis` | Use this folder | Diesen Ordner verwenden |
    | `up` | Up one level | Eine Ebene höher |
    | `chooseProvider` | Where does your account sync? | Wo synchronisiert dein Konto? |
    | `noDrive` | Google Drive and iCloud Drive don't work on Android yet. Use Dropbox, OneDrive, Nextcloud or Mosaic sync. | Google Drive und iCloud Drive funktionieren unter Android noch nicht. Nimm Dropbox, OneDrive, Nextcloud oder die Mosaic-Synchronisierung. |

    The error texts (`mobile.error.<code>`):

    | Code | EN | DE |
    |---|---|---|
    | `folderProviderSignedOut` | You're signed out of your cloud storage. Sign in again in Settings. | Du bist bei deinem Cloud-Speicher abgemeldet. Melde dich in den Einstellungen neu an. |
    | `folderNotFound` | Mosaic can't find this folder. Choose the folder your computer syncs. | Mosaic findet diesen Ordner nicht. Wähle den Ordner, den dein Computer synchronisiert. |
    | `folderStorageFull` | Your cloud storage is full. Free up space, then sync again. | Dein Cloud-Speicher ist voll. Gib Platz frei und synchronisiere dann erneut. |
    | `folderLinkExpired` | Signing in took too long. Try again. | Die Anmeldung hat zu lange gedauert. Versuch es noch einmal. |
    | `folderLinkInvalid` | Signing in didn't finish. Try again. | Die Anmeldung wurde nicht abgeschlossen. Versuch es noch einmal. |
    | `folderProviderUnsupported` | This phone can't reach this account's folder. Use a computer, or switch to Dropbox, OneDrive, Nextcloud or Mosaic sync. | Dieses Handy erreicht den Ordner dieses Kontos nicht. Nimm einen Computer oder wechsle zu Dropbox, OneDrive, Nextcloud oder der Mosaic-Synchronisierung. |
    | `folderServerTlsTooOld` | This server only offers an older, less secure connection (TLS 1.2). Mosaic needs TLS 1.3. Ask the server's admin to turn it on. | Dieser Server bietet nur eine ältere, weniger sichere Verbindung (TLS 1.2). Mosaic braucht TLS 1.3. Bitte die Person, die den Server betreut, das einzuschalten. |

- [ ] **Step 1: Failing JVM test** `OAuthCallbackTest`: the exact callback → `true`; `…:/cb2?x`, `https://…`, `…:/cb` without a query, and 4 097 characters → `false`.
- [ ] **Step 2: Implement** the Kotlin and the catalogs. Then run `pnpm --filter @mosaic/i18n export:mobile` and `pnpm --filter @mosaic/i18n test`, and check that `bash scripts/mobile/check-no-logging.sh` passes locally.
- [ ] **Step 3: Commit, push, check CI** `feat(android): folder account sign-in and join` — `mobile.yml` builds and runs the JVM tests. A red CI gets a fresh, small fix commit; do not poll in a loop.

### Task 22: Android: settings for folder accounts

**Files:**
- Modify: `…/ui/settings/SettingsScreen.kt`, `…/ui/vault/*` (the sync line), `…/ui/onboarding/FolderSteps.kt` (reuse sign-in for relink), the reset confirmation dialog
- Modify: `packages/i18n/src/catalog/{en,de}/mobile.ts`

**Interfaces:**
- Consumes: `SyncOverview.folder`, `reset_phrase`, and `folderLinkBegin` while unlocked (relink).
- Produces the settings section:
  - "Syncs through {provider}: {path}" / "Synchronisiert über {provider}: {path}" (`mobile.folder.settingsLine`);
  - a "Sign in again" / "Neu anmelden" button (`mobile.folder.relink`), shown when `last_error == folderProviderSignedOut` or `!signed_in`;
  - the `scope` text under it;
  - for OneDrive only, `mobile.folder.msRevokeHint`:
    - EN: "To remove Mosaic's access completely, remove it on your Microsoft account page under Apps."
    - DE: "Um Mosaics Zugriff ganz zu entfernen, entferne die App auf der Seite deines Microsoft-Kontos unter Apps."
  - the reset dialog asks for `reset_phrase` (`mobile.reset.typePhrase`: "Type {phrase} to confirm" / "Gib {phrase} ein, um zu bestätigen").

- [ ] **Step 1: Implement,** then run the i18n export and test.
- [ ] **Step 2: Ask the ux-reviewer agent** to review the screens (Compose screenshots come from CI's `ScreenshotTest`; add the folder steps and the settings section to it).
- [ ] **Step 3: Commit and push** `feat(android): folder account settings, sign in again, reset phrase`

### Task 23: Docs, owner checklist, final gate

**Files:**
- Modify: `docs/MOBILE.md` (§1 architecture: remote stores; §2 FFI contract: the new calls and codes; §3 security: tokens, redirect, revocation; §5 tests: the new suites; §6 OD-M10-11 row becomes "Android: resolved (ADR-0162, Dropbox, OneDrive, Nextcloud/WebDAV); iOS: deferred (iCloud Drive and File Provider)"; §6 owner checklist; §7 manual device checks for the three providers)
- Modify: `docs/PROGRESS.md` (Android folder sync section, the open owner actions)
- Modify: `CLAUDE.md` Layout (`crates/sync`: "`folder/remote`: WebDAV, Dropbox and Graph folder stores (feature `folder-remote`)")

- [ ] **Step 1: Owner checklist** in MOBILE.md §6, as checkboxes:
  - **Dropbox:**
    - create an App Console app (Scoped access, **Full Dropbox**, the four scopes, name "Mosaic", icon, branding);
    - add the redirect URI `io.github.stephanbaum.mosaic.oauth:/cb`, and check that the console accepts a custom scheme for PKCE apps. If it does not, record it, and the builder switches `REDIRECT_URI` per provider;
    - set `MOSAIC_DROPBOX_APP_KEY` in the release build secrets;
    - apply for production at 50 linked users (or request early review), with the justification "existing user-chosen folder, including team folders; access confined in code".
  - **Microsoft:**
    - create an Entra app registration for "personal and organisational accounts", platform "Mobile and desktop applications", redirect `io.github.stephanbaum.mosaic.oauth:/cb` (check that the custom URI is accepted), public client flows on, no secret;
    - set `MOSAIC_MS_CLIENT_ID`;
    - later: publisher verification (Partner Program account, verified domain; OD-M9-6, OD-M10-12).
  - **Privacy policy:** the phone connects straight to the cloud provider the user picks; Mosaic runs no server in between and collects nothing; the provider sees the metadata of CRYPTO_SPEC §12.1.
  - **Play data-safety form:** no data collected or shared by the developer. The provider connection is user-initiated transfer to a service the user chooses, encrypted in transit.
  - **Review:** THREAT_MODEL R166–R170, CRYPTO_SPEC §6.9.7/§10.1/§12.1, ADR-0161/0162.
  - **Google Drive:** skipped at launch; revisit when revenue pays for the assessment (owner answer 1).
- [ ] **Step 2: Reviewers,** one at a time:
  - **crypto-reviewer** on `crates/sync/src/folder/remote/{oauth,credential}.rs`, `crates/core/src/setup_code.rs` and `crates/mobile/src/folder.rs`;
  - **sync-chaos** on `remote_engine.rs`;
  - **test-engineer** on `folder/remote`, for untested paths in the parsers.

  Fix the findings in one commit.
- [ ] **Step 3: Full gate once.** Run `FULL_RUN_OK=1` with every command of CLAUDE.md "Commands" (with `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS=2`), plus `$CARGO test -p mosaic-sync --features test-util,folder-remote` and `$CARGO clippy -p mosaic-sync --features test-util,folder-remote --all-targets -- -D warnings`. Then end any leftover cargo, rustc or node processes.
- [ ] **Step 4: Commit** `docs(mobile): Android folder sync, OD-M10-11 resolved for Android, owner checklist`
