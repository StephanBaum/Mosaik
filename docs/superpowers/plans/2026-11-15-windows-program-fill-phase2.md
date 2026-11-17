# Windows Program Fill, Phase 2 (Picker, Fill, Submit, Linking) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a Windows program's login field gets focus, Mosaic offers the logins bound to that program in a small Mosaic window at the field; one pick (click or Enter) fills user name and password through the shim, re-verified before every write, and presses Enter to sign in; a program without a binding can be linked on the spot, with two confirmations for unsigned programs.

**Architecture:** Pure rules stay in `crates/program-fill` (field roles, placement, the fill sequence behind a `FillDriver` trait, a `Host` trait with a fake) and are host-tested on every OS. The Windows side (`crates/program-fill/src/windows/`) runs one MTA worker thread that owns the only UIA client, subscribes to focus events through `mosaic-winshim`, and executes captures and fills; the shim gains the event, write and input calls (ADR-0163). The desktop app adds a `program_fill` service (offer state machine, vault lookups, linking, block list) and two Rust-created Tauri windows, `picker-chip` (non-activating) and `picker` (the list), each with its own capability that grants only picker commands. The password goes from the vault (`SecretString`) to the worker over a Rust channel and into the shim's zeroizing `INPUT` buffer; no webview sees it.

**Tech Stack:** Rust 1.99 (edition 2024), `windows =0.62.2` (shim only, more Win32 features), `zeroize` (workspace), Tauri 2.12.5 (`WebviewWindowBuilder::focusable`, `content_protected`), React 19 + Vite multi-page, `packages/i18n`, `packages/design-tokens`, vitest.

**Spec:** `docs/superpowers/specs/2026-11-14-autofill-everywhere-design.md` §2.1 G1 to G4, §2.3 A1, A2, A8, §5.1.1 ("login field", writing values), §5.1.2, §5.1.4, §5.2.1 to §5.2.3 (programs), §6.2 Part 1 and Part 2 (programs), §6.3 program-fill tests, §7 Q1 to Q4, §8 C2, C3. Phase 1 plan: `docs/superpowers/plans/2026-11-14-windows-program-fill-phase1.md`. ADR-0157 and ADR-0158 (on `feat/program-fill`).

**Base:** branch `feat/program-fill` after phase 1 Task 7 (the shim and `WindowsProbe` exist, the manual phase 1 checklist passed). Same worktree `C:/Users/Stephan/Documents/DEV/Mosaic/.claude/worktrees/progfill`.


> **Owner decision 2026-11-15 (overrides ADR-0158 Q2 for WebView2): "also webview2 apps should work."**
> Content rendered by WebView2 (`msedgewebview2.exe`) for a host program counts as that host program's own
> content, so new Teams and other WebView2 apps get offers. Conditions, all required and all tested:
> (1) the focused element's process is `msedgewebview2.exe` and its WebView2 browser process was started by the
> host program (walk the parent chain from the renderer to the browser process to the host; the PIDs and start
> times are re-verified before each write, like the program itself); (2) the host program is signed or packaged
> (an unsigned host is refused, no two-confirmation path); (3) the host is not a browser (Edge, Chrome, Firefox,
> etc. stay refused; the extension covers them); (4) the binding stays exe plus signer of the **host**, never the
> WebView2 runtime. Record it as an amendment to ADR-0158 Q2 in Task 1 (ADR-0164 or the amendment section), widen
> R174 to "Electron, CEF and WebView2 programs that render remote web content receive the linked login when that
> content has focus", and add tests for: host signed and WebView2 child → offer; unsigned host → refused;
> browser host → refused; WebView2 process not descended from the focused host → refused; host replaced between
> offer and write → refused. Task 13's Teams check now expects a chip on the new Teams sign-in fields (the AAD
> broker window stays `Broker`).

## Global Constraints

- `#![forbid(unsafe_code)]` everywhere except `crates/winshim`; new shim code follows ADR-0157 D2 exactly (`unsafe` only in `sys*` modules, one operation per block, `// SAFETY:` on each, safe public API without `unsafe fn`, raw pointers, raw handles or `windows` types, a test per public function, `scripts/check-unsafe-confined.sh` passes).
- **No fill without a pick** (brief §4.1, A2). Focus events only ever show the chip. A write happens only inside `picker_choose`, `picker_link_confirm` (`linked`) or the two-step continuation of that same pick.
- **No secret in a webview** (A8). Picker windows get titles, user names, vault names, hosts and program display data. Passwords go vault → `SecretString` → `Zeroizing<Vec<u16>>` → the shim's zeroizing `INPUT` buffer. Never the clipboard.
- Submit is the Enter key only; Mosaic never looks for a button by caption (§5.2.2).
- Program identity and matching are phase 1's (`mosaic_origin::program`, `mosaic_program_fill::{identity, policy, verify}`); the display data in `extra.program` is never read by a matcher.
- Elevated targets are refused (no chip, notice on the hotkey). Mosaic never asks for `uiAccess` or elevation; if Mosaic itself runs above medium integrity (`current_integrity_rid() > 0x2000`), program fill does not start.
- `programFill` and `programFillSubmit` live under the settings MAC (ADR-0030); `programFill` is off by default (Q1); the hotkey has no default (Q4).
- UI strings only through `packages/i18n` (EN and DE, informal "du"); styling only through `packages/design-tokens`.
- Desktop surface tests are extended, never weakened (`remote_origins_and_other_webviews_get_no_ipc`, `the_build_has_no_devtools_remote_ipc_asset_protocol_or_webview_url_opener`, `capability_grants_exactly_the_commands`, `opener_has_one_call_site`).
- Library code returns `thiserror` results; no `unwrap`/`expect`/`panic`/`println!` outside tests. Secrets never in `Debug`, errors or logs.
- No new external crate. `zeroize` (workspace) joins `mosaic-winshim` and `mosaic-program-fill`; commit message line: "dependency: zeroize (workspace) in winshim/program-fill for the INPUT and UTF-16 buffers".
- Windows-only code behind `#[cfg(windows)]`; every crate builds on Linux and macOS (CI). Interactive tests are `#[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"]` (C3).
- ADR numbers: ADR-0163, ADR-0164 (0154 to 0162 are taken). Residual rows start at R171.
- **Runs anywhere:** Tasks 1 to 4 and 7 to 12. **Needs a Windows machine:** Tasks 5, 6, 13.
- Verification (Git Bash, worktree root), shared target dir, scoped:
  `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p <crate> <filter>`;
  `cargo fmt -p <crate> -- --config newline_style=Auto`;
  `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo clippy -p <crate> --all-targets -- -D warnings`;
  TypeScript: `pnpm -w exec vitest run --project desktop <path>`, `pnpm --filter @mosaic/i18n test`, `pnpm --filter @mosaic/desktop typecheck`, `pnpm --filter @mosaic/desktop lint`.
- Agent guard: write only inside the worktree and its `.agent-scope`; no searches outside the repo, `node_modules` or the cargo registry; at most 3 full-workspace runs per hour (`FULL_RUN_OK=1` for the final gate only); never `git worktree remove`.
- Conventional Commits. Scratch work in the system temp dir.

## Review Focus

1. **Multi-monitor and DPI** (a field on a monitor left of the primary, at negative coordinates, at 150 % next to a 100 % monitor; a field at the very bottom or right edge): the user expects the chip right of the field (left if no room) and the list under it (above if no room), on that monitor, never covering the field. Test `placement_on_a_left_monitor_with_negative_coordinates_and_mixed_dpi` (Task 3).
2. **Passwords with characters outside the BMP** (emoji, some CJK) and long ones (over 64 UTF-16 units): the user expects the exact password typed. Test `surrogate_pairs_are_never_split_across_batches` (Task 4).
3. **Modifiers still held after the pick** (Ctrl+Shift of the hotkey, AltGr on a German keyboard reported as Ctrl+Alt): the user expects Mosaic to wait, and if they keep holding, nothing typed and a clear notice. Test `held_modifiers_delay_then_abort_without_typing` (Task 4).
4. **A login with an empty user name or an empty password** (or a passkey-only login): the user expects the part that exists filled and no Enter on an empty password. Test `empty_password_fills_the_user_name_and_never_submits` (Task 4).
5. **An unsigned program updated, or a signed one re-signed by a new subject, then linked again**: the user expects one binding afterwards, not an old dead one next to the new one. Test `link_again_replaces_the_changed_binding` (Task 8).

---

## File Structure

| File | Change | Responsibility |
|---|---|---|
| `docs/DECISIONS.md` | modify | ADR-0163 (shim phase 2 API), ADR-0164 (picker, fill, submit, linking) |
| `docs/THREAT_MODEL.md` | modify | §4.10 phase 2 rows; residuals R171 to R176 |
| `docs/DESKTOP_IPC.md` | modify | picker section, program fill commands, settings fields, item URL view, errors |
| `docs/PROGRAM_FILL.md`, `docs/PROGRESS.md` | modify | phase 2 design notes, manual checklist on real programs, results |
| `crates/origin/src/program.rs`, `program_tests.rs` | modify | sign-in broker packages and `wwahost.exe` excluded |
| `crates/core/src/model/types.rs` | modify | `LoginUrl::extra` / `extra_mut` |
| `crates/program-fill/src/{binding,suggest,detect,placement,fill,host}.rs`, `verify.rs`, `fake.rs`, `lib.rs`, `Cargo.toml` | create / modify | pure phase 2 rules, fakes |
| `crates/winshim/src/{lib,sys_uia,sys_window}.rs`, `src/sys_input.rs`, `Cargo.toml` | modify / create | events, writes, input, window show |
| `crates/program-fill/src/windows/{host,driver}.rs`, `windows/mod.rs` | create / modify | the worker thread, `WindowsHost`, `WindowsDriver` |
| `apps/desktop/src-tauri/src/program_fill/{mod,offer,link,blocklist,picker_windows}.rs`, `src/program_fill_tests.rs` | create | the desktop service |
| `apps/desktop/src-tauri/src/contract/{picker,program_fill}.rs`, `contract/{settings,items,errors,mod}.rs` | create / modify | wire types |
| `apps/desktop/src-tauri/src/commands/{picker,program_fill}.rs`, `commands/{mod,names,settings,draft}.rs`, `build.rs` | create / modify | commands |
| `apps/desktop/src-tauri/capabilities/{picker,chip}.json`, `tauri.conf.json` | create / modify | window capabilities |
| `apps/desktop/src-tauri/src/{persist,shortcut,state,lib}.rs`, `settings_integrity_tests.rs`, `tests.rs`, `redteam_surface_tests.rs`, `fuzz/fuzz_targets/ipc_requests.rs` | modify | settings, hotkey, wiring, tests |
| `apps/desktop/{chip,picker}.html`, `vite.config.ts`, `src/picker/*`, `src/ipc/contract/{picker,programFill}.ts`, `src/ipc/mock/picker.ts`, `src/ipc/tauri.ts` | create / modify | picker frontend |
| `apps/desktop/src/screens/ProgramFillSettings.tsx`, `ProgramFillCard.tsx`, `screens/vault/DetailFields.tsx` | create / modify | main-window UI |
| `packages/i18n/src/catalog/{en,de}/programFill.ts`, `src/{en,de}.ts` | create / modify | strings |

---

### Task 1: ADR-0163, ADR-0164 and the threat rows (runs anywhere)

**Files:**
- Modify: `docs/DECISIONS.md` (append ADR-0163 and ADR-0164; check that no `## ADR-0163` exists on `main` or a pending branch first, else take the next free pair and use it everywhere this plan says 0163/0164)
- Modify: `docs/THREAT_MODEL.md` (§4.10 rows, §5 residuals R171 to R176; check R171 is free)

**Interfaces:**
- Consumes: ADR-0157 D1 (phase 2 sources open), D3, D5; ADR-0158; spec sections above.
- Produces: the decisions every later task builds. A later task that needs to deviate edits the ADR first.

- [ ] **Step 1: Write ADR-0163 "Windows program fill phase 2: shim scope for focus events, writes and input"**

