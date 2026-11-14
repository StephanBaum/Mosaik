# Windows Program Fill, Phase 1 (Go/No-Go and Identity) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decide whether Windows program autofill can be built within Mosaic's rules (vetting spike, ADR), and if so build its foundation: the `winapp://` program identity and its matching, the identity capture (exe path plus Authenticode signer, or package), the binding policy for unsigned programs, and the re-verification before every write.

**Architecture:** Pure rules live where the Android ones live: `crates/origin/src/program.rs` (parse, canonical form, matching, exclusion classes). A new crate `crates/program-fill` (`mosaic-program-fill`, `#![forbid(unsafe_code)]`) holds the identity model, the binding policy and the re-verification as pure, host-tested functions behind a `ProgramProbe` trait with a fake. The Windows side of that trait calls vetted crates where they pass, and a new isolated shim crate `crates/winshim` (`mosaic-winshim`) for the rest: the only crate in the workspace allowed `unsafe`, with a safe, minimal API (owner answer O2). Phase 1 stops before the picker UI: no chip, list, hotkey, typing, submit or setting.

**Tech Stack:** Rust 1.99 (edition 2024), `windows =0.62.2` (already in the tree; Win32 features added in the shim only), `sha2`, `base64ct`, `thiserror`, `proptest`, `tempfile`, cargo-fuzz (CI fuzz-smoke job).

**Spec:** `docs/superpowers/specs/2026-11-14-autofill-everywhere-design.md` §5.1.1 to §5.1.3, §6.2 "Part 1", §6.3 "Program fill", §7 Q2 and Q3, §8 O2 (resolved), C2, C3.

