# Windows program fill

Design notes for phase 1 and the manual Windows checklist. Decisions: ADR-0157
(vetting, the `mosaic-winshim` exception) and ADR-0158 (program identity and
binding). Threats: `docs/THREAT_MODEL.md` §4.10, R177, R178.

## Phase 1 scope

What exists:

- **Identity.** `winapp://` URLs and their matching (`crates/origin/src/program.rs`).
  A program is its trusted Authenticode signer plus exe name (the signer id is
  the leaf subject plus the root it chains to, and the root must be in the
  machine's `Root` or `AuthRoot` store), or, without a trusted signature, its
  package family, or (unsigned) its exact path plus file hash. Never a window
  title.
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

## Open items for phase 2

- **Renamed browsers.** Exclusion is by file name, so `chrome.exe` copied to
  `x.exe` (still signed by Google) is `Ordinary` and could be linked. Phase 2
  must exclude by signer plus product name (version resource) as well, and keep
  the file-name lists.
- **Worker recovery.** A dedicated MTA worker thread that is abandoned and
  respawned when a UI Automation call blocks (ADR-0157 D5). Phase 1 only has the
  deadlines. `WindowsProbe::new` must run on an MTA thread, not a UI (STA) thread.
- **Image path re-check.** `QueryFullProcessImageNameW` may report the name
  recorded at process start, so renaming a running exe and copying a signed
  program into the old path defeats the path re-check (R178, covered by R177's
  same-user assumption). Phase 2 compares file IDs against `GetMappedFileNameW`
  of the image base.
- **Saved signer name.** `link_again_needed` takes the signer's display name the
  caller stored with the link (the URL holds only a hash); phase 2 stores it.

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
`#[ignore]` and opt-in: set `MOSAIC_MANUAL_TESTS=1` and run (`cargo test -p mosaic-winshim -p mosaic-program-fill -- --ignored`
with a text field focused).

- [ ] Notepad (packaged on Windows 11, Win32 on 10): package binding or signed
      binding naming Microsoft.
- [ ] A signed third-party program: `signed/` URL, one confirmation, CN/O shown.
- [ ] An unsigned copy renamed `Discord.exe` in `%TEMP%`: `path/` URL, two
      confirmations, `Unsigned` warning; change one byte and capture again with
      the old URL as an argument: `link_again_needed` says `FileChanged`.
- [ ] Availability notes: a signature whose revocation lists are not cached can
      read `Untrusted` (`WTD_CACHE_ONLY_URL_RETRIEVAL`, no network wait); root
      changes (cross-signing, AuthRoot updates) can cause `SignerChanged`
      prompts for an unchanged program.
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
- [ ] A file signed under a root that is only in the per-user store
      (`HKCU\Software\Microsoft\SystemCertificates\Root`, added through the
      registry store provider): `MOSAIC_MANUAL_TESTS=1 cargo test -p mosaic-winshim -- --ignored user_only`
      with `MOSAIC_USER_ROOT_SIGNED` set to the file; the capture says
      `InvalidSignature`. Remove the certificate afterwards.
- [ ] A program started from a mapped network drive or a UNC path: `NetworkImage`.
- [ ] Network off: an Authenticode-signed program still reports `Trusted` (the
      offline revocation case of ADR-0157 D4).
- [ ] VS Code (one folder open) and a heavy web page: idle CPU and private
      memory with and without a UI Automation client (ADR-0157 D3): at most 5 %
      CPU and 150 MB more, else Chromium and Electron programs become opt-in.

### Phase 2: shim writes, input and the host (Windows; ADR-0163, ADR-0164)

Tasks 5 and 6 of the phase 2 plan. The automated tests run without a person; these
interactive ones are `#[ignore]` and need `MOSAIC_MANUAL_TESTS=1` plus the harness
`cargo run -p mosaic-program-fill --example probe -- --host` (prints each event, never a
value; `f` + Enter fills the latest offer with the user name `mosaic-test` and the password
`Pässwörd-😀-12345678`, submit off; `q` + Enter quits).

- [ ] A WinForms test dialog with a user name and a password field (a copy of a signed
      program under another name hosts it; a PowerShell form script was used in the
      development run): the password field gets focus, one `field` event with role
      `Password { username: Some(0) }` follows after about 150 ms, `f` fills both fields
      exactly (checked by eye, including the accents and the emoji) and prints
      `Done { submitted: false }`. Result of the 2026-11-15 run: passed.
- [ ] `cargo test -p mosaic-winshim -- --ignored --test-threads=1` with
      `MOSAIC_MANUAL_TESTS=1`: `focus_events_arrive_with_the_pid` (click into another program
      within 5 s), `set_value_and_value_round_trip_in_notepad_and_refuse_passwords` (focus an
      empty edit, then a password field), `send_unicode_types_into_the_focused_field` (focus an
      empty Notepad), `edits_near_and_in_window_list_a_dialogs_edits_in_order` (focus a field of
      a dialog with several edits).
- [ ] `cargo test -p mosaic-program-fill -- --ignored --test-threads=1`:
      `a_focused_password_field_produces_one_offer_after_the_debounce` (focus a password field
      within 10 s), `a_hung_program_times_out_and_the_host_recovers` (the phase 1 hung window).
- [ ] Held Ctrl+Shift while `f` is pressed (the hotkey case) and AltGr on a German keyboard:
      nothing is typed until the keys are released; held longer than 2 s the fill reports
      `ModifiersHeld`.
- [ ] A WebView2 program (new Teams, a WebView2 sample app) with a signed host: the harness
      reports a `field` for the host's exe and signer, never `msedgewebview2.exe`. A WebView2
      sample app that is unsigned reports `Refused(UnsignedWebViewHost)`.
- [ ] A copy of Chrome or Edge renamed to another file name: `Refused(Excluded(Browser))`.
