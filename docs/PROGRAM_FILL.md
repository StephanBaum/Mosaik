# Windows program fill

Design notes for phases 1 and 2 and the manual Windows checklists. Decisions: ADR-0157
(vetting, the `mosaic-winshim` exception), ADR-0158 (program identity and
binding), ADR-0163 (shim writes and input) and ADR-0164 (the desktop picker). Threats: `docs/THREAT_MODEL.md` §4.10, R177, R178.

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

Phase 1 wrote nothing into any program. Phase 2 (below) adds the writes.

## Phase 2 scope (ADR-0163, ADR-0164)

- **Offer at the field.** A focused login field of a linked or linkable program gets a
  small chip next to it (it never takes focus); a click opens the picker list. An
  optional hotkey (none by default) opens the list for the focused field.
- **One pick fills.** The user name by UI Automation `SetValue` (read back), the password
  typed as Unicode input, then Enter unless submit is off (per binding or globally). A
  second step (password on its own screen) continues once, within 60 s, in the same
  window and process, on its own only in a native window with one password field
  (see "How a fill runs"); else it is one more pick.
- **Linking** is one decision on the login (two, at least 1 s apart, for an unsigned
  program). WebView2 content counts as its signed host program (owner decision
  2026-11-15); an unsigned WebView2 host is refused.
- **Settings.** `programFill` is off by default (a card offers it after the first
  unlock), a block list, the hotkey, the submit switch.
- **Secrets never reach a webview.** The chip and list windows get titles and user
  names only; the password goes vault → `SecretString` → the host.

### How a fill runs

1. The host (an MTA worker, abandoned and restarted when UI Automation hangs) sees a
   focus change, waits about 150 ms and classifies the field.
2. The desktop matches the program identity against `winapp://` URLs and shows the chip.
3. A pick re-verifies everything (process, focus, foreground, image, field, occlusion,
   elevation) right before each write; any change aborts with a notice.
4. Held modifier keys delay typing up to 2 s, then the fill stops (`ModifiersHeld`). They
   are checked again before every typed batch, the Ctrl+A clean-up and Enter.
5. Two-step login: after the user name of a lone text field, the password field of the same
   window is written **without a new pick** only when the program is native (not Electron,
   CEF or WebView2, and the field's process owns the window) and that field is the only usable
   password edit of the window. Otherwise the list opens on that field with the first step's
   login preselected, and one Enter or click fills (ADR-0164 amendment, R207).
6. The host's events reach the desktop on their own dispatcher thread, so the desktop can call
   back into the host (`lone_text`, `fill`) from inside an event.

## Open items for phase 2

- **Renamed browsers** (done in phase 2: also by the version resource's product name). Exclusion is by file name, so `chrome.exe` copied to
  `x.exe` (still signed by Google) is `Ordinary` and could be linked. Phase 2
  must exclude by signer plus product name (version resource) as well, and keep
  the file-name lists.
- **Worker recovery** (done in phase 2, `windows/host.rs`). A dedicated MTA worker thread that is abandoned and
  respawned when a UI Automation call blocks (ADR-0157 D5). Phase 1 only has the
  deadlines. `WindowsProbe::new` must run on an MTA thread, not a UI (STA) thread.
- **Image path re-check** (still open after phase 2). `QueryFullProcessImageNameW` may report the name
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

### Phase 2: the app (desktop chip, list, hotkey; owner run on Windows)

Run `pnpm tauri dev` (a debug build) with program fill on in Settings and record each
result (pass / fail / what was seen) in `docs/PROGRESS.md`. A failure is a bug to fix with
a regression test where the fakes can show it.

- [ ] **Slack** (Electron, signed): chip at the e-mail field; link a login (one
      confirmation, "Signed by Slack Technologies"); pick → e-mail typed, Enter; the
      password step gets the password once through the continuation. Second visit: the
      single match is preselected, one Enter fills.
- [ ] **New Teams** (WebView2) and its sign-in window: a chip on the sign-in fields; the
      AAD broker window shows no chip (`Broker`). Record which windows behave how.
- [ ] **A Java/Swing app** (JMeter or a `jpackage` demo): no chip; the hotkey says
      "Mosaic can't see a login field here".
- [ ] **An Electron app with a plain login form** (Discord or a local test app): chip,
      link, both fields filled, Enter. Measure the Chromium accessibility cost with VS Code
      (one folder) and a heavy page in Edge (ADR-0157 Step 4 method); over 5 % CPU or
      150 MB is a bug.
- [ ] **A program run as administrator**: no chip; the hotkey shows the administrator
      notice; nothing typed.
- [ ] **Notepad, PowerShell ISE, `Get-Credential`**: no chip in any of them.
- [ ] **A Win32 or WinForms login dialog** (WinSCP): user name by `SetValue`, password
      typed; `Pässwörd-😀-12345678` arrives exactly.
- [ ] **A WPF login window** (`PasswordBox`): the typed password is correct.
- [ ] **An unsigned program renamed `Discord.exe` in `%TEMP%`**: two confirmations at
      least 1 s apart, "Not signed" with the path, look-alike warning when `discord.exe`
      is already linked; change one byte → "This program changed since you linked it";
      link again → still one binding.
- [ ] **Attacks** (spec §6.3 test programs in `%TEMP%`): field moves under the cursor on
      focus → abort; foreground swap 100 ms after focus → abort; a topmost click-through
      window over the field → no write (`Occluded`); a window dragged over the list or the
      list moved by `SetWindowPos` → pick refused; Ctrl held while typing → waits, then
      the modifiers notice.
- [ ] **Chip behaviour**: never takes focus (keep typing after it shows); hides on focus
      loss, on lock and while the window moves; a field on a second monitor at 150 %; chip
      and list never cover the field.
- [ ] **Locked vault**: the lock chip; click → unlock in the main window → the list opens
      for the same field.
- [ ] **Hung program** (phase 1 test window): Mosaic stays responsive; "didn't answer in
      time" after about 5 s at most.
- [ ] **Narrator**: the list reads titles and user names; a keyboard-only pick works.
- [ ] **Offline revocation**: network off, a signed program's link still shows "checked by
      Windows" or is refused; record which.
- [ ] **Memory**: Mosaic's private bytes with program fill on and both picker windows
      created, against off.
- [ ] Hotkey examples avoid `Ctrl+Alt+L`, the app's lock key.