**Base:** `main` after M10 (PR #69) and `fix/redteam-round2` are merged (the matcher copies `crates/origin/src/app.rs` from M10; ADR numbers continue after ADR-0156). Branch `feat/windows-program-fill-p1`.

## Global Constraints

- `#![forbid(unsafe_code)]` in every crate except `crates/winshim`. In `winshim`, `unsafe` sits only in modules named `sys*`, every `unsafe` block has a `// SAFETY:` comment (enforced by `clippy::undocumented_unsafe_blocks = "deny"`), one unsafe operation per block (`clippy::multiple_unsafe_ops_per_block = "deny"`), and the public API has no `unsafe fn`, no raw pointer, no `windows` crate type and no raw handle.
- The shim covers only what no vetted crate offers (O2). A vetted crate is preferred where one passes Task 1.
- Program identity is never the window title, caption or version resource. Display data is never read by the matcher.
- Browsers, script and interpreter hosts, Mosaic's own processes, credential and UAC brokers, and remote-desktop and VM clients are never bindable and never match (spec §5.1.2).
- Elevated targets (integrity above Mosaic's) are refused. Mosaic never asks for `uiAccess` or elevation.
- No fill without a pick (brief §4.1): phase 1 writes nothing into any program; it only captures and verifies.
- Library code returns `thiserror` results; no `unwrap`/`expect`/`panic`/`println!` outside tests and the example harness (`#![expect(clippy::print_stdout, reason = "manual Windows check harness")]`).
- No secret is handled in phase 1. Identity data is not secret but must not be logged.
- New dependency → one-line justification in the commit message and a row in `docs/DEPENDENCIES.md`; exact `=` pins; `cargo deny check` and `cargo audit --deny warnings` pass.
- Windows-only code is behind `#[cfg(windows)]`; every crate compiles (empty where needed) on Linux and macOS, because CI builds the workspace on all three.
- Interactive UI Automation, focus and window tests are not CI tests (C3): they are `#[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]` and the manual checklist.
- **Runs anywhere:** Tasks 2, 3, 4, 5. **Needs a Windows machine:** Tasks 1, 6, 7 (build, test and manual checks).
- Verification commands (Git Bash, worktree root), always with the shared target dir:
  `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p <crate> <filter>`;
  `cargo fmt -p <crate> -- --config newline_style=Auto`;
  `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo clippy -p <crate> --all-targets -- -D warnings`.
- Scratch work (the spike's throwaway projects, measurement logs) goes in the system temp dir, never in the repo.
- Agent guard: write only inside the worktree and its `.agent-scope`; no searches outside the repo, `node_modules` or the cargo registry; at most 3 full-workspace runs per hour (`FULL_RUN_OK=1` for a deliberate final gate); never `git worktree remove`.
- Conventional Commits.

## Review Focus

1. **Exe names in odd case or script** (`DISCORD.EXE`, `Discord.Exe`, a Cyrillic `о` in `discоrd.exe`, a dotless `ı`): the user expects the same program to match however Windows reports its case, and a homoglyph program to never match and to be flagged at link time. Tests `exe_names_fold_case_but_not_script` (Task 2) and `lookalike_exe_is_a_warning_never_a_match` (Task 4).
2. **Paths in non-final forms** (`\\?\C:\…`, `C:\PROGRA~1\…`, `C:/x`, UNC, a trailing dot): the user expects one stored form and a case-insensitive match; the parser expects only the stored canonical form. Test `path_urls_accept_only_final_drive_or_unc_paths` (Task 2).
3. **Huge unsigned programs** (over 512 MiB): the user expects a clear "can't link" instead of a long hang. Test `unsigned_image_over_the_cap_cannot_be_bound` (Task 3) with a small injected cap.
4. **Odd window geometry** (a minimised window's rectangle at `-32000`, a zero-size or inverted rectangle, huge coordinates near `i32::MAX`): the user expects no fill and no crash. Tests `probe_points_refuse_tiny_or_inverted_rects` and `probe_points_do_not_overflow` (Task 5).
5. **Clock edge cases** (pick time after "now", a wrapped or huge monotonic value): the user expects the session to count as expired, never as valid forever. Test `a_session_from_the_future_or_long_ago_is_expired` (Task 5).

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `docs/DECISIONS.md` | modify | ADR-0157 (vetting, winshim rule), ADR-0158 (program identity and binding) |
| `docs/THREAT_MODEL.md` | modify | §4.9 supply-chain row; new §4.10 "Desktop program fill (Windows)"; residual rows |
| `docs/DEPENDENCIES.md` | modify | `windows` row (shim features); a row per vetted crate |
| `docs/PROGRAM_FILL.md` | create | phase 1 design notes and the manual Windows checklist (C3) |
| `CLAUDE.md` | modify (Layout) | name the two new crates and the shim rule |
| `crates/origin/src/program.rs`, `crates/origin/src/program_tests.rs`, `crates/origin/src/lib.rs` | create / modify | `winapp://` URLs, matching, classes |
| `crates/origin/fuzz/{Cargo.toml,fuzz_targets/program_url.rs,seeds/program_url/*}` | modify / create | fuzz target |
| `crates/program-fill/{Cargo.toml,src/lib.rs,src/identity.rs,src/policy.rs,src/verify.rs,src/fake.rs}` | create | pure model, policy, verification, fake probe |
| `crates/program-fill/src/windows/{mod.rs,uia.rs}`, `crates/program-fill/examples/probe.rs` | create | Windows probe (Task 7) |
| `crates/winshim/{Cargo.toml,src/lib.rs,src/sys_process.rs,src/sys_trust.rs,src/sys_window.rs,src/sys_uia.rs}` | create | the shim (Task 6) |
| `scripts/check-unsafe-confined.sh`, `.github/workflows/ci.yml` | create / modify | keep `unsafe` confined to `crates/winshim` |

---

### Task 1: Vetting spike and the go/no-go decision (Windows machine)

**Files:**
- Modify: `docs/DECISIONS.md` (append ADR-0157; check the number: the highest `## ADR-` on `main`, and ADR-0156 on `fix/redteam-round2`; use the next free one and use it everywhere this plan says ADR-0157, ADR-0158 being the one after)
- Modify: `docs/THREAT_MODEL.md` §4.9 (one row)
- Modify: `docs/DEPENDENCIES.md` (only for crates that pass)
- Scratch (not committed): one cargo project per candidate under `%TEMP%\mosaic-spike\<crate>`, with the repo's `rust-toolchain.toml`, `deny.toml` and `.cargo/audit.toml` copied in

**Interfaces:**
- Consumes: spec §5.1.1 (APIs and candidates), §5.1.2, §5.1.3, C2.
- Produces: ADR-0157 with the six decisions D1 to D6 below. Tasks 6 and 7 build exactly what D1 and D2 say. If D2 renames or drops anything in the "Proposed shim API" of Task 6, the orchestrator edits Tasks 6 and 7 of this plan to match before Task 6 starts.

**Capabilities to source** (each gets exactly one source in D1):

| ID | Capability | APIs |
|---|---|---|
| C-UIA-READ | focused element and element at a point, with one `CacheRequest`; `CompareElements`; parent walk; connection and transaction timeouts | `IUIAutomation(2)`, `GetFocusedElementBuildCache`, `ElementFromPointBuildCache`, `CompareElements`, `ControlViewWalker`, `put_ConnectionTimeout`, `put_TransactionTimeout`; properties `ControlType`, `IsPassword`, `IsEnabled`, `IsOffscreen`, `IsKeyboardFocusable`, `BoundingRectangle`, `ProcessId`, `NativeWindowHandle`, `RuntimeId`, `FrameworkId`, `ValuePattern` availability and `IsReadOnly` |
| C-UIA-EVENTS (phase 2) | focus-changed and property-changed handlers on an MTA worker | `AddFocusChangedEventHandler`, `AddPropertyChangedEventHandler` |
| C-UIA-WRITE (phase 2) | `ValuePattern.SetValue`, `SetFocus` | `IUIAutomationValuePattern`, `IUIAutomationElement::SetFocus` |
| C-PROC | a held process handle; image path; final path; package family name; token integrity level; still running | `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION \| SYNCHRONIZE)`, `QueryFullProcessImageNameW`, `CreateFileW` + `GetFinalPathNameByHandleW`, `GetPackageFamilyName`, `OpenProcessToken` + `GetTokenInformation(TokenIntegrityLevel)`, `WaitForSingleObject` |
| C-WINDOW | foreground window, root window, owner PID, cloaked, minimised | `GetForegroundWindow`, `GetAncestor(GA_ROOT)`, `GetWindowThreadProcessId`, `DwmGetWindowAttribute(DWMWA_CLOAKED)`, `IsIconic` |
| C-TRUST | Authenticode verdict (embedded and catalog), leaf signer subject DER, CN and O for display, revocation state | `WinVerifyTrust(WINTRUST_ACTION_GENERIC_VERIFY_V2)`, `CryptCATAdmin*` for catalogs, `WTHelperProvDataFromStateData`, `WTHelperGetProvSignerFromChain`, `CertGetNameStringW` |
| C-INPUT (phase 2) | `SendInput` with `KEYEVENTF_UNICODE`, return count; modifier state | `SendInput`, `GetAsyncKeyState` |

**Vetting criteria** (a crate passes only if all hold; record each per crate in a table like ADR-0078's):
- V1 `unsafe` confined: count `unsafe` blocks and `unsafe impl`; Mosaic calls only safe functions; no open soundness issue in its tracker.
- V2 maintained: a release in the last 12 months and at least one active maintainer.
- V3 licence in the `deny.toml` allow list, with no new exception.
- V4 `cargo deny check` and `cargo audit --deny warnings` clean in the scratch project.
- V5 reuses `windows =0.62.2` or a `windows-sys` version already in `Cargo.lock`; a second `windows` major/minor fails.
- V6 exposes every API of the capability it is proposed for.
- V7 no side effects: no global hook, no undocumented thread, COM apartment under Mosaic's control (MTA for UIA).

**Candidates:** `uiautomation` (C-UIA-*), `winsafe` (C-PROC, C-WINDOW, C-INPUT), `sysinfo` (C-PROC path only; fails V6 if it reads the path through a snapshot instead of a held handle), `windows-permissions` (token integrity), `codesign-verify` and `authenticode` (C-TRUST; `authenticode` can only complement the OS verdict), `enigo` and `rdev` (C-INPUT). A capability with no passing crate goes to the shim.

- [ ] **Step 1: Vet each candidate in its own scratch project**

For each: add it as the only dependency with an exact pin; `cargo tree -e normal` (count, `windows` versions), `cargo deny check`, `cargo audit --deny warnings`, read its `unsafe` blocks for the APIs in the table, check its last release date and issue tracker. Record one table row per crate with V1 to V7 and the result.

- [ ] **Step 2: Prototype every shim-assigned capability in a scratch shim**

In `%TEMP%\mosaic-spike\shim`, against `windows =0.62.2` with the needed Win32 features, call each API once from a small binary: own process (C-PROC), the foreground window (C-WINDOW), `C:\Windows\System32\notepad.exe` (or `kernel32.dll`, catalog-signed) and an unsigned copy of a test exe (C-TRUST), the focused element of Notepad and of PowerShell ISE (C-UIA-READ). Record which calls worked, the list of `windows` features, and the number of `unsafe` blocks the prototype needed.

- [ ] **Step 3: Hung-provider bound**

In the scratch project, point the UIA client at a test window whose UIA provider sleeps 60 s (a tiny scratch WinForms or Win32 program that blocks its message loop after focus). Measure: does `GetFocusedElement` return within the transaction timeout (set 2000 ms)? If not, does abandoning the worker thread and creating a new client on a new thread recover within 5 s? Record the result.

- [ ] **Step 4: Builder check C2, idle cost**

With a UIA client registered for focus-changed events and otherwise idle, measure each target before (5 minutes) and with the client (5 minutes), sampling every 5 s with PowerShell (`Get-Process -Name <target>` summed over all processes of the target: `CPU` delta per interval / logical CPUs, and `PrivateMemorySize64`):
- Chrome with one ordinary page open;
- an Electron app (VS Code with one folder open);
- a WPF app (PowerShell ISE);
- a Win32 app (Notepad);
- and the client process itself.
Record a table: target, idle CPU % and private MB without and with the client, delta.

- [ ] **Step 5: Answer the "Not verified" items the spike can reach**

Record yes/no with the evidence: UIA timeouts exposed (by the crate or the shim); signer subject DER reachable from `WinVerifyTrust` state data; WinVerifyTrust behaviour offline (revocation server unreachable: which `HRESULT`); whether Tauri 2.12 can create a non-activating window (`focusable(false)`) — recorded for phase 2, not a gate.

- [ ] **Step 6: Write ADR-0157 "Windows program fill: wrapper vetting and the `mosaic-winshim` exception"**

Context (O2 answer, quoted), the vetting table, the prototype and measurement results, then **Decision**, exactly these six items:
- **D1 Sources:** for each capability ID of the table above: `<crate> =<version>` or `mosaic-winshim` or "not built in phase 1".
- **D2 Shim scope:** the final public API of `mosaic-winshim` for phase 1 (start from "Proposed shim API" in Task 6, minus anything a crate covers), its `windows` feature list, and the rule: `crates/winshim` is the only crate with `unsafe`; `unsafe` only in `sys*` modules; every block has `// SAFETY:`; safe minimal API with no `windows` types, raw pointers or raw handles; tests for every public function; a Fable red-team review of the whole crate before its first merge and on every change; the crypto-reviewer's reading of each `unsafe` block is recorded in the ADR's review log.
- **D3 C2 policy:** if the client adds more than 5 % idle CPU or more than 150 MB to Chrome or the Electron app, Chromium- and Electron-based programs are opt-in per program in phase 2; else they follow the global setting.
- **D4 Signature states:** `Trusted` (chain to a root in the Windows store, code-signing EKU), `Unsigned` (`TRUST_E_NOSIGNATURE`), `Untrusted` (any other failure: bad hash, untrusted root, explicit distrust), `Revoked` (`CERT_E_REVOKED`). Revocation that cannot be checked offline maps to `Trusted` (the OS runs such programs; recorded as a residual) unless Step 5 shows Windows itself reports it as a failure, in which case it maps to `Untrusted`. Only `Trusted` and `Unsigned` can be bound.
- **D5 UIA timeouts:** connection and transaction timeout 2000 ms each; a hung provider is handled by the Step 3 result (timeout, or worker abandonment and respawn).
- **D6 GO or NO-GO.**

**NO-GO** (Tasks 2 to 7 stop; the orchestrator reports to the owner and nothing more of part 1 is built) if any of:
1. C-UIA-READ, C-PROC, C-WINDOW or C-TRUST has no source: no crate passes, and the Step 2 shim prototype could not make it work.
2. Step 3: neither the timeouts nor worker respawn recover from a hung provider within 5 s.
3. Step 4: the client adds more than 5 % idle CPU or 150 MB to the **WPF or Win32** target (the cost would hit programs that can't be opted out).
4. The `windows` features or a chosen crate make `cargo deny check` or `cargo audit --deny warnings` fail in the scratch project.

Otherwise **GO**. Chromium/Electron cost over the threshold is GO with D3's opt-in, not NO-GO (C2). Phase 2 capabilities (C-UIA-EVENTS, C-UIA-WRITE, C-INPUT) are recorded but do not gate phase 1; if one has no source, the phase 2 plan says what is not built.

- [ ] **Step 7: Threat row and dependency rows**

THREAT_MODEL §4.9, new row: "Wrapper crates and `mosaic-winshim` run `unsafe` Windows calls inside the vault process (program fill) | ADR-0157 vetting; exact pins; `unsafe` only in `crates/winshim`, each block with a `// SAFETY:` reading; Fable red-team review before the first merge and on every change; a re-vetting on every version bump". `docs/DEPENDENCIES.md`: one row per crate that D1 admits (crate, used by `program-fill`, reason, ADR-0157), and the `windows` row's "Used by" gains `winshim (Win32 features, ADR-0157)` if the shim is used.

- [ ] **Step 8: Commit**

```bash
git add docs/DECISIONS.md docs/THREAT_MODEL.md docs/DEPENDENCIES.md
git commit -m "docs(adr): ADR-0157 Windows program fill vetting and the mosaic-winshim exception"
```

---

### Task 2: `winapp://` program URLs and matching in `crates/origin` (runs anywhere)

Starts only if ADR-0157 says GO.

**Files:**
- Modify: `docs/DECISIONS.md` (ADR-0158 "Windows program identity and binding", before the code)
- Modify: `docs/THREAT_MODEL.md` (new §4.10 with three rows)
- Create: `crates/origin/src/program.rs`, `crates/origin/src/program_tests.rs`
- Modify: `crates/origin/src/lib.rs` (`pub mod program;`, `#[cfg(test)] mod program_tests;`, module doc line)
- Modify: `crates/origin/fuzz/Cargo.toml` (`[[bin]] program_url`); Create: `crates/origin/fuzz/fuzz_targets/program_url.rs`, seeds `crates/origin/fuzz/seeds/program_url/{package,signed,path,bad_query}`

**Interfaces:**
- Consumes: `mosaic_core::model::LoginUrl` (`url.as_str()`), `crate::{MatchKind, Refusal}`, the private `crate::osa_distance_is_one`, `base64ct::Base64UrlUnpadded`.
- Produces (`mosaic_origin::program`):

```rust
pub const MAX_EXE_NAME_LEN: usize = 255;      // decoded UTF-8 bytes
pub const MAX_PROGRAM_PATH_LEN: usize = 4096; // decoded UTF-8 bytes

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)] #[non_exhaustive]
pub enum ProgramUrlError { NotAProgramUrl, BadPackage, BadExe, BadSigner, BadPath, BadHash }

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProgramUrl {
    Package { family: String },
    Signed { exe: String, signer: [u8; 32] },
    Path { path: String, sha256: [u8; 32] },
}
impl ProgramUrl {
    pub fn package(family: &str) -> Result<Self, ProgramUrlError>;          // ASCII-lowercases, validates
    pub fn signed(exe_name: &str, signer_subject_sha256: [u8; 32]) -> Result<Self, ProgramUrlError>; // folds
    pub fn path(final_path: &str, file_sha256: [u8; 32]) -> Result<Self, ProgramUrlError>;
    pub fn parse(s: &str) -> Result<Self, ProgramUrlError>;                 // canonical form only (outer whitespace trimmed)
    pub fn canonical(&self) -> String;
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ProgramClass { Ordinary, Browser, Host, Mosaic, Broker, Remote }

#[derive(Clone, Copy, Debug)]
pub struct ProgramIdentity<'a> {
    pub package_family: Option<&'a str>,          // folded
    pub exe_name: &'a str,                        // folded
    pub signer_subject_sha256: Option<[u8; 32]>,  // Some only for a Trusted signature
    pub final_path: &'a str,
    pub file_sha256: Option<[u8; 32]>,            // only for Unsigned images
    pub class: ProgramClass,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BindingChange { SignerChanged, FileChanged }

pub fn fold_name(s: &str) -> String;
pub fn paths_equal(a: &str, b: &str) -> bool;
pub fn classify_program(exe_folded: &str, package_family: Option<&str>) -> ProgramClass;
pub fn program_fill_allowed(saved: &LoginUrl, observed: &ProgramIdentity<'_>) -> Result<MatchKind, Refusal>;
pub fn binding_change(saved: &LoginUrl, observed: &ProgramIdentity<'_>) -> Option<BindingChange>;
pub fn program_lookalike_of<'a>(exe_folded: &str, bound_exes: impl IntoIterator<Item = &'a str>) -> Option<&'a str>;
```

**Canonical forms (decided here, recorded in ADR-0158):**
- `winapp://package/<family>`: `<Name>_<PublisherId>`, ASCII lowercase; Name 3 to 50 of `[a-z0-9.-]`; PublisherId exactly 13 of the Crockford base32 lowercase alphabet `0-9a-hjkmnp-tv-z`.
- `winapp://signed/<exe>?signer=<43 base64url>`: `<exe>` is the folded file name, percent-encoded: bytes in `[a-z0-9-._~]` stay, every other byte is `%XX` with uppercase hex; the decoded name is 1 to 255 bytes of UTF-8, equals `fold_name` of itself, is not `.` or `..`, and has no `\ / : * ? " < > |`, no control character and no NUL. `parse` decodes and re-encodes, and refuses anything that does not round-trip.
- `winapp://path/<base64url(UTF-8 path)>?sha256=<43 base64url>`: the decoded path is 1 to 4096 bytes of UTF-8, starts with a drive (`[A-Za-z]:\`) or UNC (`\\server\share\`) prefix, never with `\\?\` or `\\.\`, and has no `/`, NUL or control character. Stored as captured (not folded).
- `signer` and `sha256`: 43 base64url characters without padding that decode to 32 bytes and re-encode to the same text.
- Exactly one query member where the form has one, none otherwise; no fragment; no other member; scheme `winapp://` in lowercase.
- `fold_name`: per `char`, `c.to_lowercase()` when that yields exactly one `char`, else `c` (a simple lowercase mapping, deterministic on every OS). `paths_equal(a, b)` = `fold_name(a) == fold_name(b)`.

**Exclusion classes** (`classify_program`, folded names; package families in lowercase):
- `Browser`: `chrome.exe`, `chromium.exe`, `msedge.exe`, `msedgewebview2.exe`, `firefox.exe`, `brave.exe`, `opera.exe`, `opera_gx.exe`, `vivaldi.exe`, `iexplore.exe`, `librewolf.exe`, `waterfox.exe`, `floorp.exe`, `zen.exe`, `thorium.exe`, `arc.exe`, `whale.exe`, `yandex.exe`, `browser.exe`.
- `Host`: `java.exe`, `javaw.exe`, `python.exe`, `pythonw.exe`, `py.exe`, `pyw.exe`, `node.exe`, `deno.exe`, `bun.exe`, `electron.exe`, `powershell.exe`, `powershell_ise.exe`, `pwsh.exe`, `cmd.exe`, `conhost.exe`, `openconsole.exe`, `windowsterminal.exe`, `wt.exe`, `wscript.exe`, `cscript.exe`, `mshta.exe`, `rundll32.exe`, `regsvr32.exe`, `dllhost.exe`, `wsl.exe`, `wslhost.exe`, `bash.exe`, `ruby.exe`, `perl.exe`, `php.exe`; packages `microsoft.windowsterminal_8wekyb3d8bbwe`, `microsoft.windowsterminalpreview_8wekyb3d8bbwe`.
- `Mosaic`: `mosaic.exe`, `mosaic-desktop.exe`, `mosaic-native-host.exe`.
- `Broker`: `credentialuibroker.exe`, `consent.exe`, `logonui.exe`, `lockapp.exe`, `applicationframehost.exe`.
- `Remote`: `mstsc.exe`, `msrdc.exe`, `msrdcw.exe`, `vmconnect.exe`, `vmware.exe`, `vmware-vmx.exe`, `vmplayer.exe`, `virtualbox.exe`, `virtualboxvm.exe`, `vncviewer.exe`, `tvnviewer.exe`, `teamviewer.exe`, `anydesk.exe`, `rustdesk.exe`, `parsecd.exe`, `remote-viewer.exe`; package `microsoft.remotedesktop_8wekyb3d8bbwe`.
- By name for any signer (stricter than "by signer and name"; ADR-0158 says so). Everything else is `Ordinary`.

**Matching:** any `class` other than `Ordinary` → `Err(Refusal::NoMatch)`. `package/` matches only `Some(equal family)`. `signed/` matches only `signer_subject_sha256 == Some(signer)` **and** `exe_name == exe`. `path/` matches only `signer_subject_sha256 == None`, `file_sha256 == Some(sha256)` and `paths_equal`. A non-`winapp://` URL → `NoMatch`. A match is `MatchKind::ExactOrigin`. `binding_change`: `SignerChanged` when a `signed/` URL's exe equals and the observed signer is `Some` and different; `FileChanged` when a `path/` URL's path is equal and the observed hash is `Some` and different; else `None`. `program_lookalike_of`: the stems (names without a trailing `.exe`) of `exe_folded` and of a bound exe differ, both have at least 4 chars, and `osa_distance_is_one` holds (one substituted homoglyph such as a Cyrillic `о` is one substitution); the result is the lexicographically first such bound exe.

- [ ] **Step 1: Write ADR-0158 and the §4.10 threat rows**

ADR-0158 records: the three canonical forms and their rules above; the fold rule; the exclusion classes (by name, any signer); D4's signature states (only `Trusted` and `Unsigned` bindable); Q3 decided as path **plus** hash; Q2 decided as "refuse WebView2 and any foreign-process content in phase 1" (`ForeignContent`, Task 3); display data in the URL element's `extra.program` is never read by the matcher; unknown older builds keep `winapp://` URLs as unknown URLs that never match pages. THREAT_MODEL new "### 4.10 Desktop program fill (Windows)" table with the spec §6.2 Part 1 rows for: look-alike program; unsigned program replaced (path plus hash); browser, host, broker or remote client posing as a program (each mitigation names ADR-0158).

- [ ] **Step 2: Write the failing tests** in `crates/origin/src/program_tests.rs`

```rust
const SIGNER: [u8; 32] = [0x11; 32];
const OTHER: [u8; 32] = [0x22; 32];
const HASH: [u8; 32] = [0x33; 32];
fn id<'a>(exe: &'a str, path: &'a str, signer: Option<[u8; 32]>, hash: Option<[u8; 32]>) -> ProgramIdentity<'a> {
    ProgramIdentity { package_family: None, exe_name: exe, signer_subject_sha256: signer,
        final_path: path, file_sha256: hash, class: ProgramClass::Ordinary }
}
fn saved(u: &ProgramUrl) -> LoginUrl { LoginUrl::new(&u.canonical()) }

#[test] fn program_url_round_trips() {
    for u in [ProgramUrl::package("Microsoft.WindowsNotepad_8wekyb3d8bbwe").unwrap(),
              ProgramUrl::signed("Discord.exe", SIGNER).unwrap(),
              ProgramUrl::path(r"C:\Users\a\AppData\Local\Tool\tool.exe", HASH).unwrap()] {
        assert_eq!(ProgramUrl::parse(&u.canonical()).unwrap(), u);
    }
    assert_eq!(ProgramUrl::package("Microsoft.WindowsNotepad_8wekyb3d8bbwe").unwrap().canonical(),
               "winapp://package/microsoft.windowsnotepad_8wekyb3d8bbwe");
    assert!(ProgramUrl::signed("Discord.exe", SIGNER).unwrap().canonical()
        .starts_with("winapp://signed/discord.exe?signer="));
}
#[test] fn program_url_parse_refuses_non_canonical_forms() {
    let good = ProgramUrl::signed("tool.exe", SIGNER).unwrap().canonical();
    for bad in [good.replace("winapp://", "WINAPP://"), format!("{good}&x=1"), format!("{good}#f"),
                good.replace("tool.exe", "Tool.exe"), good.replace("tool.exe", "tool%2eexe"),
                good.replace("?signer=", "?sha256="), format!("{good}="), "winapp://signed/?signer=".into(),
                "winapp://package/ab_8wekyb3d8bbwe".into(), "winapp://package/name_8WEKYB3D8BBWE".into(),
                "winapp://other/x".into(), "androidapp://com.example".into()] {
        assert!(ProgramUrl::parse(&bad).is_err(), "{bad}");
    }
}
#[test] fn exe_names_fold_case_but_not_script() {
    assert_eq!(fold_name("DISCORD.EXE"), "discord.exe");
    assert_eq!(ProgramUrl::signed("Discord.Exe", SIGNER), ProgramUrl::signed("discord.exe", SIGNER));
    assert_ne!(fold_name("disc\u{043e}rd.exe"), "discord.exe"); // Cyrillic o stays
}
#[test] fn path_urls_accept_only_final_drive_or_unc_paths() {
    assert!(ProgramUrl::path(r"\\server\share\t.exe", HASH).is_ok());
    for p in [r"\\?\C:\t.exe", r"\\.\C:\t.exe", "C:/t.exe", "t.exe", r"C:t.exe", ""] {
        assert!(ProgramUrl::path(p, HASH).is_err(), "{p}");
    }
    assert!(paths_equal(r"C:\Tools\T.exe", r"c:\tools\t.EXE"));
}
#[test] fn signed_binding_needs_equal_signer_and_exe_name() { /* exact match Ok(ExactOrigin) */ }
#[test] fn same_exe_other_signer_does_not_match() { /* signer OTHER -> Err(NoMatch); binding_change == Some(SignerChanged) */ }
#[test] fn same_signer_other_exe_does_not_match() { /* "discord2.exe" -> Err(NoMatch); binding_change == None */ }
#[test] fn unsigned_binding_needs_equal_path_and_hash() { /* other hash -> NoMatch + FileChanged; other path -> NoMatch */ }
#[test] fn signed_image_never_matches_a_path_binding() { /* same path and hash, signer Some -> NoMatch */ }
#[test] fn package_binding_needs_equal_family() { /* other family / None -> NoMatch */ }
#[test] fn browsers_hosts_brokers_and_remote_clients_never_match() {
    for (exe, class) in [("chrome.exe", ProgramClass::Browser), ("python.exe", ProgramClass::Host),
                         ("consent.exe", ProgramClass::Broker), ("mstsc.exe", ProgramClass::Remote),
                         ("mosaic-desktop.exe", ProgramClass::Mosaic)] {
        assert_eq!(classify_program(exe, None), class);
        let url = ProgramUrl::signed(exe, SIGNER).unwrap();
        let mut o = id(exe, r"C:\x.exe", Some(SIGNER), None);
        o.class = class;
        assert_eq!(program_fill_allowed(&saved(&url), &o), Err(Refusal::NoMatch), "{exe}");
    }
    assert_eq!(classify_program("x.exe", Some("microsoft.windowsterminal_8wekyb3d8bbwe")), ProgramClass::Host);
}
#[test] fn web_and_app_urls_never_match_a_program() { /* "https://discord.com", "androidapp://..." -> NoMatch */ }
```

Fill the commented bodies with the assertions their comments name.

- [ ] **Step 3: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-origin program`
Expected: compile errors (`program` module missing).

- [ ] **Step 4: Implement `crates/origin/src/program.rs`**

Follow `app.rs`'s layout (doc header naming ADR-0158, constants, error enum, URL type with `parse`/`canonical`, then the matcher). Exclusion lists as `const` slices, looked up after folding. No new dependency.

- [ ] **Step 5: Fuzz target**

`crates/origin/fuzz/fuzz_targets/program_url.rs`: any UTF-8 input never panics `ProgramUrl::parse`; an accepted URL satisfies `url.canonical() == text.trim()`; `program_fill_allowed` of the parsed URL against an identity with `class != Ordinary` is always `Err`. Seeds: one canonical URL of each form and one with an extra query member.

- [ ] **Step 6: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-origin`
Expected: PASS (new and existing). fmt and clippy for `mosaic-origin`: clean. The fuzz target runs in CI's fuzz-smoke job (`crates/origin/fuzz` is already in its matrix).

- [ ] **Step 7: Commit**

```bash
git add docs/DECISIONS.md docs/THREAT_MODEL.md crates/origin
git commit -m "feat(origin): winapp:// program identity URLs and matching (ADR-0158)"
```

---

### Task 3: `mosaic-program-fill` identity capture model (runs anywhere)

**Files:**
- Create: `crates/program-fill/Cargo.toml`, `crates/program-fill/src/lib.rs`, `crates/program-fill/src/identity.rs` (tests in a `#[cfg(test)] mod tests` at the end of the file)
- Modify: `CLAUDE.md` (Layout: "`crates/program-fill`: program identity capture, binding policy and re-verification for Windows program fill (ADR-0157, ADR-0158); `crates/winshim`: the only crate with `unsafe`, a safe minimal shim over Win32 (ADR-0157)")
- Modify: `docs/THREAT_MODEL.md` §4.10 (row "A medium-integrity Mosaic would gain power over elevated UI")

**Interfaces:**
- Consumes: Task 2 `ProgramClass`, `ProgramIdentity`, `classify_program`, `fold_name`.
- Produces (`mosaic_program_fill::identity`, re-exported at the crate root):

```rust
pub const MAX_UNSIGNED_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SignerDisplay { pub common_name: Option<String>, pub organization: Option<String> }

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum SignatureState { Trusted { subject_der: Vec<u8>, display: SignerDisplay }, Unsigned, Untrusted, Revoked }

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RawProcess { pub pid: u32, pub final_path: String, pub package_family: Option<String>, pub integrity_rid: u32 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OwnProcess { pub pid: u32, pub integrity_rid: u32 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IdentitySource { WindowOwner, ElementProcess }

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CapturedIdentity {
    pub pid: u32,
    pub final_path: String,
    pub exe_name: String,                       // folded
    pub package_family: Option<String>,         // folded
    pub signer_subject_sha256: Option<[u8; 32]>,
    pub signer_display: Option<SignerDisplay>,
    pub file_sha256: Option<[u8; 32]>,
    pub class: ProgramClass,
}
impl CapturedIdentity { pub fn as_identity(&self) -> ProgramIdentity<'_>; }

#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)] #[non_exhaustive]
pub enum CaptureRefusal { OwnProcess, Elevated, ForeignContent, Excluded(ProgramClass), InvalidSignature, RevokedSignature, TooLargeToBind, BadPath }

#[derive(Debug, thiserror::Error)]
pub enum HashError { #[error("too large")] TooLarge, #[error("io")] Io }

pub fn exe_name_of(final_path: &str) -> Option<String>;
pub fn choose_identity_process(element: &RawProcess, window_owner: &RawProcess) -> Result<IdentitySource, CaptureRefusal>;
pub fn assemble_identity(own: OwnProcess, process: &RawProcess, signature: SignatureState, file_sha256: Option<[u8; 32]>) -> Result<CapturedIdentity, CaptureRefusal>;
pub fn hash_image_capped(path: &std::path::Path, cap: u64) -> Result<[u8; 32], HashError>;
```

Rules: `choose_identity_process`: equal PIDs → `WindowOwner`; owner's `exe_name_of` is `applicationframehost.exe` and the element process has a package family → `ElementProcess`; else `ForeignContent` (WebView2 included, Q2). `assemble_identity`, in this order: `process.pid == own.pid` → `OwnProcess`; `process.integrity_rid > own.integrity_rid` → `Elevated`; `exe_name_of` fails → `BadPath`; class from `classify_program` not `Ordinary` → `Excluded(class)`; `Revoked` → `RevokedSignature`; `Untrusted` → `InvalidSignature`; `Trusted` → `signer_subject_sha256 = Some(sha256(subject_der))`, hash dropped; `Unsigned` without hash → `TooLargeToBind`. `hash_image_capped`: `metadata().len() > cap` → `TooLarge`; stream with a 64 KiB buffer into `sha2::Sha256`, and return `TooLarge` if more than `cap` bytes are read.

Cargo: `mosaic-origin`, `mosaic-core` (paths), `sha2`, `thiserror` (workspace); `[target.'cfg(windows)'.dependencies]` stays empty until Task 7; dev: `tempfile`, `proptest`. `[lints] workspace = true`; `#![forbid(unsafe_code)]`. No new external crate.

- [ ] **Step 1: Write the failing tests**

```rust
fn raw(pid: u32, path: &str, pfn: Option<&str>, rid: u32) -> RawProcess { /* … */ }
const OWN: OwnProcess = OwnProcess { pid: 10, integrity_rid: 0x2000 };

#[test] fn mosaic_never_fills_its_own_windows() {
    assert_eq!(assemble_identity(OWN, &raw(10, r"C:\a\x.exe", None, 0x2000), SignatureState::Unsigned, Some([0; 32])),
               Err(CaptureRefusal::OwnProcess));
    assert_eq!(assemble_identity(OWN, &raw(11, r"C:\Program Files\Mosaic\mosaic-desktop.exe", None, 0x2000),
               trusted(), None), Err(CaptureRefusal::Excluded(ProgramClass::Mosaic)));
}
#[test] fn elevated_target_gets_no_chip_and_no_fill() { /* rid 0x3000 -> Elevated */ }
#[test] fn foreign_process_content_is_refused() { /* element pid != owner pid, owner notepad.exe -> ForeignContent; owner msedgewebview2 child -> ForeignContent */ }
#[test] fn uwp_content_takes_the_package_of_the_element_process() { /* owner ApplicationFrameHost.exe, element pfn Some -> ElementProcess; element pfn None -> ForeignContent */ }
#[test] fn only_trusted_and_unsigned_images_are_bindable() { /* Revoked -> RevokedSignature, Untrusted -> InvalidSignature */ }
#[test] fn signed_identity_hashes_the_subject_and_drops_the_file_hash() {
    // Trusted { subject_der: b"subject".to_vec(), .. } with file hash Some([1;32]):
    // signer_subject_sha256 == Some(Sha256::digest(b"subject")), file_sha256 == None, exe_name folded
}
#[test] fn unsigned_image_over_the_cap_cannot_be_bound() {
    // tempfile of 10 bytes: hash_image_capped(path, 9) is Err(TooLarge); cap 10 is Ok(sha256 of the bytes)
    // assemble_identity(.., Unsigned, None) == Err(TooLargeToBind)
}
#[test] fn exe_name_of_takes_the_last_component() { /* r"C:\A\Discord.EXE" -> "discord.exe"; r"C:\" -> None; "" -> None */ }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill identity`
Expected: compile errors.

- [ ] **Step 3: Implement** `crates/program-fill` and `identity.rs` per the Interfaces and Rules above. Crate doc names ADR-0157/0158 and says phase 1 writes nothing into programs.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill`
Expected: PASS. fmt, clippy for `mosaic-program-fill`: clean. `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target cargo deny check` passes (new workspace member, no new external crate).

- [ ] **Step 5: Commit**

```bash
git add crates/program-fill CLAUDE.md docs/THREAT_MODEL.md Cargo.lock
git commit -m "feat(program-fill): program identity capture model (ADR-0158)"
```

---

### Task 4: Binding policy: signed, packaged and unsigned programs (runs anywhere)

**Files:**
- Create: `crates/program-fill/src/policy.rs` (tests at the end); Modify: `crates/program-fill/src/lib.rs`
- Modify: `docs/THREAT_MODEL.md` §4.10 (the "unsigned program at a user-writable path is replaced" row gains "two separate confirmations at least 1 s apart, ADR-0158")

**Interfaces:**
- Consumes: Task 2 `ProgramUrl`, `binding_change`, `program_lookalike_of`, `BindingChange`; Task 3 `CapturedIdentity`.
- Produces (`mosaic_program_fill::policy`):

```rust
pub const SECOND_CONFIRM_MIN_GAP_MS: u64 = 1_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Confirmations { One, Two }
#[derive(Clone, PartialEq, Eq, Debug)] pub enum LinkWarning { Unsigned, LookalikeExe { similar_to: String } }
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProposedBinding { pub url: ProgramUrl, pub confirmations: Confirmations, pub warnings: Vec<LinkWarning> }
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)] #[non_exhaustive]
pub enum BindRefusal { Excluded(ProgramClass), BadIdentity }

pub fn propose_binding(identity: &CapturedIdentity, bound_exes: &[&str]) -> Result<ProposedBinding, BindRefusal>;
pub fn link_again_needed(saved: &[LoginUrl], identity: &CapturedIdentity) -> Option<BindingChange>;

pub struct LinkConfirmation { /* proposed, first_at_ms: Option<u64> */ }
#[derive(Clone, PartialEq, Eq, Debug)] pub enum ConfirmStep { NeedAnother, TooSoon, Done(LoginUrl) }
impl LinkConfirmation {
    pub fn new(proposed: ProposedBinding) -> Self;
    pub fn confirm(&mut self, now_ms: u64) -> ConfirmStep;
}
```

Rules: class not `Ordinary` → `Excluded`. Package family present → `Package`, `One`. Else signer `Some` → `Signed`, `One`. Else `file_sha256` `Some` → `Path`, `Two`, warning `Unsigned` first. Else `BadIdentity`. `LookalikeExe` from `program_lookalike_of(&identity.exe_name, bound_exes)` (the caller passes the exe names of the user's other `signed/` and `path/` bindings, never this identity's own). `LinkConfirmation::confirm`: `One` → `Done` at the first call; `Two` → first call `NeedAnother` (records the time); a second call earlier than `SECOND_CONFIRM_MIN_GAP_MS` after the first → `TooSoon` (and the first stays recorded); at or after the gap → `Done(LoginUrl::new(&url.canonical()))`. A `now_ms` below the first time counts as `TooSoon`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn packaged_and_signed_programs_need_one_confirmation() { /* Package + One; Signed + One, no Unsigned warning */ }
#[test] fn unsigned_program_binds_path_and_hash_with_two_confirmations() {
    // url == ProgramUrl::path(final_path, hash); confirmations == Two; warnings[0] == Unsigned
}
#[test] fn double_click_does_not_link_an_unsigned_program() {
    // confirm(1_000) == NeedAnother; confirm(1_200) == TooSoon; confirm(2_000) == Done(url)
    // and confirm(500) after the first (clock back) == TooSoon
}
#[test] fn lookalike_exe_is_a_warning_never_a_match() {
    // identity "dlscord.exe" with bound_exes ["discord.exe"]: warnings contain LookalikeExe { similar_to: "discord.exe" };
    // and mosaic_origin::program::program_fill_allowed of the discord binding vs this identity is Err(NoMatch)
}
#[test] fn excluded_programs_cannot_be_bound() { /* class Browser -> Err(Excluded(Browser)) */ }
#[test] fn changed_signer_or_file_asks_to_link_again() { /* link_again_needed -> SignerChanged / FileChanged / None */ }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill policy`
Expected: compile errors.

- [ ] **Step 3: Implement `policy.rs`** per the Interfaces and Rules.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill`
Expected: PASS; fmt and clippy clean.

- [ ] **Step 5: Commit**

```bash
git add crates/program-fill docs/THREAT_MODEL.md
git commit -m "feat(program-fill): binding policy with two confirmations for unsigned programs (ADR-0158)"
```

---

### Task 5: Re-verification before every write (TOCTOU, UIPI) (runs anywhere)

**Files:**
- Create: `crates/program-fill/src/verify.rs`, `crates/program-fill/src/fake.rs` (behind `#[cfg(any(test, feature = "fake"))]`); Modify: `crates/program-fill/src/lib.rs`, `crates/program-fill/Cargo.toml` (`[features] fake = []`)
- Modify: `docs/THREAT_MODEL.md` §4.10 (rows: target swaps window/focus/field or PID reuse; overlay window; each naming spec §5.1.2 and this module)

**Interfaces:**
- Consumes: Task 3 `CapturedIdentity` (its `final_path` seeds the session); Task 2 `mosaic_origin::program::paths_equal`.
- Produces (`mosaic_program_fill::verify`):

```rust
pub const PICK_TTL_MS: u64 = 30_000;
pub const CONTINUATION_TTL_MS: u64 = 60_000;
pub const MIN_FIELD_PX: i32 = 4;

#[derive(Clone, PartialEq, Eq, Debug)] pub struct RuntimeId(pub Vec<i32>);
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct Rect { pub left: i32, pub top: i32, pub right: i32, pub bottom: i32 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub struct Point { pub x: i32, pub y: i32 }
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum FieldKind { Password, Text }
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldFacts { pub runtime_id: RuntimeId, pub pid: u32, pub kind: FieldKind, pub enabled: bool, pub offscreen: bool, pub rect: Rect }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowFacts { pub root: isize, pub pid: u32, pub cloaked: bool, pub minimized: bool }
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Snapshot {
    pub focused: Option<FieldFacts>,  // None: no focused Edit
    pub focused_is_target: bool,      // CompareElements(focused, target)
    pub process_running: bool,        // through the held handle
    pub image_final_path: String,     // re-read through the held handle
    pub foreground: Option<WindowFacts>,
    pub occlusion_clear: [bool; 5],   // probe_points(target rect): target or a descendant at each point
}
pub struct PickSession<H> {
    pub held: H,                      // keeps the PID from being reused
    pub element_pid: u32, pub window_pid: u32, pub root: isize,
    pub target: RuntimeId, pub target_kind: FieldKind,
    pub final_path: String, pub picked_at_ms: u64, pub continuation: bool,
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum WriteKind { Username, Password, Submit }
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)] #[non_exhaustive]
pub enum VerifyFailure { Expired, ProcessGone, FocusMoved, WindowChanged, PathChanged, FieldKindChanged, FieldUnusable, Occluded, ProbeFailed }
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)]
pub enum ProbeError { Timeout, Gone, Os }

pub trait ProgramProbe { type Held; fn snapshot(&self, session: &PickSession<Self::Held>) -> Result<Snapshot, ProbeError>; }

impl<H> PickSession<H> { pub fn new(held: H, identity: &CapturedIdentity, element_pid: u32, window_pid: u32, root: isize, target: RuntimeId, target_kind: FieldKind, picked_at_ms: u64) -> Self; }
pub fn probe_points(rect: Rect) -> Option<[Point; 5]>;
pub fn verify_before_write<H>(session: &PickSession<H>, snap: &Snapshot, write: WriteKind, now_ms: u64) -> Result<(), VerifyFailure>;
pub fn check<P: ProgramProbe>(probe: &P, session: &PickSession<P::Held>, write: WriteKind, now_ms: u64) -> Result<(), VerifyFailure>;
```

`fake.rs`: `pub struct FakeProbe { pub snapshot: std::cell::RefCell<Result<Snapshot, ProbeError>> }` with `type Held = FakeProcess` (`pub struct FakeProcess { pub alive: std::rc::Rc<std::cell::Cell<bool>> }`); its `snapshot` returns the stored value with `process_running` forced to `alive.get()`. `pub fn good_snapshot(session) -> Snapshot` builds the all-pass snapshot for a session.

**Order of checks** in `verify_before_write` (first failure wins):
1. `Expired`: `now_ms < picked_at_ms` or `now_ms - picked_at_ms >= ttl` (`ttl` = `CONTINUATION_TTL_MS` if `continuation`, else `PICK_TTL_MS`).
2. `ProcessGone`: `!snap.process_running`.
3. `FocusMoved`: `focused` is `None`, `!focused_is_target`, `focused.runtime_id != target`, or `focused.pid != element_pid`.
4. `WindowChanged`: `foreground` is `None`, or its `root != session.root`, `pid != window_pid`, `cloaked` or `minimized`.
5. `PathChanged`: `!paths_equal(&snap.image_final_path, &session.final_path)` (Task 2's `paths_equal`).
6. `FieldKindChanged`: `Password` and `Submit` writes need `focused.kind == Password` and `target_kind == Password`; `Username` needs `Text` for both.
7. `FieldUnusable`: `!enabled`, `offscreen`, or `probe_points(rect)` is `None`.
8. `Occluded`: any `occlusion_clear` is `false`.
`check` maps `ProbeError` to `ProbeFailed`. `probe_points`: width and height computed in `i64`; `None` if either is below `MIN_FIELD_PX`; else centre and the four points at a quarter of the width and height in from each corner, all in `i32` range.

- [ ] **Step 1: Write the failing tests** (in `verify.rs`, using `FakeProbe` and `good_snapshot`)

```rust
#[test] fn a_good_snapshot_passes() { /* check(..., Password, picked_at + 1_000) == Ok(()) */ }
#[test] fn focus_change_between_pick_and_write_writes_nothing() { /* focused_is_target = false -> FocusMoved; focused None -> FocusMoved */ }
#[test] fn window_swap_or_foreground_change_aborts_typing() { /* other root / other pid / cloaked / minimized -> WindowChanged */ }
#[test] fn pid_reuse_cannot_take_over_a_session() {
    // alive.set(false) while a new process got the same pid in the snapshot facts -> ProcessGone
    // and a changed image path -> PathChanged
}
#[test] fn occluding_window_blocks_the_fill() { /* occlusion_clear[2] = false -> Occluded */ }
#[test] fn password_never_written_into_a_non_password_field() {
    // target_kind Password but focused.kind Text -> FieldKindChanged for Password and Submit; Username on Password field -> FieldKindChanged
}
#[test] fn disabled_offscreen_or_tiny_fields_are_unusable() { /* -> FieldUnusable */ }
#[test] fn a_session_from_the_future_or_long_ago_is_expired() {
    // now < picked_at -> Expired; now = picked_at + PICK_TTL_MS -> Expired; continuation at +59_999 -> Ok, +60_000 -> Expired
    // now = u64::MAX with picked_at 0 -> Expired (no overflow)
}
#[test] fn probe_points_refuse_tiny_or_inverted_rects() { /* 3x10, 10x3, inverted, minimised -32000 window with 0 size -> None */ }
#[test] fn probe_points_do_not_overflow() { /* Rect { left: i32::MIN, top: i32::MIN, right: i32::MAX, bottom: i32::MAX } -> Some, all points within the rect */ }
#[test] fn probe_errors_fail_closed() { /* FakeProbe Err(Timeout) -> ProbeFailed */ }
```

Add a `proptest`: for any `Rect`, `probe_points` returns `None` or five points inside the rect.

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill verify`
Expected: compile errors.

- [ ] **Step 3: Implement `verify.rs` and `fake.rs`** per the Interfaces and the order above.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill`
Expected: PASS; fmt and clippy clean.

- [ ] **Step 5: Commit**

```bash
git add crates/program-fill docs/THREAT_MODEL.md
git commit -m "feat(program-fill): re-verification before every write (spec 5.1.2)"
```

---

### Task 6: `mosaic-winshim`, the isolated shim crate (Windows machine to test; compiles empty elsewhere)

Build exactly what ADR-0157 D1/D2 assign to the shim. If a vetted crate covers a group below, leave that group out.

**Files:**
- Create: `crates/winshim/Cargo.toml`, `crates/winshim/src/lib.rs`, `crates/winshim/src/sys_process.rs`, `crates/winshim/src/sys_trust.rs`, `crates/winshim/src/sys_window.rs`, `crates/winshim/src/sys_uia.rs` (only if D1 gives C-UIA-READ to the shim)
- Create: `scripts/check-unsafe-confined.sh`; Modify: `.github/workflows/ci.yml` (run it in the existing lint job)
- Modify: `docs/DEPENDENCIES.md` (`windows` row: features added for `winshim`)

**Interfaces:**
- Consumes: ADR-0157 D2.
- Produces (**Proposed shim API**, all in `mosaic_winshim`, `#[cfg(windows)]`; the crate is empty on other targets):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Debug, thiserror::Error)] #[non_exhaustive]
pub enum ShimError { AccessDenied, NotFound, Timeout, Os(i32) }
pub type Result<T> = core::result::Result<T, ShimError>;

/// An open process handle (PROCESS_QUERY_LIMITED_INFORMATION | SYNCHRONIZE); closed on Drop. Not Clone.
pub struct ProcessHandle { /* private */ }
impl ProcessHandle {
    pub fn open(pid: u32) -> Result<Self>;
    pub fn pid(&self) -> u32;
    pub fn is_running(&self) -> Result<bool>;
    pub fn image_path(&self) -> Result<String>;            // QueryFullProcessImageNameW, Win32 form
    pub fn package_family_name(&self) -> Result<Option<String>>; // APPMODEL_ERROR_NO_PACKAGE -> None
    pub fn integrity_rid(&self) -> Result<u32>;            // TokenIntegrityLevel RID
}
pub fn current_integrity_rid() -> Result<u32>;
pub fn final_path(path: &str) -> Result<String>;           // GetFinalPathNameByHandleW(VOLUME_NAME_DOS); "\\?\" removed, "\\?\UNC\" -> "\\"

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Signature { Trusted { subject_der: Vec<u8>, common_name: Option<String>, organization: Option<String> }, Unsigned, Untrusted, Revoked }
pub fn verify_authenticode(path: &str) -> Result<Signature>; // embedded, then catalog; D4 mapping

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WindowFacts { pub root: isize, pub owner_pid: u32, pub cloaked: bool, pub minimized: bool }
pub fn foreground_window() -> Option<isize>;
pub fn window_facts(hwnd: isize) -> Result<WindowFacts>;    // GA_ROOT of hwnd, owner of the root

// Only if D1 gives C-UIA-READ to the shim. Uia is !Send (COM MTA on the creating thread).
pub struct Uia { /* private */ }
pub struct UiaElement { /* private: element + cached facts */ }
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ElementFacts { pub control_type: i32, pub is_password: bool, pub enabled: bool, pub offscreen: bool,
    pub keyboard_focusable: bool, pub rect: [i32; 4], pub pid: u32, pub native_window: isize,
    pub runtime_id: Vec<i32>, pub framework_id: String, pub has_value_pattern: bool, pub value_read_only: bool }
impl Uia {
    pub fn new(connection_timeout_ms: u32, transaction_timeout_ms: u32) -> Result<Self>;
    pub fn focused(&self) -> Result<Option<UiaElement>>;
    pub fn element_from_point(&self, x: i32, y: i32) -> Result<Option<UiaElement>>;
    pub fn same(&self, a: &UiaElement, b: &UiaElement) -> Result<bool>;          // CompareElements
    pub fn is_within(&self, e: &UiaElement, ancestor: &UiaElement) -> Result<bool>; // same or descendant, ≤ 64 parent steps
    pub fn nearest_window(&self, e: &UiaElement) -> Result<isize>;               // first ancestor-or-self with a NativeWindowHandle
}
impl UiaElement { pub fn facts(&self) -> &ElementFacts; }
```

- [ ] **Step 1: Crate skeleton and lints**

`Cargo.toml`: no `[lints] workspace = true`; instead copy every entry of the root `[workspace.lints.rust]` and `[workspace.lints.clippy]` verbatim, except `unsafe_code = "deny"`, and add `unsafe_op_in_unsafe_fn = "deny"`, clippy `undocumented_unsafe_blocks = "deny"`, `multiple_unsafe_ops_per_block = "deny"`, `missing_safety_doc = "deny"`. Each `sys_*` module starts with `#![expect(unsafe_code, reason = "Win32 calls, ADR-0157")]`; `lib.rs` has none. Dependencies under `[target.'cfg(windows)'.dependencies]`: `windows = { workspace = true, features = [<D2 list>] }`, `thiserror`. Commit message line: "dependency: windows (existing =0.62.2) gains Win32 features for mosaic-winshim, ADR-0157".

- [ ] **Step 2: Write the failing tests** (`#[cfg(all(test, windows))]` in `lib.rs`)

```rust
#[test] fn own_process_identity() {
    let h = ProcessHandle::open(std::process::id()).unwrap();
    assert!(h.is_running().unwrap());
    let exe = std::env::current_exe().unwrap();
    assert!(final_path(&h.image_path().unwrap()).unwrap().eq_ignore_ascii_case(&final_path(exe.to_str().unwrap()).unwrap()));
    assert_eq!(h.package_family_name().unwrap(), None);
    assert_eq!(h.integrity_rid().unwrap(), current_integrity_rid().unwrap());
}
#[test] fn bad_pids_are_errors() { assert!(ProcessHandle::open(0).is_err()); assert!(ProcessHandle::open(u32::MAX).is_err()); }
#[test] fn final_path_strips_the_device_prefix() { /* final_path(r"C:\Windows\System32\..\System32\notepad.exe") starts with "C:\" and has no "\\?\" */ }
#[test] fn a_system_binary_is_trusted_and_names_microsoft() {
    // verify_authenticode(r"C:\Windows\System32\kernel32.dll") is Trusted with organization containing "Microsoft"
}
#[test] fn a_test_binary_is_unsigned() { /* current_exe -> Unsigned */ }
#[test] fn a_tampered_signed_file_is_untrusted() {
    // copy kernel32.dll to a tempdir, flip one byte at len/2, verify -> Untrusted
}
#[test] fn window_facts_of_nothing_is_an_error() { assert!(window_facts(0).is_err()); }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn uia_reads_the_focused_element() { /* Uia::new(2000, 2000).focused() is Some */ }
#[test] fn public_api_has_no_unsafe_fn_and_no_windows_types() {
    // include_str! of lib.rs: no "pub unsafe fn", no "windows::" in any `pub` line (split the keyword like ffi_crate_has_no_logic)
}
```

- [ ] **Step 3: Run them to verify they fail**

Run (Windows): `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-winshim`
Expected: compile errors.

- [ ] **Step 4: Implement**

One `unsafe` call per block with `// SAFETY:` stating the pointer, length and lifetime invariants. Owned handles close in `Drop` (`CloseHandle`); `WinVerifyTrust` always gets its `WTD_STATEACTION_CLOSE`; catalog contexts are released on every path; buffers are sized from the API's reported length and grown at most once; `Uia` calls `CoInitializeEx(COINIT_MULTITHREADED)` and records whether it must `CoUninitialize` in `Drop`; `Uia` holds `PhantomData<*const ()>` so it is `!Send`. Strings from UTF-16 through `String::from_utf16` (an invalid one is `ShimError::Os(0)`, never lossy). Errors map `ERROR_ACCESS_DENIED` → `AccessDenied`, `ERROR_INVALID_PARAMETER`/`ERROR_NOT_FOUND`/`ERROR_FILE_NOT_FOUND` → `NotFound`, `UIA_E_TIMEOUT` → `Timeout`, others → `Os(code)`.

- [ ] **Step 5: Confinement check**

`scripts/check-unsafe-confined.sh`: fails if any `crates/*/Cargo.toml` or `apps/desktop/src-tauri/Cargo.toml` other than `crates/winshim/Cargo.toml` lacks `[lints]` + `workspace = true`, or if `git grep -n "expect(unsafe_code\|allow(unsafe_code"` matches outside `crates/winshim/src/sys_`. Add `- run: bash scripts/check-unsafe-confined.sh` to the lint job in `.github/workflows/ci.yml`.

- [ ] **Step 6: Run the tests and checks**

Run (Windows): `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-winshim`; `cargo clippy -p mosaic-winshim --all-targets -- -D warnings` (with the target-dir prefix); `bash scripts/check-unsafe-confined.sh`; `cargo deny check`; `cargo audit --deny warnings`.
Expected: all pass. On Linux/macOS the crate builds empty (CI proves it).

- [ ] **Step 7: Commit**

```bash
git add crates/winshim scripts/check-unsafe-confined.sh .github/workflows/ci.yml docs/DEPENDENCIES.md Cargo.lock
git commit -m "feat(winshim): isolated Win32 shim for program identity (ADR-0157)"
```

- [ ] **Step 8: Hand-off (orchestrator)**

Before this crate is merged: the Fable red-team review of `crates/winshim` (O2) and the crypto-reviewer's reading of every `unsafe` block, recorded in ADR-0157's review log. Fixes go back to this task's worktree.

---

### Task 7: Windows probe, harness and the manual checklist (Windows machine)

**Files:**
- Create: `crates/program-fill/src/windows/mod.rs`, `crates/program-fill/src/windows/uia.rs`; Modify: `crates/program-fill/src/lib.rs` (`#[cfg(windows)] pub mod windows;`), `crates/program-fill/Cargo.toml` (`[target.'cfg(windows)'.dependencies] mosaic-winshim = { path = "../winshim" }` and any ADR-0157 crate)
- Create: `crates/program-fill/examples/probe.rs`
- Create: `docs/PROGRAM_FILL.md`
- Modify: `docs/THREAT_MODEL.md` §4.10 (D row: hung or hostile UIA provider) and §5 residual rows (next free `R` numbers): same-user malware injecting into a signed process after the check or synthesising input (P5); an unsigned image renamed and replaced between capture and hash (the running image cannot be written, but the path can be re-pointed; the hash and the held-handle path re-read catch it at the next write)

**Interfaces:**
- Consumes: Task 3 (`RawProcess`, `OwnProcess`, `SignatureState`, `choose_identity_process`, `assemble_identity`, `hash_image_capped`, `MAX_UNSIGNED_IMAGE_BYTES`), Task 4 (`propose_binding`), Task 5 (`ProgramProbe`, `PickSession`, `Snapshot`, `FieldFacts`, `WindowFacts`, `probe_points`, `check`), Task 6 shim API, ADR-0157 D1 (UIA source) and D5 (timeouts 2000 ms).
- Produces:

```rust
// crates/program-fill/src/windows/mod.rs  (#[cfg(windows)])
pub struct WindowsProbe { /* UIA client, OwnProcess; !Send */ }
pub struct Captured { pub identity: CapturedIdentity, pub session: PickSession<mosaic_winshim::ProcessHandle> }
#[derive(Debug, thiserror::Error)] pub enum CaptureError { NotAField, Refused(CaptureRefusal), Probe(ProbeError) }
impl WindowsProbe {
    pub fn new() -> Result<Self, ProbeError>;
    /// Reads the focused Edit and its program now. `picked_at_ms` is the caller's monotonic clock.
    pub fn capture_focused(&self, picked_at_ms: u64) -> Result<Captured, CaptureError>;
}
impl ProgramProbe for WindowsProbe { type Held = mosaic_winshim::ProcessHandle; /* snapshot */ }
// crates/program-fill/src/windows/uia.rs: the one place that touches the UIA source of D1.
// `UiaElementRef` is a type alias for that source's element type (`mosaic_winshim::UiaElement`, or the vetted crate's).
pub(crate) fn focused_field(..) -> Result<Option<(UiaElementRef, FieldFacts, isize /* nearest window */)>, ProbeError>;
pub(crate) fn same(..) -> Result<bool, ProbeError>;
pub(crate) fn clear_at(.., target: &UiaElementRef, p: Point) -> Result<bool, ProbeError>;
```

`capture_focused`: focused element must be an Edit (`UIA_EditControlTypeId` = 50004), else `NotAField`; `FieldKind` from `IsPassword`; root and owner from `window_facts(nearest_window)`; open `ProcessHandle`s for the element PID and the owner PID; `RawProcess` for each (`final_path(image_path)`, `package_family_name` folded, `integrity_rid`); `choose_identity_process`; `verify_authenticode` of the chosen final path mapped to `SignatureState`; `hash_image_capped(.., MAX_UNSIGNED_IMAGE_BYTES)` only when `Unsigned` (`TooLarge` → `None`); `assemble_identity` with `OwnProcess { pid: std::process::id(), integrity_rid: current_integrity_rid()? }`; the session holds the chosen process's handle. `snapshot`: `focused_field`, `same` against the stored target, `held.is_running()`, `final_path(held.image_path())`, `foreground_window()` + `window_facts`, and `clear_at` for each of `probe_points(target rect)` (all `false` if `None`). No result is cached between snapshots.

- [ ] **Step 1: Write the tests**

`#[cfg(all(test, windows))]` in `windows/mod.rs`: `WindowsProbe::new()` succeeds; `capture_focused` without an interactive focused Edit returns `NotAField` or an error, never a panic. Interactive tests are `#[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]`.

- [ ] **Step 2: Implement `windows/` and the harness**

`examples/probe.rs` (`#![expect(clippy::print_stdout, reason = "manual Windows check harness")]`): waits 5 s (time to focus a field), runs `capture_focused`, prints the identity (exe, class, package family, signer CN/O, the first 8 bytes of the signer or file hash in hex, the path), the `propose_binding` result (URL, confirmations, warnings), then calls `check(.., WriteKind::Password, now)` every 500 ms for 30 s and prints each verdict.

- [ ] **Step 3: Write `docs/PROGRAM_FILL.md`**

Sections: "Phase 1 scope" (what exists, what waits for phase 2: picker, chip, hotkey, typing, submit, setting); "Crates" (the three modules and the shim rule, ADR-0157/0158); "Manual checklist (Windows, before each release; C3)" with these items, each run with `cargo run -p mosaic-program-fill --example probe` and recorded in PROGRESS:
- [ ] Notepad (packaged on Windows 11, Win32 on 10): package binding or signed binding naming Microsoft.
- [ ] A signed third-party program: `signed/` URL, one confirmation, CN/O shown.
- [ ] An unsigned copy renamed `Discord.exe` in `%TEMP%`: `path/` URL, two confirmations, `Unsigned` warning; change one byte and capture again: `link_again_needed` says `FileChanged`.
- [ ] A signed program re-signed with a second test certificate: `SignerChanged`.
- [ ] Chrome, Windows Terminal, `mstsc.exe`, the UAC prompt (not reachable): excluded or nothing captured.
- [ ] A program run as administrator: `Elevated`.
- [ ] Calculator (UWP): the identity is the app's package, not `ApplicationFrameHost.exe`.
- [ ] A WPF program (PowerShell ISE's own dialogs are excluded as `Host`; use another WPF program) and a WinForms program: `IsPassword` read correctly.
- [ ] During the 30 s check loop: click another window → `FocusMoved`/`WindowChanged`; drag a topmost window of another program over the field → `Occluded`; close the program → `ProcessGone`; minimise → `WindowChanged`.
- [ ] A program that moves its password field under the cursor on focus, and one that swaps its foreground window 100 ms after focus (spec §6.3 test programs, built in the temp dir): the loop reports a failure, never `Ok`, after the change.
- [ ] A hung program (the Task 1 Step 3 test window): the harness returns `ProbeFailed` within 5 s and stays responsive.

- [ ] **Step 4: Run**

Run (Windows): `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill`, clippy for `mosaic-program-fill` and `mosaic-winshim`, `bash scripts/check-unsafe-confined.sh`, `cargo deny check`. Then walk the manual checklist once with the harness and record the results in `docs/PROGRESS.md`.
Expected: tests pass; every checklist item behaves as written. A checklist failure is a bug in Tasks 3 to 7, fixed before hand-off.

- [ ] **Step 5: Commit**

```bash
git add crates/program-fill docs/PROGRAM_FILL.md docs/THREAT_MODEL.md docs/PROGRESS.md Cargo.lock
git commit -m "feat(program-fill): Windows probe, check harness and manual checklist (phase 1)"
```

- [ ] **Step 6: Phase 1 hand-off (orchestrator)**

Full gate once (`FULL_RUN_OK=1`), crypto-reviewer on Tasks 3 and 6 (hashing, signature handling), Fable red-team review of `crates/winshim` if not yet done, then the phase 2 plan (picker, fill, one-click submit, `programFill` setting under the settings MAC, focus events, typing) is written from ADR-0157's D1/D3 and the checklist results.

---

## Spec coverage (self-review)

| Spec item | Task |
|---|---|
| §5.1.1 forbid-unsafe rule and wrapper vetting; candidates; "if no candidate passes, not built" | 1 (with O2: shim instead of "not built") |
| §5.1.1 "Not verified": UIA timeouts, signer subject, Tauri non-focusable window | 1 Step 5 |
| §8 O2 (shim crate, `// SAFETY:`, safe minimal API, Fable review, tests, ADR naming crate and rule) | 1 (ADR-0157 D2), 6 |
| §8 C2 idle CPU and memory, opt-in rule | 1 Step 4, D3 |
| §8 C3 manual checklist, no self-hosted runner | 7 (`docs/PROGRAM_FILL.md`), Global Constraints |
| §5.1.2 the eight checks, held handle, UIPI and no `uiAccess`, excluded targets | 5, 3 (`Elevated`), 2 (classes) |
| §5.1.3 identity probe (final path, PFN, Authenticode, file hash with 512 MiB cap, integrity, class), which process, AFH and foreign content | 3, 6, 7 |
| §5.1.3 `winapp://` scheme, canonical-only parser, matching rules, look-alike warnings, signer-changed prompt | 2, 4 |
| §7 Q2 (refuse WebView2 in v1), Q3 (path plus hash) | 2 (ADR-0158), 3 |
| D3 / G2 unsigned: exact path plus explicit confirmation | 4 (two confirmations ≥ 1 s apart) |
| §6.2 Part 1 threat rows (look-alike, unsigned replaced, posing classes, TOCTOU, overlay, E, D, supply chain) | 1, 2, 3, 4, 5, 7 |
| §6.3 tests: `program_url_parse_refuses_non_canonical_forms` (+ fuzz `program_url`), `signed_binding_needs_equal_signer_and_exe_name`, `same_exe_other_signer_does_not_match`, `same_signer_other_exe_does_not_match`, `unsigned_binding_needs_equal_path_and_hash`, `signed_image_never_matches_a_path_binding`, `browsers_hosts_brokers_and_remote_clients_never_match`, `mosaic_never_fills_its_own_windows`, `focus_change_between_pick_and_write_writes_nothing`, `window_swap_or_foreground_change_aborts_typing`, `pid_reuse_cannot_take_over_a_session`, `occluding_window_blocks_the_fill`, `password_never_written_into_a_non_password_field`, `elevated_target_gets_no_chip_and_no_fill` | 2, 3, 5 |
| §6.3 `no_fill_without_a_pick`, picker tests, typing and submit | phase 2 (out of scope here) |
| §5.1.1 coverage matrix per framework, writing values, §5.1.4 picker, §5.2 submit | phase 2 |
