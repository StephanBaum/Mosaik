# Windows program fill

Design notes for phase 1 and the manual Windows checklist. Decisions: ADR-0157
(vetting, the `mosaic-winshim` exception) and ADR-0158 (program identity and
binding). Threats: `docs/THREAT_MODEL.md` §4.10, R154, R155.

## Phase 1 scope

What exists:

- **Identity.** `winapp://` URLs and their matching (`crates/origin/src/program.rs`).
  A program is its package family, or its trusted Authenticode signer plus exe
  name, or (unsigned) its exact path plus file hash. Never a window title.
- **Capture.** The focused edit control, its process (the window owner, or the
  element's process for a packaged app behind `ApplicationFrameHost.exe`), its
  signature, and for an unsigned image a capped file hash
  (`crates/program-fill/src/identity.rs`).
- **Binding policy.** What a login would be linked to, the warnings, and two
  confirmations at least one second apart for an unsigned program
  (`policy.rs`).
- **Re-verification.** Before every write: expiry, process still running (held
  handle), focus, foreground window, image path, field kind and state,
  occlusion at five points (`verify.rs`).
- **Windows probe and harness.** `crates/program-fill/src/windows` reads the
  real system through the shim; `examples/probe.rs` runs it by hand.

What waits for phase 2: the picker, the chip, the hotkey, typing, submit, the
`programFill` setting and focus events. Phase 1 writes nothing into any program.

## Crates

- `crates/origin` (`program.rs`): pure rules, `#![forbid(unsafe_code)]`.
- `crates/program-fill`: the model, policy and verification, pure and tested on
  every OS behind the `ProgramProbe` trait, plus the Windows probe
  (`#[cfg(windows)]`), `#![forbid(unsafe_code)]`.
- `crates/winshim` (`mosaic-winshim`): the only crate with `unsafe`.
  - `unsafe` only in modules named `sys_*`, one operation per block, each with a
    `// SAFETY:` comment (clippy `undocumented_unsafe_blocks` and
    `multiple_unsafe_ops_per_block` are deny).
  - The public API has no `unsafe fn`, no raw pointer, no `windows` type, no raw
    handle. Handles are RAII and closed on every path.
  - `tools/check-unsafe-confined.sh` (CI, lint step) keeps the exception here.
  - It builds as an empty crate on other targets.
  - A Fable red-team review of the whole crate and the crypto-reviewer's reading
    of each `unsafe` block are recorded in ADR-0157's review log before the first
    merge.
- Shim facts for later phases: `ElementFacts::pid` is the focused element's
  process, `WindowFacts::owner_pid` the window owner's, and `parent_pid` /
  `process_chain` give the parent chain (a hint: verify each parent's image and
  signer before trusting it). This is what WebView2 programs need.

## Manual checklist (Windows, before each release; C3)

Run each item with `cargo run -p mosaic-program-fill --example probe`, focus a
field within the 5 s wait, and record the result in `docs/PROGRESS.md`. The
automated tests of the shim and the probe run without a person
(`cargo test -p mosaic-winshim -p mosaic-program-fill`); the interactive ones are
`#[ignore]` (`cargo test -p mosaic-winshim -p mosaic-program-fill -- --ignored`
with a text field focused).

- [ ] Notepad (packaged on Windows 11, Win32 on 10): package binding or signed
      binding naming Microsoft.
- [ ] A signed third-party program: `signed/` URL, one confirmation, CN/O shown.
- [ ] An unsigned copy renamed `Discord.exe` in `%TEMP%`: `path/` URL, two
      confirmations, `Unsigned` warning; change one byte and capture again with
      the old URL as an argument: `link_again_needed` says `FileChanged`.
- [ ] A signed program re-signed with a second test certificate: `SignerChanged`.
- [ ] Chrome, Windows Terminal, `mstsc.exe`, the UAC prompt (not reachable):
      excluded or nothing captured.
- [ ] A program run as administrator: `Elevated`.
- [ ] Calculator (UWP): the identity is the app's package, not
      `ApplicationFrameHost.exe`.
- [ ] A WPF program (PowerShell ISE's own dialogs are excluded as `Host`; use
      another WPF program) and a WinForms program: `IsPassword` read correctly.
- [ ] During the 30 s check loop: click another window to see
      `FocusMoved`/`WindowChanged`; drag a topmost window of another program over
      the field to see `Occluded`; close the program to see `ProcessGone`;
      minimise to see `WindowChanged`.
- [ ] A program that moves its password field under the cursor on focus, and one
      that swaps its foreground window 100 ms after focus (spec §6.3 test
      programs, built in the temp dir): the loop reports a failure, never `Ok`,
      after the change.
- [ ] A hung program (the ADR-0157 Step 3 test window): the harness returns
      `ProbeFailed` within 5 s and stays responsive.
- [ ] A WebView2 or Electron program with a native window (phase 2 needs it):
      `ElementFacts::pid` differs from the window owner; the capture says
      `ForeignContent` for now, and `process_chain` of the element's process
      reaches the owner.
- [ ] Network off: an Authenticode-signed program still reports `Trusted` (the
      offline revocation case of ADR-0157 D4).
- [ ] VS Code (one folder open) and a heavy web page: idle CPU and private
      memory with and without a UI Automation client (ADR-0157 D3): at most 5 %
      CPU and 150 MB more, else Chromium and Electron programs become opt-in.