Context: ADR-0157 D1 left C-UIA-EVENTS, C-UIA-WRITE and C-INPUT open and forbids two UIA clients. Decision:
- All three go to `mosaic-winshim` (the shim's UIA client already has the `IUIAutomation2` timeouts; `uiautomation` has none; `enigo`/`rdev` failed; `winsafe` copies `INPUT` into a non-zeroized `Vec`).
- The added public API is exactly Task 5's Interfaces block; the added `windows` features: `Win32_UI_Input_KeyboardAndMouse`, `Win32_System_Variant`; the event handler uses `windows::core::implement` (already in the tree through `windows-implement`, no new crate).
- `send_unicode` builds its `INPUT` records in a `Zeroizing<Vec<InputRecord>>` (exact capacity, never grown), at most 16 UTF-16 units per call; `set_value` and `value` refuse password elements; `show_no_activate` accepts only windows of Mosaic's own process; `set_foreground` accepts only top-level windows.
- Review log rows: crypto-reviewer reading of the new `unsafe` blocks, Fable red-team review of the changed crate before merge.

- [ ] **Step 2: Write ADR-0164 "Windows program fill phase 2: picker windows, fill, submit and linking"**

Decision items, each one line with its value:
- **Picker surface.** Two Tauri webview windows created from Rust only (`program_fill::picker_windows`): `picker-chip` (24 × 24 logical px, `focusable(false)`, shown through the shim's `show_no_activate`) and `picker` (the list, takes focus). Both: no decorations, not resizable, skip taskbar, always on top, `content_protected(true)`, served from the bundled `chip.html` / `picker.html`. Capabilities `chip` and `picker` grant only `CHIP_COMMANDS` / `LIST_COMMANDS`. Why not a native layered window: it would put a window procedure, text rendering, IME, DPI and accessibility into the `unsafe` shim, and lose i18n, design tokens and screen-reader support.
- **Not spoofable.** A target program cannot draw into Mosaic's windows. Before `picker_choose` and `picker_link_confirm` Rust checks: the generation is current, the item is in Rust's own offer (or the link step Rust opened), the list window is foreground, its outer rectangle equals the last placement (another process may move windows), and the five probe points of the list rectangle resolve through UIA to the list's own top-level window. Picks within 300 ms of the list appearing are ignored (a double click on the chip). Link confirmation keeps the 1 s guard of `prompts.rs`, and unsigned programs need the second confirmation of ADR-0158. Synthesised input into the picker is residual R171.
- **Detection.** One MTA worker owns the UIA client; focus events are debounced 150 ms; events of Mosaic's own process are ignored; an identity cache of 16 entries keyed by PID holds the process handle. The worker runs only while `programFill` is on and (the vault is unlocked, or it was unlocked once in this run so the setting is verified). The field rect is polled every 250 ms only while the chip is visible (no property-changed handler: one handler type less in the shim). Container for the user-name search: the ancestor 3 levels above the focused Edit; at most 64 Edits scanned. A lone text field is offered only when the program has a matching login and the whole window holds exactly one usable Edit.
- **Writing.** User name: `ValuePattern.SetValue` with read-back for frameworks `Win32`, `WinForm`, `WPF`, else typing. Password: always typed in phase 2 (no framework is proven for a password `SetValue`). Typing: modifier wait up to 2000 ms (poll 20 ms), batches of at most 8 UTF-16 units that never split a surrogate pair, a full `verify::check` before every batch, a focus event since the last `SetFocus` (after a 50 ms settle) aborts, a short `SendInput` count aborts; on abort, if the same field still verifies, Ctrl+A then Backspace.
- **Submit.** Enter, after `check(Submit)`, when the global `programFillSubmit` is on and no matched binding has `extra.submit = false`. Never on an empty password.
- **Two-step.** A pick on a lone text field fills the user name (and presses Enter when submit is on) and arms a continuation: a password field focused in the same root window, window PID and element PID within 60 s of the pick gets the password and Enter, once; the worker keeps the old process handle until the continuation ends. The code step (TOTP) is not in phase 2.
- **Linking.** Only Rust-captured identities become `winapp://` URLs: `UrlChange` add and update refuse `winapp://`. A link replaces the user's bindings that `binding_change` reports for this identity (no stale duplicates). Display data `extra.program = { name, signer }` (each at most 256 chars); per-binding `extra.submit = false` turns Enter off.
- **Block list.** "Don't offer for this program" stores `{ key, label }` in `program_fill_blocked.json` next to `settings.json`, outside the MAC (it only ever removes offers; R176), at most 200 entries. Keys: `package:<family>`, `signed:<exe>:<signer b64url>`, `path:<folded path>` (an update of an unsigned program stays blocked).
- **Settings.** `programFill` (default off, Q1, first-unlock card), `programFillSubmit` (default on), both in the MAC only while `programFill` is on (so existing MACs stay valid, like `breachCheck`); an unverified file reads both as off. `programFillHotkey` (default none, Q4; not a protection, outside the MAC; must differ from `globalShortcut`). D3 of ADR-0157 holds: Chromium and Electron programs follow the global setting.
- **Hotkey.** Opens the list directly for a detected login field; on an elevated target it shows the administrator notice; on anything else "Mosaic can't see a login field here". "Type into this program" for undetected fields (spec §5.1.4) is not in phase 2.
- **Locked vault.** Before the first unlock of a run nothing is watched. Later, while locked, the chip shows a lock on password and user-name fields; a click opens the main window's unlock and, within 60 s, Mosaic refocuses the field, captures again and opens the list only if the same element (RuntimeId) is focused.
- **Exclusions added to ADR-0158.** Package families `microsoft.aad.brokerplugin_cw5n1h2txyewy` and `microsoft.windows.cloudexperiencehost_cw5n1h2txyewy`, exe `microsoft.aad.brokerplugin.exe` → `Broker` (they show federated sign-in pages of any tenant); `wwahost.exe` → `Host`.
- **Timers** in one table: debounce 150 ms, pick guard 300 ms, link guard 1000 ms, second confirmation gap 1000 ms, pick TTL 30 s, continuation 60 s, UIA timeouts 2000 ms, host call timeout 5000 ms (then the worker is abandoned and respawned), rect poll 250 ms, notice 6000 ms, unlock resume 60 s.

Alternatives considered: one picker window with `set_focusable` toggling (not verified at runtime in tao 0.37.2); polling focus instead of events (idle cost); SetValue for passwords (not readable back, unproven).

- [ ] **Step 3: THREAT_MODEL §4.10 rows** (append to the table; each names ADR-0164 and the test that pins it)

| Threat | Mitigation |
|---|---|
| A fill or submit without a pick: a program focuses its own field again and again | focus events only show the chip; writes only from `picker_choose`, a confirmed link or that pick's continuation (`no_fill_without_a_pick`) |
| A secret reaches a picker webview | the picker gets titles, user names, hosts and program display data only; passwords go from the vault to the shim inside Rust (`picker_webview_never_receives_a_secret`) |
| Typed characters land in another field or window when focus moves mid-fill | modifier wait, batches of at most 8 units with the full check before each, a focus event aborts, a short `SendInput` aborts, cleanup only in the same field (`focus_change_during_typing_aborts_and_cleans_up`) |
| Another program moves, covers or overlays Mosaic's list to steer a pick or a link | the list must be foreground, at Rust's placement and clear at five UIA probe points; picks only from Rust's offer; 1 s link guard; two confirmations for unsigned programs (`a_moved_or_covered_list_refuses_picks_and_links`) |
| Script in the main webview writes a `winapp://` binding through the item editor | `UrlChange` add and update refuse `winapp://`; bindings come only from a captured identity in the link flow (`the_item_editor_cannot_add_or_edit_a_program_binding`) |
| A two-step continuation fills a second screen of another program or window | the same root window, window and element PID, the held process handle, a password field, 60 s, once (`continuation_needs_same_window_and_time`) |
| A hung program stalls the fill or Mosaic | 2 s UIA timeouts, a 5 s host call timeout and a new worker; the fill ends with nothing more written |
| Mosaic itself runs as administrator and would drive elevated programs | program fill does not start above medium integrity (`an_elevated_mosaic_does_not_watch_programs`) |
| A federated sign-in broker shows any tenant's page as if it were one program | the AAD broker and CloudExperienceHost are excluded as `Broker` (ADR-0164) |

- [ ] **Step 4: Residual rows R171 to R176** (§5 register, format of R150)

- R171: a same-user process synthesises input (SendInput, posted messages) into the chip and list; WebView2 input arrives in the webview process, so Mosaic cannot tell injected from hardware input. It can pick a login offered for the focused program or walk the link flow after the guards and link any login to a program it controls. Accepted (P5, like R45); narrowed by the checks of ADR-0164.
- R172: while the list is open, any same-user UIA client can read its titles, user names, vault names and hosts (search results in the link flow included); never a secret; the window is excluded from screen capture. Accepted.
- R173: typed characters pass through the Windows input queue, where low-level keyboard hooks of same-user processes see them, and Windows' own copies are not zeroized. Accepted, as a user typing.
- R174: Electron, CEF and WebView2 programs that render remote web content (in their own process, or in a WebView2 process descended from the signed host) receive the linked login when that content has focus. Accepted (spec §6.4; owner decision 2026-11-15 for WebView2).
- R175: an abort after part of the password was typed leaves those characters in the field when focus already moved (Mosaic clears only the same field) and delivers at most one batch (8 units) to whatever took focus before the check saw it. Accepted.
- R176: the block list is outside the settings MAC; an outside edit can remove entries so Mosaic offers logins again where the user said no (it never fills without a pick). Accepted.

- [ ] **Step 5: Commit**

```bash
git add docs/DECISIONS.md docs/THREAT_MODEL.md
git commit -m "docs(adr): ADR-0163 shim phase 2 and ADR-0164 program picker, fill and linking"
```

---

### Task 2: Binding data, broker exclusions and suggestions (runs anywhere)

**Files:**
- Modify: `crates/origin/src/program.rs`, `crates/origin/src/program_tests.rs`
- Modify: `crates/core/src/model/types.rs` (accessors + a test in its test module)
- Create: `crates/program-fill/src/binding.rs`, `crates/program-fill/src/suggest.rs` (tests at the end of each); Modify: `crates/program-fill/src/lib.rs`

**Interfaces:**
- Consumes: phase 1 `ProgramUrl`, `program_fill_allowed`, `binding_change`, `BindingChange`, `fold_name`, `classify_program`, `CapturedIdentity`, `SignerDisplay`; core `Object`, `Value`.
- Produces:

```rust
// mosaic_core::model::LoginUrl
pub fn extra(&self) -> &Object;
pub fn extra_mut(&mut self) -> &mut Object; // "url", "match", "allow_http" written here are overwritten on save

// mosaic_program_fill::binding
pub const MAX_DISPLAY_CHARS: usize = 256;
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProgramDisplay { pub name: String, pub signer: Option<String> }
pub fn display_for(identity: &CapturedIdentity) -> ProgramDisplay;
pub fn program_display(url: &LoginUrl) -> Option<ProgramDisplay>;
pub fn set_program_display(url: &mut LoginUrl, display: &ProgramDisplay);
pub fn submit_enabled(url: &LoginUrl) -> bool;
pub fn set_submit(url: &mut LoginUrl, on: bool);
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BindingMatch { Matched(Vec<usize>), LinkAgain(BindingChange), NoBinding }
pub fn match_bindings(saved: &[LoginUrl], identity: &CapturedIdentity) -> BindingMatch;
pub fn superseded(saved: &[LoginUrl], identity: &CapturedIdentity) -> Vec<usize>;
pub fn block_key(identity: &CapturedIdentity) -> Option<String>;

// mosaic_program_fill::suggest
pub const MIN_TERM_CHARS: usize = 3;
pub const MAX_SUGGESTIONS: usize = 5;
pub const STOP_WORDS: &[&str] = &["microsoft", "corporation", "corp", "inc", "llc", "ltd", "gmbh", "windows", "app", "software", "the"];
pub fn suggestion_terms(identity: &CapturedIdentity) -> Vec<String>;
pub fn suggests(terms: &[String], title: &str, hosts: &[&str]) -> bool;
```

Rules: `display_for`: name = the package family's part before `_` when packaged, else `exe_name`; signer = `organization`, else `common_name`; both truncated to `MAX_DISPLAY_CHARS` chars. `program_display` reads `extra["program"]` = object with `name` (text, 1 to 256 chars) and optional `signer` (text, at most 256); anything else is `None`. `set_submit(on: true)` removes `extra["submit"]`; `false` stores `Value::Bool(false)`; `submit_enabled` is false only for `Bool(false)`. `match_bindings`: indices where `program_fill_allowed` is `Ok`; none → `link_again_needed` (phase 1) as `LinkAgain`; else `NoBinding`. `superseded`: indices where `binding_change(url, identity)` is `Some`. `block_key`: class not `Ordinary` → `None`; packaged → `package:<family>`; signer `Some` → `signed:<exe>:<base64url signer>`; else `path:<fold_name(final_path)>`. `suggestion_terms`: exe stem (without a trailing `.exe`), package Name split on `.`, signer O and CN split on non-alphanumerics; folded; at least 3 chars; stop words dropped; deduplicated in that order. `suggests`: `fold_name(title)` or any host contains any term.

Broker additions in `program.rs` (ADR-0164): package families `microsoft.aad.brokerplugin_cw5n1h2txyewy`, `microsoft.windows.cloudexperiencehost_cw5n1h2txyewy` and exe `microsoft.aad.brokerplugin.exe` → `Broker`; exe `wwahost.exe` → `Host`.

- [ ] **Step 1: Write the failing tests**

```rust
// crates/origin/src/program_tests.rs
#[test] fn federated_sign_in_brokers_and_web_app_hosts_are_excluded() {
    assert_eq!(classify_program("x.exe", Some("microsoft.aad.brokerplugin_cw5n1h2txyewy")), ProgramClass::Broker);
    assert_eq!(classify_program("x.exe", Some("microsoft.windows.cloudexperiencehost_cw5n1h2txyewy")), ProgramClass::Broker);
    assert_eq!(classify_program("microsoft.aad.brokerplugin.exe", None), ProgramClass::Broker);
    assert_eq!(classify_program("wwahost.exe", None), ProgramClass::Host);
}

// crates/core/src/model/types.rs tests
#[test] fn login_url_extra_members_round_trip_and_cannot_override_known_keys() {
    // extra_mut().insert("program", object {name:"Slack"}) and insert("url", text "evil"):
    // from_value(to_value()) keeps "program", and .url is the original URL, not "evil"
}

// crates/program-fill/src/binding.rs tests
#[test] fn display_comes_from_package_name_or_exe_and_prefers_the_organization() { /* family "microsoft.windowscalculator_8wekyb3d8bbwe" -> name "microsoft.windowscalculator"; exe "slack.exe" -> "slack.exe"; O over CN; a 300-char O -> 256 chars */ }
#[test] fn program_display_round_trips_and_rejects_malformed_members() { /* set then read; name 0 or 257 chars -> None; name a number -> None */ }
#[test] fn submit_is_on_unless_explicitly_false() { /* default true; set_submit(false) -> false; set_submit(true) removes the member */ }
#[test] fn match_bindings_reports_matches_then_link_again() { /* matched indices; signer changed -> LinkAgain(SignerChanged); unrelated -> NoBinding */ }
#[test] fn superseded_lists_only_changed_bindings_of_this_program() { /* old signer of same exe -> listed; other exe -> not; an equal binding -> not */ }
#[test] fn block_keys_ignore_the_unsigned_file_hash() {
    // two unsigned identities, same path in other case, other hashes -> equal keys "path:c:\\tools\\t.exe"
    // excluded class -> None; packaged -> "package:<family>"
}

// crates/program-fill/src/suggest.rs tests
#[test] fn terms_come_from_exe_package_and_signer_without_stop_words() {
    // exe "Slack.exe", O "Slack Technologies, LLC" -> ["slack", "technologies"]; "Microsoft Corporation" adds nothing
}
#[test] fn suggestions_match_title_or_host_case_insensitively() { /* "SLACK work" and host "app.slack.com" match; "Slick" does not */ }
```

Fill the commented bodies with the assertions their comments name.

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-origin program` and the same for `-p mosaic-core login_url_extra` and `-p mosaic-program-fill binding suggest`
Expected: FAIL (missing items).

- [ ] **Step 3: Implement** per the Interfaces and Rules. `binding.rs` uses `mosaic_core::model::{Object, Value}`; no parsing of display data anywhere outside `binding.rs`.

- [ ] **Step 4: Run the tests**

Run: the three commands of Step 2 without filters for `mosaic-program-fill` and `mosaic-origin`; `-p mosaic-core model` for core.
Expected: PASS; fmt and clippy clean for the three crates.

- [ ] **Step 5: Commit**

```bash
git add crates/origin crates/core/src/model/types.rs crates/program-fill
git commit -m "feat(program-fill): binding display data, submit switch, block keys and link suggestions (ADR-0164)"
```

---

### Task 3: Login-field roles and picker placement (runs anywhere)

**Files:**
- Create: `crates/program-fill/src/detect.rs`, `crates/program-fill/src/placement.rs` (tests at the end of each); Modify: `crates/program-fill/src/lib.rs`

**Interfaces:**
- Consumes: phase 1 `verify::{Rect, Point, RuntimeId, probe_points}`.
- Produces:

```rust
// mosaic_program_fill::detect
pub const CONTAINER_LEVELS: u8 = 3;
pub const MAX_EDITS_SCANNED: usize = 64;
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct EditInfo { pub runtime_id: RuntimeId, pub is_password: bool, pub enabled: bool, pub offscreen: bool, pub rect: Rect }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FieldRole { Password { username: Option<usize> }, Username { password: usize }, Text }
pub fn usable(edit: &EditInfo) -> bool;                         // enabled && !offscreen && probe_points(rect).is_some()
pub fn classify(edits: &[EditInfo], focused: usize) -> Option<FieldRole>;
pub fn lone_text(window_edits: &[EditInfo], focused: &RuntimeId) -> bool; // exactly one usable Edit, and it is `focused`

// mosaic_program_fill::placement
pub const CHIP_PX: i32 = 24;      // logical
pub const GAP_PX: i32 = 4;        // logical
pub const LIST_WIDTH_PX: i32 = 320;
pub const LIST_HEIGHT_PX: i32 = 360;
pub const LIST_MIN_HEIGHT_PX: i32 = 160;
pub fn chip_rect(field: Rect, monitor: Rect, scale: f64) -> Option<Rect>;
pub fn list_rect(field: Rect, monitor: Rect, scale: f64) -> Option<Rect>;
```

Rules: `edits` are in UIA tree order from one container (the caller scans the ancestor `CONTAINER_LEVELS` above the focused Edit). `classify`: `focused` out of range or not `usable` → `None`. Password → `username` = the nearest index before it that is usable and not a password. Non-password → `Username { password: k }` if `k` is the nearest usable password Edit after it **and** `classify(edits, k)` names `focused` as its user name; else `Text` if no usable password Edit is in `edits`; else `None`. Placement (all arithmetic in `i64`, results in `i32` physical px; logical sizes × `scale`, rounded): chip square of side `round(24 × scale)`, `x = field.right + gap` if it fits the monitor, else `field.left − gap − side`, else `None`; `y` centred on the field and clamped into the monitor. List: width `min(round(320 × scale), monitor width)`, `x = clamp(field.left, monitor.left, monitor.right − width)`; below the field when `round(360 × scale)` fits; else above; else the larger side with the height shrunk to fit if at least `round(160 × scale)`; else `None`. Either result never intersects `field` and lies inside `monitor`; a field that does not intersect the monitor, or for which `probe_points` is `None`, gives `None`; `scale` outside 0.5 to 5.0 gives `None`.

- [ ] **Step 1: Write the failing tests**

```rust
// detect.rs
fn e(id: i32, pw: bool) -> EditInfo { /* usable 200x30 rect at y = id * 40 */ }
#[test] fn a_password_field_takes_the_nearest_text_field_before_it() { /* [user, pw] focus 1 -> Password{username: Some(0)} */ }
#[test] fn only_the_nearest_text_field_before_a_password_is_a_username() {
    // [email, name, pw]: focus 1 -> Username{password: 2}; focus 0 -> None; focus 2 -> Password{username: Some(1)}
}
#[test] fn disabled_hidden_or_tiny_fields_are_skipped() { /* [user(disabled), pw] focus 1 -> Password{username: None}; focus 0 -> None */ }
#[test] fn a_text_field_without_any_password_field_is_a_text_candidate() { /* [search] focus 0 -> Text; [search, chat] focus 1 -> Text */ }
#[test] fn a_text_field_after_the_password_is_not_a_login_field() { /* [user, pw, code] focus 2 -> None */ }
#[test] fn lone_text_needs_exactly_one_usable_edit_in_the_window() { /* [x] -> true; [x, y] -> false; [x, y(disabled)] -> true; other focused id -> false */ }

// placement.rs
#[test] fn the_chip_sits_right_of_the_field_or_left_when_there_is_no_room() { /* never intersects; inside monitor */ }
#[test] fn the_list_opens_below_or_flips_above_and_shrinks_to_fit() { /* bottom-edge field -> above; 300 px tall monitor -> shrunk; 100 px -> None */ }
#[test] fn placement_on_a_left_monitor_with_negative_coordinates_and_mixed_dpi() {
    // monitor Rect{left:-2560, top:-200, right:0, bottom:1240}, scale 1.5, field at the monitor's right edge:
    // chip_rect is left of the field, side 36; list_rect width 480, inside the monitor; neither intersects the field
}
#[test] fn odd_fields_and_scales_place_nothing() { /* inverted, 3 px, off-monitor field; scale 0.0, NaN, 9.0 -> None */ }
```

Add a `proptest` per function: for any field, monitor and `scale` in 0.5..5.0, the result is `None` or inside the monitor and disjoint from the field.

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill detect placement`
Expected: compile errors.

- [ ] **Step 3: Implement** per the Interfaces and Rules.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill`
Expected: PASS; fmt and clippy clean.

- [ ] **Step 5: Commit**

```bash
git add crates/program-fill
git commit -m "feat(program-fill): login-field roles and picker placement (ADR-0164)"
```

---

### Task 4: The fill sequence, the host trait and their fakes (runs anywhere)

**Files:**
- Create: `crates/program-fill/src/fill.rs`, `crates/program-fill/src/host.rs`
- Modify: `crates/program-fill/src/verify.rs` (`retarget`), `crates/program-fill/src/fake.rs` (`FakeDriver`, `FakeHost`, `FakeClock`), `crates/program-fill/src/lib.rs`, `crates/program-fill/Cargo.toml` (`zeroize.workspace = true`)

**Interfaces:**
- Consumes: phase 1 `PickSession`, `check`, `ProgramProbe`, `ProbeError`, `VerifyFailure`, `WriteKind`, `FieldKind`, `FieldFacts`, `RuntimeId`, `CONTINUATION_TTL_MS`, `CaptureRefusal`, `CapturedIdentity`; Task 3 `FieldRole`, `Rect`; `mosaic_core::SecretString`.
- Produces:

```rust
// verify.rs
impl<H> PickSession<H> { pub fn retarget(&mut self, target: RuntimeId, kind: FieldKind); }

// mosaic_program_fill::fill
pub const TYPE_BATCH_UNITS: usize = 8;
pub const MODIFIER_WAIT_MS: u64 = 2_000;
pub const MODIFIER_POLL_MS: u64 = 20;
pub const FOCUS_SETTLE_MS: u64 = 50;
pub const SETVALUE_FRAMEWORKS: &[&str] = &["Win32", "WinForm", "WPF"];
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Key { Enter, SelectAll, Backspace }
pub trait Clock { fn now_ms(&self) -> u64; fn sleep_ms(&self, ms: u64); }
pub trait FillDriver {
    type Held;
    fn focus(&self, session: &PickSession<Self::Held>) -> Result<(), ProbeError>; // foreground root, UIA SetFocus, settle, reset the focus flag
    fn focus_changed(&self) -> bool;                                               // a focus event since the last `focus`
    fn can_set_value(&self, session: &PickSession<Self::Held>) -> bool;
    fn set_value(&self, session: &PickSession<Self::Held>, value: &str) -> Result<bool, ProbeError>; // true: read back equal
    fn send_units(&self, units: &[u16]) -> Result<u32, ProbeError>;                // events inserted
    fn send_key(&self, key: Key) -> Result<u32, ProbeError>;
    fn modifiers_down(&self) -> bool;
}
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FillPlan { pub username: Option<RuntimeId>, pub password: Option<RuntimeId>, pub submit: bool }
pub struct FillSecrets<'a> { pub username: Option<&'a str>, pub password: Option<&'a SecretString> }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FillAbort { Verify(VerifyFailure), ModifiersHeld, InputBlocked, ReadBackMismatch, Probe(ProbeError) }
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FillOutcome { Done { submitted: bool }, Aborted { at: WriteKind, reason: FillAbort, cleared: bool }, NothingToFill }
pub fn run_fill<P, D>(probe: &P, driver: &D, session: &mut PickSession<P::Held>, plan: &FillPlan,
                      secrets: &FillSecrets<'_>, clock: &dyn Clock) -> FillOutcome
where P: ProgramProbe, D: FillDriver<Held = P::Held>;
pub fn utf16_batches(units: &[u16]) -> Vec<core::ops::Range<usize>>;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Continuation { pub root: isize, pub window_pid: u32, pub element_pid: u32, pub picked_at_ms: u64, pub used: bool }
pub fn continues(c: &Continuation, field: &FieldFacts, root: isize, window_pid: u32, now_ms: u64) -> bool;

// mosaic_program_fill::host
pub type Token = u64;
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FieldOffer { pub token: Token, pub identity: CapturedIdentity, pub role: FieldRole, pub rect: Rect, pub target: RuntimeId }
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum HostEvent { Field(FieldOffer), Gone, Refused { refusal: CaptureRefusal, near: Option<Rect> }, NoField { near: Option<Rect> }, Continue { token: Token } }
pub struct FillRequest { pub token: Token, pub username: Option<String>, pub password: Option<SecretString>, pub submit: bool, pub picked_at_ms: u64, pub continuation: bool }
pub type HostSink = std::sync::Arc<dyn Fn(HostEvent) + Send + Sync>;
pub trait Host: Send + Sync {
    fn start(&self, sink: HostSink) -> Result<(), ProbeError>;
    fn stop(&self);
    fn capture_now(&self) -> Result<HostEvent, ProbeError>;          // the hotkey: Field, Refused or NoField
    fn lone_text(&self, token: Token) -> Result<bool, ProbeError>;
    fn fill(&self, request: FillRequest) -> Result<FillOutcome, ProbeError>;
    fn refocus(&self, token: Token) -> Result<(), ProbeError>;
    fn field_rect(&self, token: Token) -> Result<Option<Rect>, ProbeError>;
    fn arm_continuation(&self, token: Token, continuation: Continuation);
    fn window_clear(&self, own_hwnd: isize, rect: Rect) -> Result<bool, ProbeError>;
}
```

`FillRequest` has a hand-written `Debug` that prints only `token`, `submit` and `continuation`. `fake.rs` adds `FakeDriver` (records every call as an enum `DriverCall`, scripted `modifiers_down` sequence, `send_units` returning `2 × len` unless told to short-count, `focus_changed` scripted per batch), `FakeClock` (`Cell<u64>`, `sleep_ms` advances it), and `FakeHost` (scripted results, a `Mutex<Vec<HostCall>>` log, `emit(event)` calling the stored sink; `HostCall::Fill` records `token`, `username`, whether a password was present, `submit`, `continuation`, never the password).

**Order in `run_fill`** (first abort wins; `check` is phase 1's):
1. `plan` has no password target with a non-empty password and no user-name target with a non-empty user name → `NothingToFill`.
2. Modifier wait: poll `modifiers_down` every `MODIFIER_POLL_MS` until false; after `MODIFIER_WAIT_MS` → `Aborted { at: first write, ModifiersHeld, cleared: false }`.
3. User name (target and non-empty value): `retarget(id, Text)`, `focus`, `check(Username)`; if `can_set_value` → `set_value`, `false` → `ReadBackMismatch`; else type it.
4. Password (target and non-empty value): `retarget(id, Password)`, `focus`, type it, never `set_value`.
5. Submit only if `plan.submit` and a password was typed: `check(Submit)`, `send_key(Enter)`, fewer than 2 events → `InputBlocked`.
- Typing a value: `Zeroizing<Vec<u16>>` of its UTF-16; for each range of `utf16_batches`: `focus_changed()` → `Verify(FocusMoved)`; `check(kind)`; `send_units`, fewer than `2 × len` events → `InputBlocked`.
- Cleanup after an abort while typing a value with at least one batch sent: `check(kind)` once; `Ok` → `send_key(SelectAll)`, `send_key(Backspace)`, `cleared: true`; else `cleared: false`.
- `utf16_batches`: consecutive ranges of at most `TYPE_BATCH_UNITS`, shortened by one when the last unit is a high surrogate followed by a low surrogate.
- `continues`: `!used`, `now ≥ picked_at`, `now − picked_at < CONTINUATION_TTL_MS`, equal `root`, `window_pid`, `field.pid == element_pid`, `field.kind == Password`.

- [ ] **Step 1: Write the failing tests** (in `fill.rs`, with `FakeProbe`, `good_snapshot`, `FakeDriver`, `FakeClock`)

```rust
#[test] fn a_pick_fills_user_name_then_password_then_presses_enter() {
    // calls in order: focus(user), set_value or send_units..., focus(pw), send_units x ceil(len/8), send_key(Enter); Done{submitted:true}
}
#[test] fn the_password_is_typed_even_where_set_value_is_possible() { /* can_set_value true -> no set_value on the password target */ }
#[test] fn focus_change_during_typing_aborts_and_cleans_up() {
    // focus_changed true before batch 2 -> Aborted{at: Password, Verify(FocusMoved), cleared: true} and SelectAll+Backspace sent
    // and when the cleanup check fails (snapshot focused None) -> cleared: false, no SelectAll
}
#[test] fn window_swap_or_foreground_change_aborts_typing() { /* snapshot foreground other root after batch 1 -> Verify(WindowChanged), no more send_units */ }
#[test] fn short_send_input_aborts() { /* send_units returns len (not 2*len) -> InputBlocked */ }
#[test] fn held_modifiers_delay_then_abort_without_typing() {
    // modifiers down for 500 ms then up -> Done; down for 2_000 ms -> Aborted{ModifiersHeld}, zero send_units, clock advanced >= 2_000
}
#[test] fn empty_password_fills_the_user_name_and_never_submits() { /* password Some("") -> user name written, no Enter, Done{submitted:false} */ }
#[test] fn submit_off_leaves_focus_in_the_password_field() { /* plan.submit false -> no send_key(Enter) */ }
#[test] fn an_expired_pick_writes_nothing() { /* clock at picked_at + PICK_TTL_MS -> Aborted{Verify(Expired)}, zero writes */ }
#[test] fn surrogate_pairs_are_never_split_across_batches() {
    // "1234567😀x": utf16_batches == [0..7, 7..10]; a 70-unit ASCII password -> 9 ranges covering 0..70 without gaps
}
#[test] fn continuation_needs_same_window_and_time() {
    // continues: equal facts at +59_999 true; +60_000 false; used true false; other root/pid/element pid false; Text field false; now < picked false
}
```

`no_fill_without_a_pick` lives in Task 8 (the desktop decides when `fill` is called); here a `proptest` checks that for any scripted snapshot sequence with a failing check, the number of `send_units` calls after the first failing check is zero.

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill fill`
Expected: compile errors.

- [ ] **Step 3: Implement** `retarget`, `fill.rs`, `host.rs` and the fakes per the Interfaces and the order above.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill --features fake`
Expected: PASS; fmt and clippy clean (also with `--features fake`).

- [ ] **Step 5: Commit**

```bash
git add crates/program-fill Cargo.lock
git commit -m "feat(program-fill): fill sequence with checks before every batch, host trait and fakes (ADR-0164)"
```

---

### Task 5: `mosaic-winshim` phase 2 API (Windows machine; compiles empty elsewhere)

**Files:**
- Modify: `crates/winshim/Cargo.toml` (features of ADR-0163, `zeroize.workspace = true` under `cfg(windows)`), `crates/winshim/src/lib.rs`, `crates/winshim/src/sys_uia.rs`, `crates/winshim/src/sys_window.rs`
- Create: `crates/winshim/src/sys_input.rs`
- Modify: `docs/DEPENDENCIES.md` (`windows` row: the two features; `zeroize` row: used by `winshim`, `program-fill`)

**Interfaces:**
- Consumes: phase 1 shim (`Uia`, `UiaElement`, `ElementFacts`, `ShimError`, `window_facts`, `foreground_window`); ADR-0163.
- Produces (all `#[cfg(windows)]`):

```rust
pub const MAX_UNITS_PER_CALL: usize = 16;
// ShimError gains: TooLong, NotOwnWindow
pub type FocusSink = std::sync::mpsc::SyncSender<u32>; // PID of the newly focused element, 0 if unknown
pub struct FocusSubscription { /* private; !Send; Drop removes the handler */ }
impl Uia {
    pub fn subscribe_focus(&self, sink: FocusSink) -> Result<FocusSubscription>; // try_send, never blocks a UIA thread
    pub fn edits_near(&self, e: &UiaElement, levels_up: u8, max: usize) -> Result<Vec<UiaElement>>; // Edit descendants of the ancestor `levels_up` above (stops at the window), tree order
    pub fn edits_in_window(&self, root: isize, max: usize) -> Result<Vec<UiaElement>>;
    pub fn refresh(&self, e: &UiaElement) -> Result<UiaElement>;     // fresh cached facts
    pub fn set_focus(&self, e: &UiaElement) -> Result<()>;
    pub fn set_value(&self, e: &UiaElement, value: &str) -> Result<()>; // AccessDenied for a password element
    pub fn value(&self, e: &UiaElement) -> Result<String>;              // AccessDenied for a password element
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Key { Enter, SelectAll, Backspace }
pub fn send_unicode(units: &[u16]) -> Result<u32>; // 2 INPUTs per unit (down, up, KEYEVENTF_UNICODE); > MAX_UNITS_PER_CALL -> TooLong
pub fn send_key(key: Key) -> Result<u32>;          // Enter: 2 events; SelectAll: Ctrl down, A down, A up, Ctrl up; Backspace: 2
pub fn modifiers_down() -> bool;                   // GetAsyncKeyState high bit of VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN
pub fn set_foreground(hwnd: isize) -> Result<bool>; // top-level windows only (GetAncestor(GA_ROOT) == hwnd), else NotFound
pub fn show_no_activate(hwnd: isize) -> Result<()>; // own-process windows only, else NotOwnWindow; SW_SHOWNOACTIVATE + HWND_TOPMOST with SWP_NOACTIVATE
```

`FocusSubscription` holds the handler's interface and the client's `IUIAutomation`; its handler implements `IUIAutomationFocusChangedEventHandler` with `#[windows::core::implement]` and reads the sender's `CurrentProcessId`; a failure sends 0. `send_unicode`'s records: `#[repr(transparent)] struct InputRecord(INPUT)` with `Default`, `Copy` and `zeroize::DefaultIsZeroes`, in `Zeroizing<Vec<InputRecord>>` with capacity `2 × units.len()`.

- [ ] **Step 1: Write the failing tests** (`#[cfg(all(test, windows))]` in `lib.rs`)

```rust
#[test] fn send_unicode_refuses_more_than_sixteen_units() { assert_eq!(send_unicode(&[0x41; 17]), Err(ShimError::TooLong)); }
#[test] fn send_unicode_of_nothing_sends_nothing() { assert_eq!(send_unicode(&[]), Ok(0)); }
#[test] fn show_no_activate_refuses_other_processes_and_null() {
    assert!(show_no_activate(0).is_err());
    // foreground_window() owned by another process (window_facts(..).owner_pid != std::process::id()) -> Err(NotOwnWindow); skip when None
}
#[test] fn set_foreground_refuses_null_and_child_windows() { assert!(set_foreground(0).is_err()); }
#[test] fn modifiers_down_answers_without_input() { let _ = modifiers_down(); }
#[test] fn public_api_still_has_no_unsafe_fn_and_no_windows_types() { /* phase 1 test extended to the new pub lines */ }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn focus_events_arrive_with_the_pid() { /* subscribe, click into Notepad within 5 s, recv_timeout(5 s) gives Notepad's pid */ }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn set_value_and_value_round_trip_in_notepad_and_refuse_passwords() { }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn send_unicode_types_into_the_focused_field() { }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn edits_near_and_in_window_list_a_dialogs_edits_in_order() { }
```

- [ ] **Step 2: Run them to verify they fail**

Run (Windows): `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-winshim`
Expected: compile errors.

- [ ] **Step 3: Implement** per the Interfaces and ADR-0157 D2 (one `unsafe` operation per block with `// SAFETY:`; `Drop` removes the handler on the owning thread; `SAFEARRAY` and `BSTR` freed on every path; `FindAll` uses the phase 1 cache request plus `CreatePropertyCondition(UIA_ControlTypePropertyId, UIA_EditControlTypeId)`; `edits_*` stop at `max`).

- [ ] **Step 4: Run the tests and checks**

Run (Windows): `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-winshim`, then run the four ignored tests by hand with `-- --ignored --test-threads=1` following their comments, clippy for `mosaic-winshim`, `bash scripts/check-unsafe-confined.sh`, `cargo deny check`, `cargo audit --deny warnings`.
Expected: all pass; the ignored ones pass when walked by hand; `Cargo.lock` gains no new package.

- [ ] **Step 5: Commit**

```bash
git add crates/winshim docs/DEPENDENCIES.md Cargo.lock
git commit -m "feat(winshim): focus events, UIA writes, zeroized SendInput and no-activate show (ADR-0163)"
```

- [ ] **Step 6: Hand-off (orchestrator)**

Crypto-reviewer reading of every new `unsafe` block and a Fable red-team review of the changed shim, recorded in ADR-0163's review log; fixes go back to this task's worktree before Task 6 merges.

---

### Task 6: The Windows host: worker thread, captures, fills (Windows machine)

**Files:**
- Create: `crates/program-fill/src/windows/host.rs`, `crates/program-fill/src/windows/driver.rs`
- Modify: `crates/program-fill/src/windows/mod.rs` (`pub use host::WindowsHost;`, `WindowsProbe::field_set`), `crates/program-fill/examples/probe.rs` (a `--host` mode)

**Interfaces:**
- Consumes: Task 4 `Host`, `HostEvent`, `FieldOffer`, `FillRequest`, `run_fill`, `FillDriver`, `Clock`, `Continuation`, `continues`; Task 3 `classify`, `lone_text`, `EditInfo`, `CONTAINER_LEVELS`, `MAX_EDITS_SCANNED`; Task 5 shim; phase 1 `WindowsProbe`, `Captured`, `CaptureError`, `check`.
- Produces:

```rust
// crates/program-fill/src/windows/host.rs (#[cfg(windows)])
pub const DEBOUNCE_MS: u64 = 150;
pub const HOST_CALL_TIMEOUT_MS: u64 = 5_000;
pub const IDENTITY_CACHE: usize = 16;
pub struct WindowsHost { /* Mutex<Option<Worker>>: the command sender, the worker's epoch, the sink */ }
impl WindowsHost { pub fn new() -> Self; }
impl Host for WindowsHost { /* Task 4 trait */ }

// windows/driver.rs
pub(crate) struct WindowsDriver<'a> { /* &Uia, the session's elements by RuntimeId, Arc<AtomicBool> focus flag */ }
impl FillDriver for WindowsDriver<'_> { type Held = mosaic_winshim::ProcessHandle; /* … */ }

// windows/mod.rs
impl WindowsProbe {
    /// The Edits of the focused field's container (shim `edits_near(.., CONTAINER_LEVELS, MAX_EDITS_SCANNED)`), as `EditInfo`, and the focused one's index.
    pub fn field_set(&self, captured: &Captured) -> Result<(Vec<EditInfo>, usize), ProbeError>;
}
```

Worker rules (one thread per worker, `CoInitializeEx(MTA)` through `Uia::new(2000, 2000)`, owns `WindowsProbe` and the `FocusSubscription`):
- Commands arrive on an `mpsc` channel; focus pings on the `FocusSink` (`sync_channel(64)`). A ping with Mosaic's own PID is dropped. After a ping the worker waits until `DEBOUNCE_MS` pass without another, then: `capture_focused`; `NotAField` or a refusal → `Gone`; else `field_set` + `classify`; `None` → `Gone`; `Some(role)` → if an armed continuation `continues` for the focused facts → `Continue { token }`, else a new token and `Field(FieldOffer)`. The latest capture replaces the previous one (its handle drops), except the armed continuation's capture, kept until used or expired.
- `capture_now` runs the same capture at once and answers `Field`, `Refused { refusal, near }` or `NoField { near }` (`near` = the focused element's rect when one was read).
- `fill`: unknown or old token → `Err(ProbeError::Gone)`; builds the session from the capture (`picked_at_ms`, `continuation` from the request), the plan from the role (Password → user name from the role's index; Username → its password index; Text → user name only), runs `run_fill` with `WindowsDriver`; pings during the fill set the driver's focus flag and are not turned into offers.
- `lone_text(token)`: `edits_in_window(root, MAX_EDITS_SCANNED)` → `lone_text`.
- `window_clear(own_hwnd, rect)`: each of `probe_points(rect)` through `element_from_point` → `nearest_window` → `window_facts(..).root == own_hwnd`; all five must hold.
- Identity cache: by PID, `IDENTITY_CACHE` entries, an entry valid while its held handle `is_running()`.
- A call that gets no answer in `HOST_CALL_TIMEOUT_MS` returns `Err(ProbeError::Timeout)`, the worker is abandoned (its epoch no longer matches, so its later events are dropped) and a new one starts.

- [ ] **Step 1: Write the tests** (`#[cfg(all(test, windows))]` in `host.rs`)

```rust
#[test] fn a_host_starts_and_stops_without_a_desktop_focus() { /* new(), start(sink), capture_now() is Ok(NoField|Field|Refused) or Err, never a panic; stop() */ }
#[test] fn calls_after_stop_fail_cleanly() { /* stop(); fill(..) == Err(ProbeError::Gone) */ }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn a_focused_password_field_produces_one_offer_after_the_debounce() { }
#[test] #[ignore = "interactive desktop: docs/PROGRAM_FILL.md checklist"] fn a_hung_program_times_out_and_the_host_recovers() { /* phase 1 Step 3 test window; capture_now Err(Timeout) within 5.5 s; next call Ok */ }
```

- [ ] **Step 2: Implement** `host.rs`, `driver.rs`, `field_set`, and `examples/probe.rs --host`: starts a `WindowsHost`, prints each `HostEvent` (exe, class, role, rect; never values), and on `f` + Enter in its console fills the latest offer with the user name `mosaic-test` and the password `Pässwörd-😀-12345678` (submit off) and prints the `FillOutcome`.

- [ ] **Step 3: Run**

Run (Windows): `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-program-fill --features fake`, clippy for `mosaic-program-fill`, `bash scripts/check-unsafe-confined.sh`; then the two ignored tests by hand.
Expected: PASS; the harness fills a WinForms or Win32 test dialog's user name and password exactly (checked by eye in the dialog).

- [ ] **Step 4: Commit**

```bash
git add crates/program-fill
git commit -m "feat(program-fill): Windows host with focus events, debounced offers and verified fills (ADR-0164)"
```

---

### Task 7: Settings, block list, hotkey and status commands (runs anywhere)

**Files:**
- Modify: `apps/desktop/src-tauri/src/contract/settings.rs`, `src/persist.rs`, `src/settings_integrity_tests.rs`, `src/commands/settings.rs`, `src/shortcut.rs`, `src/lib.rs`
- Create: `apps/desktop/src-tauri/src/contract/program_fill.rs`, `src/commands/program_fill.rs`, `src/program_fill/mod.rs` (service skeleton: `ProgramFillService` with the block list and status only), `src/program_fill/blocklist.rs`
- Modify: `src/commands/names.rs`, `src/commands/mod.rs`, `capabilities/main.json`, `src/contract/mod.rs`, `src/contract/errors.rs` (codes `pickerStale`, `pickerTooSoon`, `programFillUnavailable`), `apps/desktop/src/ipc/contract/{settings,programFill,commands,index}.ts`, `apps/desktop/src/ipc/mock/` (settings defaults, the two commands)
- Modify: `docs/DESKTOP_IPC.md` (§3 rows, settings fields, §5 codes); `packages/i18n` `error.<code>` for the three codes (EN, DE)

**Interfaces:**
- Consumes: Task 2 `block_key`; `shortcut::{parse_accelerator, Registrar}`; `persist::{normalize, stricter_than_default, same_protection, canonical_settings}`.
- Produces:

```rust
// contract::settings::Settings gains (camelCase on the wire)
pub program_fill: bool,                    // default false
pub program_fill_submit: bool,             // default true
pub program_fill_hotkey: Option<String>,   // default None
pub program_fill_card_dismissed: bool,     // default false

// contract::program_fill
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)] #[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BlockedProgram { pub key: String, pub label: String }
wire_enum!(pub enum ProgramFillUnavailable { NotWindows, MosaicElevated, HostFailed });
pub struct ProgramFillStatus { pub available: bool, pub reason: Option<ProgramFillUnavailable>, pub running: bool, pub blocked: Vec<BlockedProgram> }
pub struct ProgramFillUnblockRequest { pub key: String }

// program_fill::blocklist
pub const MAX_BLOCKED: usize = 200;
pub const BLOCKLIST_FILE: &str = "program_fill_blocked.json";
pub struct BlockList { /* dir: Option<PathBuf>, entries: Vec<BlockedProgram> */ }
impl BlockList {
    pub fn in_dir(dir: &Path) -> Self;   // unreadable or malformed file -> empty
    pub fn memory() -> Self;
    pub fn contains(&self, key: &str) -> bool;
    pub fn add(&mut self, entry: BlockedProgram) -> IpcResult<()>; // dedup by key; > MAX_BLOCKED -> invalidInput; key <= 4200 bytes, label <= 256 chars
    pub fn remove(&mut self, key: &str) -> IpcResult<()>;
    pub fn entries(&self) -> &[BlockedProgram];
}

// commands (main window): `program_fill_status` null -> ProgramFillStatus; `program_fill_unblock` { key } -> ProgramFillStatus
// shortcut.rs: the registrar takes its action
pub fn install<R: Runtime>(app: &AppHandle<R>, accelerator: Accelerator) -> (ShortcutStatus, Option<Registrar>); // unchanged
pub fn install_action<R: Runtime>(app: &AppHandle<R>, action: fn(&AppHandle<R>)) -> Option<Registrar>; // second accelerator, plugin already installed
pub fn on_program_fill_hotkey<R: Runtime>(app: &AppHandle<R>); // -> backend.program_fill_hotkey_pressed() (Task 8 body; a no-op here)
```

Rules: `canonical_settings` adds `"programFill":true,"programFillSubmit":<bool>,` between `lockOnSleep` and `quickUnlockMaxAgeDays` only when `program_fill` is true; `stricter_than_default` sets both to `false` and keeps hotkey and card; `same_protection` compares both; `normalize`: `program_fill = false` unless `cfg!(windows)`; a hotkey that does not parse or equals `global_shortcut` (parsed, compared as `Shortcut`) becomes `None`. `PersistedSettings` reads missing fields as the defaults. `update_settings` applies a changed hotkey through the second registrar (errors as `apply_global_shortcut`) and calls `Backend::program_fill_apply_settings()` (Task 8; a no-op until then). The block list file is written atomically (temp file + rename) and is not part of `StoredState` or the MAC.

- [ ] **Step 1: Write the failing tests**

```rust
// settings_integrity_tests.rs
#[test] fn program_fill_and_submit_are_in_the_mac_only_while_on() {
    // defaults: canonical bytes equal today's fixture (existing MACs stay valid); program_fill true -> bytes contain "programFill":true,"programFillSubmit":true,
}
#[test] fn settings_without_a_valid_mac_turn_program_fill_and_submit_off() { /* file with programFill true and a bad MAC -> after the first unlock both false, settingsReset sent */ }
// persist.rs tests
#[test] fn a_hotkey_equal_to_the_global_shortcut_or_unparsable_is_dropped() { }
#[test] fn program_fill_is_off_on_other_systems() { /* cfg!(not(windows)) -> normalize gives false */ }
// program_fill/blocklist.rs tests
#[test] fn block_list_survives_a_restart_and_ignores_a_broken_file() { /* tempdir: add, in_dir again contains; garbage file -> empty */ }
#[test] fn block_list_dedups_and_caps() { /* same key twice -> one; 201st -> invalidInput; 4201-byte key -> invalidInput */ }
// handler_tests.rs
#[test] fn program_fill_status_reports_availability_and_the_block_list() { /* not Windows -> available false, reason notWindows; unblock removes */ }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop program_fill settings_integrity persist`
Expected: compile errors.

- [ ] **Step 3: Implement**, then update fixtures: `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures` (with the target-dir prefix), the TypeScript types, `Commands` map and `commandSet`, the mock defaults; DESKTOP_IPC §3 rows ("Program fill" group), the settings field list, §5 codes; i18n `error.pickerStale` ("This list is out of date. Click the field again." / "Diese Liste ist nicht mehr aktuell. Klick noch einmal ins Feld."), `error.pickerTooSoon` ("Wait a moment, then confirm again." / "Warte einen Moment und bestätige dann noch einmal."), `error.programFillUnavailable` ("Filling in programs isn't available on this device." / "Das Ausfüllen in Programmen ist auf diesem Gerät nicht verfügbar.").

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop`; `pnpm --filter @mosaic/desktop typecheck`; `pnpm -w exec vitest run --project desktop src/ipc`; `pnpm --filter @mosaic/i18n test`.
Expected: PASS; clippy and fmt clean for `mosaic-desktop`.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop packages/i18n docs/DESKTOP_IPC.md
git commit -m "feat(desktop): program fill settings under the MAC, block list, hotkey and status (ADR-0164)"
```

---

### Task 8: The program fill service: offers, picks, links, continuation (runs anywhere)

**Files:**
- Modify: `apps/desktop/src-tauri/src/program_fill/mod.rs`; Create: `src/program_fill/offer.rs`, `src/program_fill/link.rs`, `src/program_fill/picker_windows.rs` (the trait and a `NoPickerWindows`), `src/program_fill_tests.rs`
- Create: `apps/desktop/src-tauri/src/contract/picker.rs`; Modify: `src/contract/mod.rs`, `src/state.rs` (`Backend.program_fill`, `with_program_fill`), `src/lib.rs` (`#[cfg(test)] mod program_fill_tests;`), `apps/desktop/src-tauri/Cargo.toml` (`mosaic-program-fill = { path = "../../../crates/program-fill" }`; dev: with `features = ["fake"]`)

**Interfaces:**
- Consumes: Task 2 (`match_bindings`, `superseded`, `display_for`, `set_program_display`, `submit_enabled`, `block_key`, `suggestion_terms`, `suggests`, `MAX_SUGGESTIONS`), Task 3 (`chip_rect`, `list_rect`, `FieldRole`), Task 4 (`Host`, `HostEvent`, `FieldOffer`, `FillRequest`, `FillOutcome`, `FillAbort`, `Continuation`, `FakeHost`), phase 1 `propose_binding`, `LinkConfirmation`, `ConfirmStep`, `LinkWarning`, `CaptureRefusal`; Task 7 `BlockList`, settings.
- Produces:

```rust
// contract::picker (camelCase; tagged unions by `kind`)
wire_enum!(pub enum PickerMode { Hidden, Chip, List, Notice });
wire_enum!(pub enum ProgramKind { Package, Signed, Unsigned });
wire_enum!(pub enum LinkAgainReason { SignerChanged, FileChanged });
wire_enum!(pub enum PickerNotice { Elevated, NoField, Aborted, AbortedCleared, ModifiersHeld, NotResponding });
pub struct ProgramView { pub kind: ProgramKind, pub exe: String, pub name: String, pub signer: Option<String>, pub path: Option<String> }
pub struct PickerLogin { pub vault_id: WireVaultId, pub item_id: WireItemId, pub title: String, pub username: Option<String>, pub vault_name: Option<String>, pub host: Option<String> }
pub struct HashChange { pub old_prefix: String, pub new_prefix: String }          // 16 lowercase hex chars each
pub struct LinkStep { pub login: PickerLogin, pub program: ProgramView, pub confirmations: u8, pub confirmed: u8, pub lookalike_of: Option<String>, pub hash_change: Option<HashChange>, pub submit: bool }
pub struct PickerState { pub generation: u64, pub mode: PickerMode, pub locked: bool, pub language: Language, pub theme: Theme,
    pub program: Option<ProgramView>, pub matches: Vec<PickerLogin>, pub preselected: Option<u32>, pub suggestions: Vec<PickerLogin>,
    pub link_again: Option<LinkAgainReason>, pub link: Option<LinkStep>, pub notice: Option<PickerNotice> }
pub struct PickerGenerationRequest { pub generation: u64 }
pub struct PickerSearchRequest { pub generation: u64, pub query: String }
pub struct PickerItemRequest { pub generation: u64, pub vault_id: WireVaultId, pub item_id: WireItemId }
pub enum LinkConfirmResult { NeedAnother, TooSoon, Linked }                         // { kind }
pub enum PickerEvent { Changed { generation: u64 } }                               // { type: "changed", generation }

// program_fill::picker_windows
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)] pub enum PickerLabel { Chip, List }
pub trait PickerWindows: Send + Sync {
    fn show_chip(&self, at: Rect) -> bool;
    fn show_list(&self, at: Rect, focus: bool) -> bool; // focus false: the notice
    fn hide(&self, which: PickerLabel);
    fn placed(&self, which: PickerLabel) -> Option<Rect>;
    fn hwnd(&self, which: PickerLabel) -> Option<isize>;
    fn is_foreground(&self, which: PickerLabel) -> bool;
    fn monitor_at(&self, p: Point) -> Option<(Rect, f64)>;
    fn changed(&self, which: PickerLabel, generation: u64);
    fn focus_main(&self);
}

// program_fill::mod
pub const PICK_MIN_DELAY_MS: u64 = 300;
pub const LINK_MIN_DELAY_MS: u64 = 1_000;
pub const UNLOCK_RESUME_MS: u64 = 60_000;
pub const RECT_POLL_MS: u64 = 250;
pub const NOTICE_MS: u64 = 6_000;
pub const MAX_MATCHES: usize = 50;
pub const MAX_SEARCH: usize = 20;
pub const MAX_QUERY_CHARS: usize = 128;
pub struct ProgramFillService { /* host: Option<Arc<dyn Host>>, windows: Arc<dyn PickerWindows>, blocklist: Mutex<BlockList>, model: Mutex<PickerModel>, clock: Arc<dyn Fn() -> u64 + Send + Sync> */ }
impl ProgramFillService { pub fn new(host: Option<Arc<dyn Host>>, windows: Arc<dyn PickerWindows>, blocklist: BlockList, clock: Arc<dyn Fn() -> u64 + Send + Sync>) -> Self; }
impl Backend {
    pub fn program_fill_apply_settings(&self);
    pub fn program_fill_on_event(&self, event: HostEvent);
    pub fn program_fill_hotkey_pressed(&self);
    pub fn picker_state(&self) -> IpcResult<PickerState>;
    pub fn picker_chip_click(&self, r: PickerGenerationRequest) -> IpcResult<()>;
    pub fn picker_search(&self, r: PickerSearchRequest) -> IpcResult<Vec<PickerLogin>>;
    pub fn picker_choose(&self, r: PickerItemRequest) -> IpcResult<()>;
    pub fn picker_link_begin(&self, r: PickerItemRequest) -> IpcResult<LinkStep>;
    pub fn picker_link_confirm(&self, r: PickerGenerationRequest) -> IpcResult<LinkConfirmResult>;
    pub fn picker_block(&self, r: PickerGenerationRequest) -> IpcResult<()>;
    pub fn picker_dismiss(&self, r: PickerGenerationRequest) -> IpcResult<()>;
    pub fn picker_list_blurred(&self);
}
```

Rules (state machine; every change of what a picker shows increments `generation` and calls `windows.changed` for the shown window):
- `program_fill_apply_settings`: start the host when `program_fill` is on, `available`, the settings were verified in this run and Mosaic is not elevated; stop it, hide both windows and drop the model otherwise. Registered as an unlock hook and called by `update_settings`; the lock hook hides both windows and drops offer, link step and continuation (a pending unlock resume is set only while locked).
- `Field(offer)` while a fill runs → ignored. `block_key` in the block list → hide. Locked → chip with `locked: true` only for `Password` and `Username` roles. Unlocked → logins of every usable vault (not trashed): `match_bindings` per login, at most `MAX_MATCHES` matches sorted by title, `preselected = Some(0)` when exactly one; no match but `LinkAgain` → `link_again`; `Text` role → offered only with matches and `host.lone_text(token) == Ok(true)`; then `chip_rect` (monitor from `windows.monitor_at` at the field's centre) → `show_chip`; `None` placement → hide. `Gone` → hide the chip (the list stays while it is foreground). `Refused`/`NoField` from the hotkey → `Notice` (`Elevated` for `CaptureRefusal::Elevated`, else `NoField`) at `list_rect(near)`, shown without focus, hidden after `NOTICE_MS`; from focus events they only hide.
- `chip_click`: generation current; locked → `windows.focus_main()` and a resume `{ target, until: now + UNLOCK_RESUME_MS }`; unlocked → `list_rect` → `show_list(.., true)`, `list_shown_at = now`. The unlock hook with a live resume: `host.refocus(token)`, `host.capture_now()`; `Field` with the same `target` → open the list; else drop.
- Pick checks (`choose` and `link_confirm`), in order: generation current else `pickerStale`; for `choose`: mode `List`, the item is in `matches` else `invalidInput`, `now − list_shown_at ≥ PICK_MIN_DELAY_MS` else `pickerTooSoon`; then `windows.is_foreground(List)`, `windows.placed(List) == last placement`, `host.window_clear(list hwnd, placement) == Ok(true)`, each else `pickerStale`.
- `choose` then: read user name (the login's `username` text field) and password (`SecretString`) in Rust; `submit = settings.program_fill_submit && every matched URL submit_enabled && password non-empty`; hide chip and list; `host.fill(FillRequest { picked_at_ms: now, continuation: false, .. })`; `Done` with role `Text` → `host.arm_continuation` and keep `{ vault, item, token, picked_at, used: false }`; `NothingToFill` (empty user name and password) → nothing shown; `Aborted` → notice (`ModifiersHeld`, `AbortedCleared` when `cleared`, `NotResponding` for `Probe(Timeout)`, else `Aborted`); `Err(Timeout)` → `NotResponding`. The command returns `null` after the fill.
- `Continue { token }`: the stored continuation matches the token, is unused and younger than `CONTINUATION_TTL_MS` → mark used, read the password again, `host.fill(FillRequest { username: None, continuation: true, picked_at_ms: original, .. })`. Anything else ignores it.
- `search`: query trimmed, at most `MAX_QUERY_CHARS` chars else `invalidInput`; empty → `[]`; logins whose folded title, user name or URL host contains the folded query, at most `MAX_SEARCH`. `suggestions` (in the state when there is no match): `suggests(suggestion_terms(identity), title, hosts)`, at most `MAX_SUGGESTIONS`.
- `link_begin`: generation; mode `List`; the item is a non-deleted login in a writable vault else `invalidInput`; `propose_binding(identity, bound_exes)` where `bound_exes` are the exe names of all the user's `signed/` and `path/` bindings except those `superseded` for this identity; `Excluded` → `invalidInput`; opens the step with `shown_at = now`; `hash_change` for `LinkAgain(FileChanged)`: first 8 bytes hex of the old URL's `sha256` and the identity's `file_sha256`.
- `link_confirm`: the step exists; `now − shown_at < LINK_MIN_DELAY_MS` → `TooSoon`; the pick checks; `LinkConfirmation::confirm(now)`: `NeedAnother` → `confirmed = 1`, `NeedAnother`; `TooSoon` → `TooSoon`; `Done(url)` → `link::write` (one `update_item`: remove `superseded` URLs, add the new URL unless an equal one exists, with `set_program_display(display_for(identity))`) then the fill as in `choose` (without the 300 ms check); `Linked`.
- `block`: `block_key` + `display_for(..).name` into the block list; hide both. `dismiss`: hide the list, `host.refocus(token)`. `picker_list_blurred` (the list window lost focus): hide the list unless a fill runs.
- `picker_state` never contains a secret; the user name is the login's plain `username` field.

- [ ] **Step 1: Write the failing tests** (`program_fill_tests.rs`, with `FakeHost`, a `FakePickerWindows` recording calls, a fake clock and an unlocked in-memory backend holding logins "Slack" (bound `signed/slack.exe`), "Bank" (unbound, https) and "Tool" (bound `path/` with hash H1))

```rust
#[test] fn no_fill_without_a_pick() { /* 100 Field and Continue events, chip clicks, searches, link_begin: FakeHost has zero Fill calls */ }
#[test] fn picker_webview_never_receives_a_secret() {
    // the logins' passwords "pw-SLACK-0x5f…" etc.: serialize every PickerState, search answer, LinkStep, LinkConfirmResult and PickerEvent
    // produced by a full scripted session (offer, list, search, link, notice) to JSON: none contains any password
}
#[test] fn a_bound_program_gets_its_login_preselected() { /* Field(slack identity) -> chip; chip_click -> List; matches == [Slack]; preselected Some(0) */ }
#[test] fn a_pick_fills_with_the_login_and_the_submit_settings() {
    // choose after 300 ms -> one Fill { username Some("u"), password present, submit true, continuation false }
    // programFillSubmit off -> submit false; binding extra.submit false -> submit false
}
#[test] fn picks_too_soon_stale_or_outside_the_offer_are_refused() { /* < 300 ms -> pickerTooSoon; old generation -> pickerStale; Bank (not in matches) -> invalidInput */ }
#[test] fn a_moved_or_covered_list_refuses_picks_and_links() { /* placed != placement, not foreground, window_clear false -> pickerStale each, no Fill */ }
#[test] fn elevated_target_gets_no_chip_and_no_fill() { /* Refused{Elevated} from focus -> hidden; hotkey -> Notice Elevated, no Fill */ }
#[test] fn blocked_program_gets_no_chip() { /* picker_block then the same identity -> hidden; another unsigned file at the same path -> hidden */ }
#[test] fn a_lone_text_field_is_offered_only_for_a_linked_program_with_one_edit() { /* Text role: no match -> hidden; match and lone_text false -> hidden; true -> chip */ }
#[test] fn two_step_login_continues_once_in_the_same_session() {
    // Text pick -> Fill(username only); Continue{token} -> Fill(password, continuation true, picked_at original); second Continue -> nothing;
    // Continue after 60 s -> nothing; Continue with another token -> nothing
}
#[test] fn linking_a_signed_program_needs_one_guarded_confirmation() { /* link_begin Bank; confirm at +999 -> TooSoon; at +1000 -> Linked; Bank has a signed/ URL with extra.program; then a Fill */ }
#[test] fn linking_an_unsigned_program_needs_two_confirmations() { /* +1000 -> NeedAnother; +1500 -> TooSoon; +2000 -> Linked */ }
#[test] fn link_again_replaces_the_changed_binding() {
    // Tool identity with hash H2: state.link_again == FileChanged, hash_change prefixes of H1 and H2; link Tool -> Tool has exactly one path/ URL, with H2
}
#[test] fn the_link_flow_cannot_bind_excluded_programs() { /* Browser-class identity: no chip; a forged link_begin -> invalidInput */ }
#[test] fn lock_ends_offers_links_and_continuations() { /* lock during a link step -> hidden; link_confirm -> pickerStale; Continue -> nothing */ }
#[test] fn a_locked_chip_click_unlocks_then_reopens_only_the_same_field() { /* resume: same target -> List; other target -> hidden; after 60 s -> nothing */ }
#[test] fn offer_caps_matches_and_handles_many_bindings() { /* 5 000 logins, 120 bound to the program -> 50 matches sorted by title */ }
#[test] fn an_elevated_mosaic_does_not_watch_programs() { /* status reason mosaicElevated -> host.start never called */ }
#[test] fn a_host_timeout_shows_not_responding_and_writes_nothing_more() { /* FakeHost fill -> Err(Timeout) -> Notice NotResponding */ }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop program_fill`
Expected: compile errors.

- [ ] **Step 3: Implement** per the Interfaces and Rules. `offer.rs` holds the vault reads (matches, search, suggestions, the login's user name and password); `link.rs` the item write; `mod.rs` the model and the rect poller thread (runs only while the mode is `Chip`; a changed or missing rect hides the chip, two equal polls show it again at the new place). Add `picker.rs` samples and run `MOSAIC_UPDATE_FIXTURES=1 cargo test -p mosaic-desktop contract_fixtures`.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop program_fill contract`
Expected: PASS; clippy and fmt clean for `mosaic-desktop`.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/src-tauri Cargo.lock
git commit -m "feat(desktop): program fill offers, picks, links and two-step continuation (ADR-0164)"
```

---

### Task 9: Picker windows, capabilities and picker commands (runs anywhere; the window behaviour is checked in Task 13)

**Files:**
- Modify: `apps/desktop/src-tauri/src/program_fill/picker_windows.rs` (`TauriPickerWindows`), `src/lib.rs` (wiring: `WindowsHost` on Windows, `None` elsewhere; window events of `picker` → `picker_list_blurred`), `src/commands/names.rs`, `build.rs`, `src/commands/mod.rs`
- Create: `apps/desktop/src-tauri/src/commands/picker.rs`, `capabilities/picker.json`, `capabilities/chip.json`
- Modify: `apps/desktop/src-tauri/tauri.conf.json` (`"capabilities": ["main", "picker", "chip"]`), `src/tests.rs`, `src/redteam_surface_tests.rs`, `fuzz/fuzz_targets/ipc_requests.rs` (+ seeds `fuzz/seeds/ipc_requests/picker_*`), `docs/DESKTOP_IPC.md` (new "Picker windows" section)

**Interfaces:**
- Consumes: Task 8 `PickerWindows`, `Backend::picker_*`, `PickerEvent`; Task 5 `show_no_activate`; Task 6 `WindowsHost`.
- Produces:

```rust
// commands/names.rs
pub const CHIP_COMMANDS: &[&str] = &["picker_subscribe", "picker_state", "picker_chip_click"];
pub const LIST_COMMANDS: &[&str] = &["picker_subscribe", "picker_state", "picker_search", "picker_choose",
    "picker_link_begin", "picker_link_confirm", "picker_block", "picker_dismiss"];
// build.rs: app manifest commands = COMMANDS ++ (CHIP_COMMANDS ∪ LIST_COMMANDS, deduplicated)

// commands/picker.rs — every command takes `webview: tauri::Webview<R>` and refuses (invalidInput) a label other than
// "picker-chip" (CHIP_COMMANDS) or "picker" (LIST_COMMANDS)
pub const CHIP_LABEL: &str = "picker-chip";
pub const LIST_LABEL: &str = "picker";
#[tauri::command] pub async fn picker_subscribe<R: Runtime>(webview: Webview<R>, state: State<'_, AppState>, channel: Channel<PickerEvent>) -> IpcResult<()>;
// picker_state: null -> PickerState; picker_chip_click / picker_link_confirm / picker_block / picker_dismiss: PickerGenerationRequest;
// picker_search: PickerSearchRequest -> PickerLogin[]; picker_choose / picker_link_begin: PickerItemRequest

// picker_windows.rs
pub struct TauriPickerWindows<R: Runtime> { /* AppHandle, last placement per label, Channel per label */ }
impl<R: Runtime> PickerWindows for TauriPickerWindows<R> { /* … */ }
```

Window options (`WebviewWindowBuilder::new(app, label, WebviewUrl::App("chip.html" | "picker.html"))`, built on first use, hidden, destroyed on lock and when program fill stops): `.title("Mosaic").visible(false).decorations(false).resizable(false).skip_taskbar(true).always_on_top(true).shadow(false).content_protected(true)`, and `.focusable(false)` for the chip. Placement through `set_position` / `set_size` with `PhysicalPosition` / `PhysicalSize`; the chip and the notice are shown with `mosaic_winshim::show_no_activate(hwnd)`; the list with `show()` + `set_focus()`. `monitor_at` uses `AppHandle::monitor_from_point` (position, size, scale factor). `WebviewWindowBuilder` appears nowhere else in `src/` outside tests.

- [ ] **Step 1: Write the failing tests**

```rust
// tests.rs
#[test] fn picker_capabilities_grant_exactly_the_picker_commands() {
    // picker.json: windows ["picker"], permissions == allow-<LIST_COMMANDS>; chip.json: windows ["picker-chip"], == allow-<CHIP_COMMANDS>;
    // no "remote"; main.json grants no picker_* command; tauri.conf capabilities == ["main", "picker", "chip"]
}
// redteam_surface_tests.rs
#[test] fn picker_window_gets_only_picker_commands() {
    // mock windows "picker" and "picker-chip": app_status, list_items, reveal_secret, get_item, copy_to_clipboard, subscribe_events -> refused;
    // "picker-chip" -> picker_choose, picker_link_confirm, picker_search refused; "main" -> picker_state, picker_choose refused;
    // "popup" -> picker_state refused
}
#[test] fn picker_commands_check_the_window_label_in_rust_too() { /* Backend-level: a picker command handler called with label "main" -> invalidInput */ }
#[test] fn picker_windows_are_built_only_in_picker_windows_rs() { /* source scan of src/ like opener_has_one_call_site */ }
// extend remote_origins_and_other_webviews_get_no_ipc: picker commands from https://evil.example and from window "popup" refused
// extend the_build_has_no_devtools_remote_ipc_asset_protocol_or_webview_url_opener: still one configured window; capabilities list exact
// extend no_webview_navigates_away_from_the_app: chip.html and picker.html load; picker.html?x=https://evil.example stays local
```

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop picker capabilit surface`
Expected: FAIL / compile errors.

- [ ] **Step 3: Implement** the names split, `build.rs`, the capability files, the commands, `TauriPickerWindows` (Windows parts behind `#[cfg(windows)]`; elsewhere `NoPickerWindows`), the wiring in `lib.rs` (`with_program_fill(ProgramFillService::new(host, windows, BlockList::in_dir(&dir), monotonic_ms))`); fuzz requests and seeds; DESKTOP_IPC "Picker windows": the commands table, `PickerEvent`, "one channel per picker window, the latest `picker_subscribe` of a window wins", and §6: "picker windows receive no secret".

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop`; clippy and fmt for `mosaic-desktop`; `cargo check` of `apps/desktop/src-tauri/fuzz` (target-dir prefix).
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop docs/DESKTOP_IPC.md
git commit -m "feat(desktop): picker and chip windows with their own capabilities (ADR-0164)"
```

---

### Task 10: Program bindings in the item view and the editor guard (runs anywhere)

**Files:**
- Modify: `apps/desktop/src-tauri/src/contract/items.rs` (`UrlEntry.program`, `UrlChange::ProgramSubmit`), `src/commands/draft.rs`, the item detail builder (where `UrlEntry` is filled), `src/redteam_tests.rs` or `handler_tests.rs`, `fuzz/fuzz_targets/ipc_requests.rs` seed
- Modify: `apps/desktop/src/ipc/contract/items.ts`, mock items; `docs/DESKTOP_IPC.md` (`get_item` URL entry, `UrlChange` ops)

**Interfaces:**
- Consumes: phase 1 `ProgramUrl::parse`; Task 2 `program_display`, `submit_enabled`, `set_submit`; Task 8 `ProgramKind`.
- Produces:

```rust
pub struct ProgramBindingView { pub kind: ProgramKind, pub name: String, pub signer: Option<String>, pub path: Option<String>, pub submit: bool }
// UrlEntry gains: pub program: Option<ProgramBindingView>   // Some exactly for a parseable winapp:// URL
// UrlChange gains (op "programSubmit"): ProgramSubmit { element_id: String, submit: bool }
```

Rules: `name` = `program_display(url).name`, else the exe (signed), the package family (package) or the path's file name (path); `signer` from the display data; `path` only for `path/` URLs (decoded). `Add` and `Update` whose URL, trimmed, starts with `winapp:` in any ASCII case → `invalidInput` (whole draft refused). `Remove` of a binding is allowed (Unlink). `ProgramSubmit` on a non-`winapp://` element → `invalidInput`; else `set_submit` and `set_url` with the same URL (other members kept).

- [ ] **Step 1: Write the failing tests**

```rust
#[test] fn the_item_editor_cannot_add_or_edit_a_program_binding() {
    // add_item / update_item with UrlChange::Add { url: "winapp://signed/x.exe?signer=…" } and " WinApp://…" and Update to winapp -> invalidInput, item unchanged
}
#[test] fn a_program_binding_shows_its_program_and_submit_switch() { /* get_item: program Some { kind signed, name "Slack", signer "Slack Technologies", path None, submit true } */ }
#[test] fn program_submit_switch_round_trips_and_keeps_the_binding() { /* ProgramSubmit false -> submit false, url and extra.program unchanged; on an https URL -> invalidInput */ }
#[test] fn unlinking_removes_only_the_binding() { /* Remove of the winapp element -> other URLs stay */ }
```

- [ ] **Step 2: Run them to verify they fail**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop program_binding item_editor`
Expected: FAIL.

- [ ] **Step 3: Implement**, update fixtures (`MOSAIC_UPDATE_FIXTURES=1 … contract_fixtures`), TypeScript types and mock, DESKTOP_IPC.

- [ ] **Step 4: Run the tests**

Run: `CARGO_TARGET_DIR=C:/Users/Stephan/Documents/DEV/Mosaic/target CARGO_BUILD_JOBS=2 cargo test -p mosaic-desktop`; `pnpm --filter @mosaic/desktop typecheck`; `pnpm -w exec vitest run --project desktop src/ipc`.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop docs/DESKTOP_IPC.md
git commit -m "feat(desktop): program bindings in the item view; the editor cannot write winapp URLs (ADR-0164)"
```

---

### Task 11: Picker frontend and its strings (runs anywhere)

**Files:**
- Create: `apps/desktop/chip.html`, `apps/desktop/picker.html` (copies of `index.html` pointing at `/src/picker/chip.tsx` and `/src/picker/list.tsx`)
- Modify: `apps/desktop/vite.config.ts` (`build.rollupOptions.input = { main: "index.html", chip: "chip.html", picker: "picker.html" }`), `apps/desktop/src/ipc/tauri.ts` (export `call`)
- Create: `apps/desktop/src/picker/{chip.tsx,list.tsx,ipc.ts,usePickerState.ts,ChipView.tsx,PickerList.tsx,LinkStepView.tsx,NoticeView.tsx,picker.css}`, tests `apps/desktop/src/picker/{ChipView,PickerList,LinkStepView,NoticeView}.test.tsx`
- Create: `apps/desktop/src/ipc/contract/picker.ts`, `apps/desktop/src/ipc/mock/picker.ts`; Modify: `src/ipc/contract/{commands,index}.ts`
- Create: `packages/i18n/src/catalog/en/programFill.ts`, `packages/i18n/src/catalog/de/programFill.ts`; Modify: `packages/i18n/src/en.ts`, `de.ts`

**Interfaces:**
- Consumes: Task 8/9 contract (`PickerState`, `PickerLogin`, `LinkStep`, `LinkConfirmResult`, `PickerEvent`, the picker commands).
- Produces:

```ts
// src/picker/ipc.ts
export function createPickerIpc(): { call: MosaicIpc["call"]; subscribe(onChanged: (generation: number) => void): Promise<() => void> }; // picker_subscribe
// src/picker/usePickerState.ts
export function usePickerState(ipc: PickerIpc): PickerState | null; // picker_state on mount and on every "changed"
```

Behaviour: `ChipView`: one button, Mosaic mark (or a lock when `locked`), `aria-label` `programFill.chip.label` / `programFill.chip.locked`, click → `picker_chip_click`. `PickerList`: header `programFill.list.title`; matches as a listbox (`role="listbox"`, `aria-activedescendant`), Up/Down move, Enter or click → `picker_choose`, Esc → `picker_dismiss`, typing filters matches locally by title and user name; with no match: `programFill.list.none`, the `linkAgain` line, suggestions and a search box (each change → `picker_search` with the current generation); Tab order: list, search, "Link a login…", "Don't offer for this program" (→ `picker_block`); choosing in search or suggestions → `picker_link_begin`. `LinkStepView`: the program line by kind, the login line, the unsigned note, the look-alike warning, the hash change, one button (`confirm`, `confirmFill` when `submit` is false, `confirmUnsigned` for two confirmations, then `confirmAgain`), disabled for the first second after it appears (Rust enforces it anyway), → `picker_link_confirm`; `tooSoon` keeps the step. `NoticeView`: the notice text and Close (→ `picker_dismiss`). No `<input type="password">` anywhere; theme and language from `PickerState`.

**Strings** (`programFill.ts`; EN / DE; plain `{name}` placeholders):

| Key | EN | DE |
|---|---|---|
| `programFill.chip.label` | Fill with Mosaic | Mit Mosaic ausfüllen |
| `programFill.chip.locked` | Unlock Mosaic to fill | Entsperre Mosaic zum Ausfüllen |
| `programFill.list.title` | Logins for {program} | Logins für {program} |
| `programFill.list.none` | No login for {program} yet | Noch kein Login für {program} |
| `programFill.list.signerChanged` | This program's signer changed. Link again? | Der Herausgeber dieses Programms hat sich geändert. Erneut verknüpfen? |
| `programFill.list.fileChanged` | This program changed since you linked it. Link again? | Dieses Programm hat sich seit dem Verknüpfen geändert. Erneut verknüpfen? |
| `programFill.list.suggestions` | Suggestions | Vorschläge |
| `programFill.list.search` | Search logins | Logins suchen |
| `programFill.list.link` | Link a login… | Login verknüpfen … |
| `programFill.list.block` | Don't offer for this program | Für dieses Programm nicht mehr anbieten |
| `programFill.link.packaged` | {name} · Store app · Publisher {publisher} | {name} · Store-App · Herausgeber {publisher} |
| `programFill.link.packagedNoPublisher` | {name} · Store app | {name} · Store-App |
| `programFill.link.signed` | {exe} · Signed by {signer}, checked by Windows | {exe} · Signiert von {signer}, von Windows geprüft |
| `programFill.link.unsigned` | {exe} · Not signed · {path} | {exe} · Nicht signiert · {path} |
| `programFill.link.unsignedNote` | Mosaic will fill only this exact file. If it changes, Mosaic asks again. | Mosaic füllt nur genau diese Datei aus. Wenn sie sich ändert, fragt Mosaic erneut. |
| `programFill.link.lookalike` | Careful: this name looks like {similar}, which you linked before. | Vorsicht: Dieser Name sieht aus wie {similar}, das du schon verknüpft hast. |
| `programFill.link.hashChange` | Linked file {old}…, this file {new}… | Verknüpfte Datei {old} …, diese Datei {new} … |
| `programFill.link.login` | Login: {title} | Login: {title} |
| `programFill.link.confirm` | Link and sign in | Verknüpfen und anmelden |
| `programFill.link.confirmFill` | Link and fill | Verknüpfen und ausfüllen |
| `programFill.link.confirmUnsigned` | Link this unsigned program | Dieses unsignierte Programm verknüpfen |
| `programFill.link.confirmAgain` | Confirm again to link | Zum Verknüpfen erneut bestätigen |
| `programFill.notice.elevated` | Mosaic can't fill programs that run as administrator. Copy the password from Mosaic instead. | Mosaic kann keine Programme ausfüllen, die als Administrator laufen. Kopiere das Passwort stattdessen aus Mosaic. |
| `programFill.notice.noField` | Mosaic can't see a login field here. | Mosaic erkennt hier kein Anmeldefeld. |
| `programFill.notice.aborted` | Mosaic stopped filling because the window changed. Check the field. | Mosaic hat das Ausfüllen abgebrochen, weil sich das Fenster geändert hat. Prüf das Feld. |
| `programFill.notice.abortedCleared` | Mosaic stopped filling because the window changed and removed what it had typed. | Mosaic hat das Ausfüllen abgebrochen, weil sich das Fenster geändert hat, und das bereits Getippte entfernt. |
| `programFill.notice.modifiersHeld` | Release Shift, Ctrl, Alt and the Windows key, then pick the login again. | Lass Umschalt, Strg, Alt und die Windows-Taste los und wähle den Login dann noch einmal. |
| `programFill.notice.notResponding` | This program didn't answer in time. Nothing more was filled. | Dieses Programm hat nicht rechtzeitig reagiert. Es wurde nichts weiter ausgefüllt. |
| `programFill.notice.close` | Close | Schließen |

- [ ] **Step 1: Write the failing tests** (vitest + Testing Library, mock IPC)

```ts
it("picks the preselected single match with Enter", …)            // picker_choose called once with that item and the generation
it("moves with the arrow keys and closes with Escape", …)          // Escape -> picker_dismiss
it("filters matches as the user types", …)
it("searches with the current generation when nothing matches", …) // picker_search { generation, query }
it("shows link again with the reason", …)
it("disables the link button for one second and asks twice for unsigned programs", …) // fake timers; needAnother -> confirmAgain label
it("shows the look-alike warning and the hash change", …)
it("re-reads the state on every changed event", …)
it("renders no password input and no secret", …)                   // no input[type=password]; no "pw-" sample text in the DOM
it("has the same keys and placeholders in EN and DE, informal du", …) // i18n package test covers it; add programFill to the parity list if one exists
```

- [ ] **Step 2: Run them to verify they fail**

Run: `pnpm -w exec vitest run --project desktop src/picker`
Expected: FAIL.

- [ ] **Step 3: Implement** the pages, views, CSS through design tokens only (no inline styles under the CSP), the mock picker handlers (a `?mockPicker=list|chip|link|notice` option for the ux-reviewer).

- [ ] **Step 4: Run the tests**

Run: `pnpm -w exec vitest run --project desktop src/picker src/ipc`; `pnpm --filter @mosaic/i18n test`; `pnpm --filter @mosaic/desktop typecheck`; `pnpm --filter @mosaic/desktop lint`; `pnpm --filter @mosaic/desktop build` (three pages in `dist/`).
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop packages/i18n
git commit -m "feat(desktop): picker chip and list UI with EN and DE strings (ADR-0164)"
```

---

### Task 12: Main-window UI: settings, first-unlock card, item bindings (runs anywhere)

**Files:**
- Create: `apps/desktop/src/screens/ProgramFillSettings.tsx`, `ProgramFillSettings.test.tsx`, `apps/desktop/src/screens/ProgramFillCard.tsx`, `ProgramFillCard.test.tsx`
- Modify: `apps/desktop/src/screens/SettingsScreen.tsx` (a section after the browser section), the vault home or shell where post-unlock cards show (follow `BrowserConnectorBanner.tsx`), `apps/desktop/src/screens/vault/DetailFields.tsx` (+ its test), `packages/i18n/src/catalog/{en,de}/programFill.ts`

**Interfaces:**
- Consumes: Task 7 settings fields, `program_fill_status`, `program_fill_unblock`; Task 10 `UrlEntry.program`, `UrlChange::ProgramSubmit`.
- Produces: UI only.

Behaviour: Settings section shown on every OS; when `available` is false only the reason line. Toggle `programFill` (no password needed); `programFillSubmit` toggle (disabled while `programFill` is off); hotkey field (empty means none; validation errors from `update_settings` as today's shortcut field); blocked list with "Offer again" per entry. The card shows after an unlock when `available`, `programFill` is off and `programFillCardDismissed` is false: "Turn on" sets `programFill: true, programFillCardDismissed: true`; "Not now" sets only the dismissed flag. Item detail: a `winapp://` URL renders as a program row (kind line, signer or path, a "Sign in after filling" switch → `programSubmit`, "Unlink" → `Remove`), never as a link.

**Strings** (append to `programFill.ts`):

| Key | EN | DE |
|---|---|---|
| `programFill.settings.title` | Fill in programs | In Programmen ausfüllen |
| `programFill.settings.description` | Mosaic watches which field has focus in other programs on this device to offer your logins there. Nothing is stored or sent. | Mosaic beobachtet auf diesem Gerät, welches Feld in anderen Programmen den Fokus hat, um dir dort deine Logins anzubieten. Nichts wird gespeichert oder gesendet. |
| `programFill.settings.enable` | Offer logins in programs | Logins in Programmen anbieten |
| `programFill.settings.submit` | Sign in after filling | Nach dem Ausfüllen anmelden |
| `programFill.settings.hotkey` | Shortcut to open the list | Tastenkürzel für die Liste |
| `programFill.settings.hotkeyNone` | No shortcut | Kein Tastenkürzel |
| `programFill.settings.blocked` | Programs Mosaic doesn't offer logins for | Programme, für die Mosaic keine Logins anbietet |
| `programFill.settings.blockedNone` | None | Keine |
| `programFill.settings.unblock` | Offer again | Wieder anbieten |
| `programFill.settings.notWindows` | Only on Windows for now. | Vorerst nur unter Windows. |
| `programFill.settings.mosaicElevated` | Mosaic runs as administrator, so it doesn't fill programs. Start it normally. | Mosaic läuft als Administrator und füllt deshalb keine Programme aus. Starte es normal. |
| `programFill.settings.hostFailed` | Mosaic couldn't start watching programs. Restart Mosaic to try again. | Mosaic konnte das Beobachten von Programmen nicht starten. Starte Mosaic neu, um es noch einmal zu versuchen. |
| `programFill.card.title` | Fill logins in programs too | Logins auch in Programmen ausfüllen |
| `programFill.card.turnOn` | Turn on | Einschalten |
| `programFill.card.notNow` | Not now | Nicht jetzt |
| `programFill.item.program` | Program | Programm |
| `programFill.item.storeApp` | Store app | Store-App |
| `programFill.item.signedBy` | Signed by {signer} | Signiert von {signer} |
| `programFill.item.unsigned` | Not signed: {path} | Nicht signiert: {path} |
| `programFill.item.submit` | Sign in after filling | Nach dem Ausfüllen anmelden |
| `programFill.item.unlink` | Unlink | Verknüpfung lösen |

(The card's body reuses `programFill.settings.description`.)

- [ ] **Step 1: Write the failing tests**

```ts
it("turns program fill on without a password and disables submit while off", …)
it("shows only the reason when program fill is unavailable", …)
it("lists blocked programs and offers them again", …)              // program_fill_unblock { key }
it("shows the card once and remembers Not now", …)
it("renders a program binding as a row with its submit switch and Unlink", …) // programSubmit and remove ops sent; no anchor element
```

- [ ] **Step 2: Run them to verify they fail**

Run: `pnpm -w exec vitest run --project desktop src/screens/ProgramFill src/screens/vault/DetailFields`
Expected: FAIL.

- [ ] **Step 3: Implement** with the existing `SettingsSection`, toggle and banner components.

- [ ] **Step 4: Run the tests**

Run: the Step 2 command; `pnpm --filter @mosaic/i18n test`; `pnpm --filter @mosaic/desktop typecheck`; `pnpm --filter @mosaic/desktop lint`.
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop packages/i18n
git commit -m "feat(desktop): program fill settings, first-unlock card and program rows in the item view"
```

---

### Task 13: PROGRAM_FILL.md phase 2, the manual checklist on real programs, the gate (Windows machine)

**Files:**
- Modify: `docs/PROGRAM_FILL.md` (sections "Phase 2 scope", "How a fill runs", "Manual checklist, phase 2"), `docs/PROGRESS.md` (results), `docs/THREAT_MODEL.md` (only if a checklist result changes a row)

**Interfaces:**
- Consumes: everything above; ADR-0157 "Open" items (VS Code / heavy page re-measure, offline revocation).

- [ ] **Step 1: Write the checklist** in `docs/PROGRAM_FILL.md`, each item with its expected result; run with `pnpm tauri dev` (a debug build) and record each result (pass / fail / observed) in PROGRESS:
- [ ] **Slack** (Electron, signed): first sign-in screen; chip at the e-mail field; link a login (one confirmation, "Signed by Slack Technologies"); pick → e-mail typed, Enter; the password step gets the password via the continuation and Enter, once. Second visit: single match preselected, one Enter.
- [ ] **Microsoft Teams** (new Teams on WebView2) and its sign-in window: expected a chip on the sign-in fields (WebView2 content of the signed Teams host, owner decision 2026-11-15) and `Broker` for the AAD broker window; record which windows behave how.
- [ ] **A Java/Swing app** (for example a `jpackage` demo or JMeter): no chip; the hotkey shows "Mosaic can't see a login field here".
- [ ] **An Electron app with a plain login form** (for example a local test app from `%TEMP%` built with Electron, or Discord): chip, link, fill both fields, Enter; Chromium accessibility cost: VS Code with one folder open and a heavy page in Edge, measured with the ADR-0157 Step 4 method (closes the ADR's Open item; over 5 % CPU or 150 MB → D3's opt-in applies and is a bug for this task).
- [ ] **An elevated installer or any program run as administrator**: no chip; the hotkey shows the administrator notice; nothing typed.
- [ ] **Notepad and PowerShell ISE**: Notepad's edit gets no chip (no password field, no binding); ISE is `Host` (no chip); PowerShell's `Get-Credential` dialog (CredentialUIBroker) gets no chip.
- [ ] **A Win32 or WinForms dialog with user name and password** (WinSCP login or a test dialog in `%TEMP%`): user name by `SetValue` (read back), password typed; the probe harness password `Pässwörd-😀-12345678` arrives exactly.
- [ ] **A WPF login window** (`PasswordBox`): typed password correct.
- [ ] **Unsigned test program** renamed `Discord.exe` in `%TEMP%`: two confirmations at least 1 s apart, "Not signed" line with the path, look-alike warning when `discord.exe` is already linked; change one byte → "This program changed since you linked it", hash prefixes; link again → one binding.
- [ ] **Attacks** (spec §6.3 test programs in `%TEMP%`): field moves under the cursor on focus → abort notice, nothing in another field; foreground swap 100 ms after focus → abort; topmost click-through window over the field → no write (`Occluded`); a window dragged over Mosaic's list or the list moved by a script (`SetWindowPos` from PowerShell) → pick refused; typing while the user holds Ctrl → waits, then the modifiers notice.
- [ ] **Chip behaviour**: appearing does not take focus from the field (type on after it shows); hides on focus loss, on lock and while the window moves; a field on a second monitor at 150 %; chip and list never cover the field.
- [ ] **Locked vault**: lock chip; click → unlock in the main window → list opens for the same field.
- [ ] **Hung program** (phase 1 test window): no Mosaic hang; "didn't answer in time" notice after at most about 5 s.
- [ ] **Narrator**: the list reads titles and user names; keyboard-only pick works.
- [ ] **Offline revocation** (ADR-0157 Open): network off, a signed program's link still shows "checked by Windows" or is refused; record which.
- [ ] **Memory**: Mosaic's private bytes with program fill on and both picker windows created, versus off.

- [ ] **Step 2: Walk the checklist** once on the owner's Windows 10 machine. A failure is a bug in Tasks 2 to 12, fixed in their files with a regression test where the behaviour is testable on the fakes.

- [ ] **Step 3: Full gate once** (`FULL_RUN_OK=1`, the CLAUDE.md command list, with the target-dir prefix).

- [ ] **Step 4: Commit**

```bash
git add docs/PROGRAM_FILL.md docs/PROGRESS.md docs/THREAT_MODEL.md
git commit -m "docs(program-fill): phase 2 notes and the manual checklist results"
```

- [ ] **Step 5: Hand-off (orchestrator)**

Reviews, sequentially: Fable red team of the whole program fill path (shim changes, host, service, picker windows, against ADR-0164's spoofing list); crypto-reviewer on the secret path (vault → `SecretString` → UTF-16 → `INPUT`, `Debug` of `FillRequest`); ux-reviewer on the picker (mock mode, light/dark, DE/EN, WCAG 2.2 AA) and the settings section; test-engineer for uncovered paths in `fill.rs` and `program_fill`. Then PR.

---

## Spec coverage (self-review)

| Spec item | Task |
|---|---|
| G1, D2: offer at the field (chip and list), hotkey as fallback | 3, 8, 9, 11; hotkey 7, 8 |
| G2, D3: binding by package, exe plus signer, unsigned path plus hash with explicit confirmation | phase 1; linking 8 (two confirmations), 11 |
| G3, D5: one pick fills both fields and submits; single match preselected; never without a pick | 4, 8 (`no_fill_without_a_pick`, `a_bound_program_gets_its_login_preselected`) |
| G4: first-time link is one decision on the login | 8, 11 |
| A1 (fake program, window swap), A2, A8 | phase 1 matcher; 4 (checks per batch), 8, 9 (`picker_webview_never_receives_a_secret`, `picker_window_gets_only_picker_commands`) |
| §5.1.1 login field definition, user-name search, lone Edit; UIA timeouts; writing values (SetValue, typing rules, zeroized INPUT) | 3, 5, 6, 4 |
| §5.1.1 Chromium cost measurement | 13 (closes ADR-0157 Open) |
| §5.1.2 checks before each write, held handle, UIPI and elevated refusal, no `uiAccess`, excluded targets | phase 1 `verify`; 4, 6, 8; Mosaic elevated 8; brokers 2 |
| §5.1.4 chip (size, place, no focus, hide rules), list (keyboard, preselect), picker window capability, locked vault, linking texts and 1 s guard, block list, setting off by default with a card | 3, 8, 9, 11, 12, 7 |
| §5.1.4 "Type into this program" for undetected fields | not in phase 2 (ADR-0164) |
| §5.2.1 per-binding and global submit switches | 2 (`extra.submit`), 7 (`programFillSubmit`), 8, 10, 12 |
| §5.2.2 submit is Enter, no caption search | 4, ADR-0164 |
| §5.2.3 two-step continuation, 60 s, same window and process | 4 (`continues`), 6, 8 |
| §5.2.3 code step (TOTP) | not in phase 2 (ADR-0164) |
| §6.2 Part 1 rows (TOCTOU, overlay, mid-typing, foreign content, keyboard hooks, observation, E, D) and Part 2 program rows | 1 |
| §6.3 `no_fill_without_a_pick`, `picker_webview_never_receives_a_secret`, `picker_window_gets_only_picker_commands`, `elevated_target_gets_no_chip_and_no_fill`, `focus_change_between_pick_and_write_writes_nothing` (phase 1) and its typing form, `window_swap_or_foreground_change_aborts_typing`, `two_step_continuation_is_bound_to_window_and_time` (as `continuation_needs_same_window_and_time` + `two_step_login_continues_once_in_the_same_session`), Windows integration programs | 4, 8, 9, 13 |
| §7 Q1 (off, card), Q2 (refuse, checked on Teams), Q4 (no default hotkey) | 7, 12, 13, ADR-0164 |
| §8 C2 (D3 kept), C3 (manual checklist) | 13 |
| Constraints: i18n DE "du", design tokens, settings MAC, surface tests extended | 7, 9, 11, 12 |
