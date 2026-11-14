# Autofill everywhere: design

- Date: 2026-11-14
- Status: design, approved direction (owner conversation of 2026-11-14); ADRs follow per part
- Scope: four parts, built in order after v0.1.0 ships
- Grounded in: `main` at `df024dd` and `origin/feat/m10-mobile` (Android parts)

Places marked **Not verified** could not be checked in the code or against a library or
platform document while writing this. The builder checks each one before relying on it.

---

## 1. Intent and why

The owner, in their words: "currently when in a app or a programm i have to open up the
password manager type in the programm or just know where what i saved it under and then copy
the password paste copy the useremail or user name and paste... this is tediuous". On websites
a dropdown at the field works. In desktop programs and in Android apps (with Enpass) it does
not. The owner wants it to work "flawless and seamless".

Three further wishes:

1. "ideally we can select the credentials we want to login and it autologs us in and we dont
   see a loginscreen at all".
2. "with changing a password ... if we could just hit a button to renew the password this would
   be awesome".
3. A mail assistant that reads the user's mail over IMAP, only for the service Mosaic is acting
   on, to handle one-time codes, magic links and sign-up confirmations, and tags what it handled
   with "Mosaic" so mail programs can sort it. "the actions have to be dispatched by mosaic ...
   the only email it looks for is the email from the domain of this service ... any other thing
   is blocked ... within this timewindow the app listens for this specific email."

Why this matters for Mosaic: copy and paste is the path where people make mistakes (the wrong
item, a phishing window, the clipboard left full). A fill bound to a verified program identity
is safer than copy and paste, as long as the binding is strict. The same holds for codes: a code
Mosaic reads from a DKIM-verified mail and types into the form it started is safer than a user
who copies any code from any mail into any window.

---

## 2. Goals, non-goals and anti-goals

### 2.1 Goals

- G1. In a Windows program, Mosaic offers matching logins at the login field itself, as a small
  chip and a list. A hotkey exists as a fallback.
- G2. A login is bound to a program by a strong identity: the package identity, or the
  executable plus its code-signing signer. An unsigned program is bound by its exact path (and
  file hash, see §5.1.6) after an explicit confirmation.
- G3. One pick fills user name and password and submits, on websites and in programs. A single
  match is preselected, so the pick is one click or one Enter.
- G4. Linking a login to a program or an Android app on first use takes one decision on the
  login itself.
- G5. One button renews a website password: generate, open the change page, fill, submit, wait
  for a confirmation mail where the site sends one, then one confirmation click.
- G6. The mail assistant handles login codes, renewal codes and reset links, and sign-up
  verification links, for actions Mosaic started, for 15 minutes, from the service's verified
  domain only.
- G7. Everything runs on the device. The server sees nothing new (CRYPTO_SPEC §12 unchanged).

### 2.2 Non-goals (this design)

- Renew inside desktop programs or Android apps. Programs have no change page Mosaic can find.
- Tier 2 rotation connectors (ADR-0065 keeps them out; they need their own spec).
- Mosaic creating accounts by itself. Sign-up help starts only from the user's own sign-up.
- Reading, searching, sending, moving or deleting mail. The mail assistant reads new mail of one
  domain during one window, and only adds a tag.
- An ML classifier, a remote classifier, or any mail content leaving the device.
- A standing IMAP connection. The connection exists only while a window is open.
- Programs that run elevated, on the secure desktop, inside remote-desktop or VM windows, and
  generic interpreter hosts (`java.exe`, `javaw.exe`, `python.exe`, `node.exe`, `powershell.exe`,
  `cmd.exe`, `wscript.exe`, `mshta.exe`, `rundll32.exe`). See §5.1.
- Java Swing programs through the Java Access Bridge (v1). See §5.1.2.
- The mail assistant on Android and iOS (v1). Desktop first.
- macOS and Linux program fill in v1. §5.5 says what they need.

### 2.3 Anti-goals (any of these makes the work a failure, even if it works)

- A1. Mosaic fills a fake program: a look-alike executable, a same-named program from another
  signer, a program that swaps its window between the pick and the fill, or a browser that
  pretends to be a program.
- A2. Mosaic fills or submits without a pick. Fill-on-load (website or program) is the fake-login
  attack and stays forbidden (brief §4.1).
- A3. The mail assistant acts on a mail an attacker triggered: "approve this sign-in", "was this
  you", "confirm this change", a new e-mail address or phone number, recovery, or any mail whose
  sender domain was not verified.
- A4. The mail assistant follows a link to a host other than the login's exact origin.
- A5. Mailbox access, mail content, codes or OAuth tokens leave the device, or reach the server,
  the extension's storage, logs, crash reports or the webview.
- A6. The mail assistant reads mail outside a window Mosaic started, or keeps a connection open
  without one.
- A7. Any new telemetry, analytics, usage reporting or "phone home", including to Google or
  Microsoft beyond the OAuth and IMAP calls the user set up.
- A8. A secret enters a webview (the picker included), the clipboard (as a fill path), a log or an
  error message.
- A9. Renew replaces the current password without the user's confirmation, or loses any value
  that was ever an attempt's new password (ADR-0066 invariant, brief §5 tier 1).
- A10. An `unsafe` block in a Mosaic crate.

---

## 3. Constraints

From `CLAUDE.md`, the brief and accepted ADRs. They are not up for change in this work.

- **Never invent cryptography.** Only CRYPTO_SPEC §2 primitives. DKIM verification brings
  RSA-SHA256 and Ed25519-SHA256 verification as an interoperability use: CRYPTO_SPEC §2 gets a row
  and an ADR before any code (§5.4.5).
- **Secrets in zeroizing types.** Passwords, codes, mail bodies, OAuth access and refresh tokens:
  `SecretString` / zeroizing buffers, never `Clone`/`Debug`-printable, never logged, never in
  errors or panics.
- **`#![forbid(unsafe_code)]` in every crate**, the new ones included. OS calls that need `unsafe`
  go through vetted wrapper crates, each admitted by an ADR (§5.1.1).
- **Brief §4.1:** no fill without a trusted user gesture. **§4.2:** exact origin by default.
  **§4.3:** no cross-origin iframe fills. **§4.4:** clickjacking resistance; an inline web UI
  needs its own ADR (ADR-0035 chose none). **§4.5:** content scripts get one credential.
- **Brief §5 tier 1:** "commit the new password to the vault only after the user confirms
  success". ADR-0066: never delete a value that was ever an attempt's new password; never replace
  the current password without a confirmation in trusted UI.
- **CRYPTO_SPEC §12:** the server sees nothing new. No server change in any part.
- **No telemetry.** No third-party analytics. The README stays labeled pre-audit.
- **UI strings through `packages/i18n`** (DE with informal "du", and EN). **Styling through
  `packages/design-tokens`.** The picker window included.
- **Settings integrity (ADR-0030):** every new switch that turns on fills, submits or mail access
  lives under the settings MAC, so an outside edit of `settings.json` resets it to off.
- **Dependencies:** each new one gets a one-line justification in the commit message and a row in
  `docs/DEPENDENCIES.md`; crypto and TLS dependencies need an ADR; exact `=` pins; `cargo deny` and
  `cargo audit --deny warnings` pass.
- **Desktop surface tests stay meaningful:** `opener_has_one_call_site`,
  `the_handoff_opener_takes_only_a_canonical_web_origin`,
  `remote_origins_and_other_webviews_get_no_ipc` and
  `the_build_has_no_devtools_remote_ipc_asset_protocol_or_webview_url_opener`
  (`apps/desktop/src-tauri/src/redteam_surface_tests.rs`, `redteam_tests.rs`) are extended, not
  weakened, when the picker window and the OAuth opener arrive.

---

## 4. Decisions (decided by the owner, recorded as decided)

- D1. **Four parts, in this order:** (1) desktop program autofill, Windows first; (2) one pick
  logs you in, on websites and in programs; (3) one-button Renew, building on the guided change
  of M7b (ADR-0065, ADR-0066, ADR-0068, ADR-0069, ADR-0070); (4) the mail assistant. v0.1.0 ships
  before any of this.
- D2. **Part 1:** Mosaic offers matching logins at the program's login field itself, like the
  website dropdown. A hotkey alone is not enough; a hotkey may exist as a fallback.
- D3. **Program identity:** the executable identity plus its code-signing signer where it has one,
  never the window title alone. An unsigned program is bound by its exact path plus an explicit
  user confirmation.
- D4. **Android:** app fill exists (M10, ADR-0092: package plus a single current signing cert,
  "needs linking" for imported logins). Make the first-time linking one tap.
- D5. **Part 2:** after the user picks a login, Mosaic fills both fields and submits. It never
  fills or submits without a pick. A single match may appear preselected.
- D6. **Mail assistant, location:** on the device only, never through the Mosaic server. Nothing
  is added to CRYPTO_SPEC §12.
- D7. **Mail assistant, access:** IMAP. Gmail and Outlook through OAuth2 (XOAUTH2); other
  providers through IMAP with a password. Both are supported, because the owner's provider is not
  known.
- D8. **Mail assistant, window:** it listens only while an action Mosaic itself started is pending
  (sign-up, login code, renewal), only for 15 minutes, and only for mail from that login's service
  domain.
- D9. **Mail assistant, tag:** mail it handled gets the IMAP keyword `Mosaic` (and the Gmail label
  `Mosaic`).
- D10. **Security rules:**
  - (a) Trust the domain verified by DKIM/DMARC, using the provider's `Authentication-Results`
    header and a DKIM check of Mosaic's own where needed. Never trust the visible `From`.
  - (b) Act only on the expected kind of mail. Renewal: the reset or confirmation of Mosaic's own
    request. Login: the one-time code. Never act on "approve this sign-in", "was this you" or
    "confirm this change" mails, even inside the window.
  - (c) Follow a link only when its host is exactly the login's service origin (the exact-origin
    rules of `crates/origin`), with no redirect to another host.
  - (d) Every mail that does not fit is left untouched for the user.

Consequences of these decisions for accepted ADRs (each needs an amendment, written before the
code of its part):

| ADR | Today | Change |
|---|---|---|
| ADR-0065 | Out of scope: "automatic submission"; "Mosaic never submits" | Submission after a pick (part 2) and inside a Renew session (part 3) |
| ADR-0068 | "the form is never submitted" | A change form is submitted inside a Renew session |
| ADR-0035 | No inline UI; gestures: popup, `fill-login`, context menu | Gestures unchanged; the fill may submit. An inline web menu is a separate ADR (§7, Q9) |
| ADR-0092 | Linking: item editor or "No login for X" → pick → confirm | One-tap linking for strong candidates (§5.5.2) |
| ADR-0077 | One opener call site (`Backend::open_handoff`) | A second, equally narrow call site for OAuth consent (§5.4.6) |

---

## 5. Per-part design

### 5.0 What this reuses

| Need | Existing code | Notes |
|---|---|---|
| Match decisions | `crates/origin` (`fill_allowed`, `PageOrigin`, `registrable_domain`, `lookalike_of`, `idn_unicode`) | Pure, host-tested; the program matcher joins it as a module |
| App URL pattern | `crates/origin/src/app.rs` on m10 (`androidapp://<package>?cert=<b64url sha256>`, `app_fill_allowed`, browser packages never apps) | The Windows scheme copies its shape and its "browser is never an app" rule |
| Login URLs | `mosaic_core::model::LoginUrl` (`url`, `match_mode`, `allow_http`, `extra`), `url:<id>` collection keys | Program URLs are ordinary login URLs; unknown members are kept by older builds |
| Unknown item types | `ItemType::Other(String)` | A new `mail_account` type is kept and merged by older builds |
| Attempts and history | `crates/core/src/model/item/rotation.rs`, `Session::begin_rotation`, ADR-0066 | Renew is a rotation attempt |
| Change URL | `mosaic_insights::change_url::resolve` (ADR-0070) | Renew reuses it unchanged |
| Change form | `apps/extension/src/content/change.ts`, `src/background/rotation.ts`, ADR-0068 | Classification before any secret is requested |
| Web fill | `src/background/fill.ts` (`runGesture`), `src/content/fill.ts` (`setFieldValue`), `frames.ts` (`decideFrame`), `visibility.ts` (`isFillable`) | Part 2 adds a submit step after the fill |
| Desktop dialogs | `src-tauri/src/browser/prompts.rs` (1 s Allow guard, one at a time, expiry), `rotation_prompt.rs` | Linking confirmation and the Renew result dialog follow this pattern |
| Global hotkey | `src-tauri/src/shortcut.rs` (`tauri-plugin-global-shortcut`, Rust-registered) | The fallback hotkey is a second accelerator |
| TLS | `rustls` (`ring` provider) through `reqwest` blocking, `tls_config()`, TLS 1.3, OS trust store (ADR-0046); `tokio-rustls 0.26.6` already in `Cargo.lock` | IMAP over implicit TLS and the OAuth token calls reuse it; no new TLS stack |
| Opener | `src-tauri/src/sync/webauthn_flow.rs` (`UrlOpener`, `open::that_detached`) | OAuth consent is a second call site with a URL built from constants |
| Settings MAC | ADR-0030, `settings_integrity.rs` | Every new switch is under it |
| Windows APIs today | `windows =0.62.2` with WinRT features only (Hello, globalization); WinRT projections need no `unsafe` | UI Automation and process identity are COM/Win32 and do need `unsafe` → wrapper crates |

### 5.1 Part 1: desktop program autofill (Windows first)

#### 5.1.1 Detection and the forbid-unsafe rule

**Mechanism: UI Automation (UIA), client side.**

- Subscribe to `IUIAutomation::AddFocusChangedEventHandler` on a dedicated MTA worker thread,
  only while the vault is unlocked and the setting is on (or locked, to show "Unlock").
- On each focus change, read cached properties of the focused element in one request
  (`CacheRequest`): `ControlType`, `IsPassword`, `IsEnabled`, `IsOffscreen`, `IsKeyboardFocusable`,
  `BoundingRectangle`, `ProcessId`, `NativeWindowHandle`, `RuntimeId`, `FrameworkId`, and whether
  `ValuePattern` is available and `IsReadOnly`.
- A **login field** is an `Edit` control with `IsPassword = true`, or an `Edit` that comes before a
  password `Edit` in the same top-level window (the user-name field), or a lone `Edit` in a window
  whose identity has a linked login (the first step of a two-step login, §5.2.3).
- The user-name field is found by walking the UIA tree of the same top-level window: the nearest
  preceding enabled, on-screen, non-password `Edit` in tree order, inside the same container as the
  password field. No guess across containers. None found: fill the password only.
- The UIA worker never blocks the UI thread. Every UIA call has a timeout: UIA's connection and
  transaction timeouts (`IUIAutomation2`). A hung target must not hang Mosaic.
  **Not verified:** that the wrapper crate below exposes these timeouts.

**Coverage by framework** (to be confirmed by the builder's test matrix; each row is a claim
about platform behaviour that this design could not run):

| Framework | Focus events | `IsPassword` | Value write | Notes |
|---|---|---|---|---|
| Win32 `Edit` (`ES_PASSWORD`), WinForms | yes (MSAA/UIA proxy) | yes | `ValuePattern.SetValue` for the user name; password: **Not verified**, fall back to typing | Classic dialogs |
| WPF `TextBox` / `PasswordBox` | yes | yes for `PasswordBox` | `TextBox`: `SetValue`. `PasswordBox`: **Not verified** whether its peer allows `SetValue` | |
| UWP / WinUI (packaged) | yes | yes | **Not verified**; fall back to typing | Window owned by `ApplicationFrameHost.exe`; content in the app process (§5.1.3) |
| Electron, CEF, Chromium-based | yes, once Chromium's accessibility is active | yes for `<input type=password>` | **Not verified**: whether `SetValue` raises the page's `input` events; fall back to typing | Content is web content; see §5.1.3 |
| WebView2 inside an app | yes | yes | as Chromium | Content is provided by `msedgewebview2.exe`, not the host app; see §5.1.3 and Q2 |
| Qt (Widgets, Quick) | yes when the Qt accessibility plugin is on | **Not verified** | typing | |
| JavaFX | **Not verified** (JavaFX has a UIA bridge) | **Not verified** | typing | Only with its own signed launcher (`jpackage`) |
| Java Swing/AWT | no (Java Access Bridge, not UIA) | no | — | Not in v1; the hotkey fallback types into a bound launcher only |

Performance: when a UIA client is present, Chromium and Electron turn on their full
accessibility tree, which costs memory and CPU in every Chromium-based program.
**Not verified:** the size of that cost. The builder measures it (Slack, Discord, VS Code, Teams)
before the setting can default to on.

**Writing values.** Two methods, chosen per field, never the clipboard:

1. `ValuePattern.SetValue` when the pattern exists, the control is not read-only and the framework
   is on the tested list. For the user-name field, read `Value` back and compare (constant time,
   the value is not secret, but the comparison code is shared). UIA does not return a password
   field's value, so a password `SetValue` cannot be read back; it is used only for frameworks the
   test matrix proved.
2. Typing with `SendInput` and `KEYEVENTF_UNICODE` (one UTF-16 unit per key down/up pair), after
   `SetFocus` on the element and a full re-verification (§5.1.2). Rules:
   - Wait until the user has released every modifier (`GetAsyncKeyState` on Shift, Ctrl, Alt,
     Win), at most 2 s, or the hotkey's own Ctrl would turn characters into shortcuts.
   - Type in batches of at most 8 units. Re-verify focus and identity before each batch. A focus
     event during typing aborts at once.
   - `SendInput` returning fewer events than sent aborts.
   - On abort, if focus is still in the same field, select all and delete what was typed
     (Ctrl+A, Backspace). Otherwise do nothing more and tell the user.
   - The `INPUT` arrays hold secret characters: they live in a zeroizing buffer.
     **Not verified:** whether the wrapper copies them into a non-zeroized buffer; R40-style
     residue is recorded if so.

**Meeting `#![forbid(unsafe_code)]`.** UIA is COM; in the `windows` crate its interface methods
are `unsafe fn`, and `OpenProcess`, `QueryFullProcessImageNameW`, `GetPackageFamilyName`,
`WinVerifyTrust`, `SendInput`, `GetForegroundWindow` and `GetTokenInformation` are `unsafe` too. So:

- A new crate `crates/program-fill` (`mosaic-program-fill`), `#![forbid(unsafe_code)]`, holds the
  OS side behind a trait (`ProgramProbe`: focused element, identity, verify, write, submit) with a
  fake for tests. It depends only on vetted wrapper crates, which hold the `unsafe`.
- An ADR admits each wrapper. The ADR states, per crate: the pinned version, the maintainer and
  activity, the license (`deny.toml`), the number and kind of `unsafe` blocks and a reading of
  them (the crypto-reviewer's reading for ADR-0078 is the pattern), whether it reuses the
  workspace's `windows 0.62.2` (a second `windows` version doubles the build), and what it would
  take to fork it if it goes stale.
- Candidates, with what to verify (no version is claimed here):
  - **UI Automation:** `uiautomation` (crates.io, a safe wrapper over the `windows` crate's UIA
    bindings). Verify: focus-changed and property-changed event handlers are wrapped safely;
    `CacheRequest`, `ValuePattern`, `IsPassword`, `RuntimeId`, `ElementFromPoint`, `SetFocus` and
    `CompareElements` are exposed; the COM apartment it initialises; its `windows` version;
    license; open soundness issues. Fallback if it fails vetting: a small Mosaic-owned wrapper crate
    outside the workspace's forbid rule is **not** allowed by `CLAUDE.md`, so the fallback is to
    contribute the missing safe wrappers upstream, or stop.
  - **Process identity (image path, package identity, integrity level):** no single crate is known
    to wrap `QueryFullProcessImageNameW`, `GetPackageFamilyName` and `GetTokenInformation`
    together. Candidates to evaluate: `sysinfo` (exe path; large; verify it reads the path through
    a handle and not a snapshot that a PID reuse can confuse) and `windows-permissions` or similar
    for the token. **Not verified:** any crate that wraps `GetPackageFamilyName` safely.
  - **Authenticode:** `WinVerifyTrust` gives the OS's verdict (trust store, revocation, policy).
    Candidates: `codesign-verify` (wraps `WinVerifyTrust` on Windows and `SecStaticCode` on macOS;
    **Not verified** that it exposes the signer certificate's subject) and `authenticode` (pure-Rust
    PE signature parser; it does not check the chain, so it can only complement the OS verdict).
  - **Input:** `enigo` or `rdev` wrap `SendInput`. Verify: `KEYEVENTF_UNICODE` support, no global
    keyboard hook installed as a side effect, the return value of `SendInput` surfaced.
- If no candidate passes for a capability, that capability is not built. The design never asks
  for an `unsafe` exception.

#### 5.1.2 Re-verification before every write (TOCTOU) and elevated windows

At detection, Mosaic opens a handle to the target process (`PROCESS_QUERY_LIMITED_INFORMATION`)
and **keeps it open** until the pick session ends. An open handle keeps the PID from being reused.
Identity is computed once from that handle (§5.1.3) and cached in the session.

Before each write and before submit, all of these must hold, else the session ends and nothing
more is written:

1. `GetFocusedElement` returns an element equal to the target (`CompareElements`, and the same
   `RuntimeId`).
2. The focused element's `ProcessId` equals the session's PID, and the session's process handle
   reports the process as still running.
3. `GetForegroundWindow` is the session's top-level window, owned by the same PID, and it is not
   cloaked or minimised.
4. The image path read again through the held handle equals the path the identity was computed
   for.
5. For a password write: `IsPassword` is still true. For a user-name write: the element is still a
   non-password `Edit`.
6. The element is enabled, not off-screen, and its `BoundingRectangle` has a usable size (at least
   4 × 4 physical pixels, the extension's rule).
7. **Occlusion:** `ElementFromPoint` at the centre and at four inset points of the rectangle
   returns the target element or a descendant. A window of another process on top (a click-through
   overlay included, since `ElementFromPoint` works on the window stack) fails the check. Mosaic's
   own chip is placed outside the field so it never fails this check.
8. Less than 30 s have passed since the pick (60 s for a two-step continuation, §5.2.3).

What this does not stop (recorded as residuals): same-user malware that injects code into a signed
process after the check, or that synthesises input (THREAT_MODEL attacker P5, as R38 and R45).

**Elevated windows (UIPI).** Mosaic runs at medium integrity and never asks to run elevated.

- Read the target's integrity level from its token. If it is higher than Mosaic's, show no chip.
  The hotkey on such a window shows a Mosaic toast: "Mosaic can't fill programs that run as
  administrator. Copy the password from Mosaic instead." UIPI blocks `SendInput` into such windows
  anyway, and UIA access is limited.
- Mosaic never sets `uiAccess="true"` in its manifest. That would let it drive elevated UI, a power
  Mosaic does not need.
- The secure desktop (UAC prompt, Ctrl+Alt+Del, the lock screen) is not reachable. Nothing is
  offered there.

**Excluded targets** (no chip, no hotkey fill), decided in `crates/origin` from the identity:

- Browsers (Chrome, Edge, Firefox, Brave, Opera, Vivaldi, and the others on the extension's list),
  by signer and executable name. The extension serves them. Same rule as M10-G3-5 ("a browser
  package is never an app").
- Mosaic's own processes.
- Interpreter and script hosts (§2.2), whose signer says nothing about the program they run.
- `CredentialUIBroker.exe` and `consent.exe` (Windows credential and UAC prompts): one
  Microsoft-signed binary prompts for every target, so a binding would match all of them.
- `mstsc.exe` and other remote-desktop and VM clients: the field is in another machine.

#### 5.1.3 Program identity, its storage and its matching

**Identity probe** (`mosaic-program-fill`, from the held process handle):

1. Final image path (`QueryFullProcessImageNameW`, then resolved like `GetFinalPathNameByHandle`
   to remove 8.3 names, junctions and symlinks).
2. Package identity: the package family name (PFN) if the process is packaged (MSIX, UWP, Store).
3. Authenticode verdict of the image file from `WinVerifyTrust` (embedded or catalog signature,
   code-signing EKU, chain to a root in the Windows store). If it passes: the signer leaf
   certificate's subject as DER bytes, and its display form (CN and O) for the UI only.
4. For unsigned files only: SHA-256 of the file (read in one pass, cap 512 MiB; larger files
   cannot be bound unsigned).
5. Integrity level (§5.1.2) and the exclusion class (§5.1.2).

**Which process.** The identity is that of the process that owns the top-level window of the
focused element (`GetAncestor(GA_ROOT)` of its native window, `GetWindowThreadProcessId`).

- UWP/WinUI: the frame window belongs to `ApplicationFrameHost.exe`; the identity is the PFN of
  the process that owns the focused element's content window.
- If the focused element's own process differs from the window owner's: allowed only for the
  WebView2 case under Q2. Otherwise refused (`foreignContent`), like the Android rule that a
  third-party `WebView` inside an app never gets the app's login (ADR-0092 amendment N1).

**Storage: a URL-like scheme on the login, like `androidapp://`.** One `url:<id>` element per
binding (`LoginUrl`), so bindings sync, merge per element and survive in older builds as unknown
URLs (which never match web pages, because `saved_origin` does not parse them as http(s)).

```text
winapp://package/<PFN>
winapp://signed/<exe>?signer=<b64url(SHA-256(DER signer subject))>
winapp://path/<b64url(UTF-8 final path)>?sha256=<b64url(SHA-256(file))>
```

- `<PFN>`: the package family name, case-folded ASCII (`Name_PublisherId`). The publisher ID is a
  hash of the publisher's signing subject, so the PFN binds the signer too.
- `<exe>`: the file name of the image, lower-cased (Unicode simple case folding), percent-encoded,
  1 to 255 bytes, no path separators.
- `signer`: 43 base64url characters, 32 bytes. The subject DN, not the certificate: vendors renew
  certificates every one to three years, usually with the same subject. A subject change needs a
  new link.
- `path`: the final path as stored at link time. `sha256`: the file hash at link time.
- Display data (program name from the version resource, signer CN/O, path for unsigned) goes in
  the URL element's `extra` object (`program: {name, signer}`), shown in the item view, **never
  read by the matcher**. The version resource is chosen by the program's author and is untrusted.
- Canonical form only: the parser refuses anything else (other query members, padding, a missing
  member), as `AndroidAppUrl::parse` does.

**Matching** (`crates/origin/src/program.rs`, pure, host-tested, fuzzed like `app.rs`):

```rust
pub struct ProgramIdentity<'a> {
    pub package_family: Option<&'a str>,
    pub exe_name: &'a str,            // folded
    pub signer_subject_sha256: Option<[u8; 32]>, // Some only when WinVerifyTrust passed
    pub final_path: &'a str,
    pub file_sha256: Option<[u8; 32]>, // computed only for unsigned images
    pub class: ProgramClass,          // Ordinary | Browser | Host | Mosaic | Broker | Remote
}

pub fn program_fill_allowed(saved: &LoginUrl, observed: &ProgramIdentity<'_>)
    -> Result<MatchKind, Refusal>;
```

- Any `class` other than `Ordinary` → `NoMatch`.
- `package/<PFN>` matches only an equal PFN.
- `signed/<exe>?signer=` matches only a signed image (OS verdict passed) with an equal folded exe
  name **and** an equal signer subject hash. The exe name is required because one signer (for
  example Microsoft) signs many programs, including ones that show arbitrary content.
- `path/<p>?sha256=` matches only an **unsigned** image with an equal final path (case-insensitive
  ordinal) **and** an equal file hash. A signed image never matches a path binding (it should be
  bound by signer).
- Match → `MatchKind::ExactOrigin`. Nothing has an eTLD+1-like loose mode.
- Look-alikes are warnings in the link UI, never matches: an exe name or a signer CN close to one of
  another binding of the user (reuse `lookalike_of` ideas on these strings).
- When a bound program's signer changed (same exe, other signer), the picker shows "This program's
  signer changed. Link again?" and fills nothing.

Why the file hash on unsigned bindings, beyond D3: unsigned programs often live in user-writable
folders, where any same-user process can replace the file. The hash turns a silent swap into a
visible "This program changed since you linked it" prompt. The cost is a prompt after each update
of an unsigned program (Q3).

#### 5.1.4 Picker UX

**Where it appears.**

- A **chip**: a small Mosaic mark (24 × 24 logical px, per-monitor DPI aware) just outside the right
  edge of the focused login field, vertically centred, never covering the field (§5.1.2 item 7).
  It appears about 150 ms after focus lands on a login field, and goes away on focus loss, on
  window move or resize (UIA `BoundingRectangle` property-changed event on the target), and on
  lock.
- The chip window is topmost, has no taskbar entry and **must not take focus** when it appears, or
  it would steal the focus it reacts to. **Not verified:** whether Tauri 2.12 windows can be
  created non-focusable and non-activating (`focusable(false)` / `WS_EX_NOACTIVATE`). If not, the
  chip needs a wrapper-crate call, or it becomes a non-activating native layered window drawn from
  Rust (builder's call inside §5.1.1 vetting).
- A **click on the chip** opens the **list**: a small Mosaic window under the field (flipped above
  when there is no room), which does take focus.
- The list and chip are a **second Tauri window** (`picker`) with its own capability file
  (`capabilities/picker.json`) that grants only `picker_state`, `picker_search`, `picker_choose`,
  `picker_link`, `picker_dismiss`. It receives titles, user names and display identity data, never
  a secret. `remote_origins_and_other_webviews_get_no_ipc` and the core/plugin refusal tests are
  extended to the new label. It uses React, `packages/i18n` and `packages/design-tokens` like the
  main window.

**Keyboard.**

- In the list: Up/Down move, Enter picks, Esc closes and returns focus to the field, typing
  filters, Tab reaches "Link a login…" and "Don't offer for this program".
- A single match is preselected: Enter or one click picks it.
- **Fallback hotkey** (configurable, Rust-registered next to `shortcut.rs`; default left to the
  builder because common combinations collide with programs): opens the list for the field that
  has focus. On a field that is not detected (Swing, a custom control), it opens the list with
  "Type into this program" for a bound identity only; the fill then types the password only (no
  user name, no submit) after the full re-verification.
- After a pick, Mosaic gives focus back to the target (Mosaic is the foreground process at that
  moment, so `SetForegroundWindow` is permitted; **Not verified** for every window type), focuses
  the field with UIA `SetFocus`, and re-verifies.

**Locked vault.** The chip shows a lock. A click opens the main window's unlock (quick unlock with
Windows Hello where enrolled), then the list. The target is re-verified from scratch after the
unlock.

**First use: linking.**

- No match: the list says "No login for <program> yet", shows suggestions and a search box.
  Suggestions are logins whose title or URL host contains the exe stem, the package name or the
  signer's O/CN (case-folded). They are suggestions only; nothing links by itself.
- Choosing a login opens a **confirm step inside the list**:
  - Packaged: "<display name> · Store app · Publisher <publisher>".
  - Signed: "<exe> · Signed by <CN/O>, checked by Windows".
  - Unsigned: "<exe> · Not signed · <full path>", plus: "Mosaic will fill only this exact file.
    If it changes, Mosaic asks again." and a look-alike warning when one applies.
- One button, "Link and sign in", with the 1 s guard of the desktop prompts (`prompts.rs`): a click
  within one second of the step appearing is ignored. For unsigned programs the button reads
  "Link this unsigned program" and the step needs a second, separate confirmation (D3: explicit
  confirmation).
- Linking adds one `url:<id>` with the `winapp://` URL to the login through the normal item edit
  (it syncs), then the fill runs.
- "Don't offer for this program" stores the identity in a local, unsynced deny list (like the
  extension's `neverSave:`), shown and removable in Settings.

**Setting.** `programFill` (off | on, under the settings MAC). Off by default until the user turns
it on once, from a card after the first unlock that explains what Mosaic watches (which field has
focus in other programs, on this device only, nothing stored). See Q1.

### 5.2 Part 2: one pick logs you in

#### 5.2.1 The rule

A pick is: a click or Enter on a login in the program list, a click in the extension popup, the
`fill-login` command or the context-menu item. One pick authorises, for one login and one target
(one program identity and window, or one tab, frame, document and exact origin): fill the user name,
fill the password, submit once, and the two-step continuation of §5.2.3. Nothing else. No pick,
nothing: the brief §4.1 rule stays exactly as it is.

A per-login switch "Sign in automatically" (default on, stored on the login, synced) and a global
switch (default on, under the settings MAC) turn the submit step off. In a program the switch is
per binding (`extra.submit = false` on the `url:<id>`), because one login can be bound to several
programs.

#### 5.2.2 Programs: what "submit" means

- Submit is **Enter**, typed with `SendInput` into the password field after the full re-verification
  (§5.1.2). Enter triggers the default button in Win32 dialogs, WPF `IsDefault` buttons and web
  forms.
- Mosaic does not search for a button by its name or caption. Captions are chosen by the program,
  localised, and spoofable.
- If the binding's switch is off, Mosaic fills and leaves focus on the password field.
- Option for later (not v1): UIA `InvokePattern` on the button whose MSAA state includes
  `STATE_SYSTEM_DEFAULT` (`LegacyIAccessiblePattern`). **Not verified** across frameworks.

#### 5.2.3 Two-step logins and the code step

A **pick session** lasts at most 60 s after the pick and belongs to one process handle, one
top-level window (or one tab and exact origin on the web), and one login.

- User name first, password later: if only a user-name field is present, fill it and submit. If a
  password field then gets focus in the same window of the same process within the session, fill
  the password and submit, once, after the full re-verification. No second pick is needed: the
  pick named this login for this window.
- Code step: if a field that is not a password field gets focus in the same window within the
  session after the password submit, and the login has a TOTP, fill the current code and submit
  once. If the login expects a mail code (part 4), the code goes there when it arrives.
- Anything else ends the session: another window, another process, the session time, a lock.

#### 5.2.4 Websites

Changes in `apps/extension`, keeping ADR-0035's gesture list:

- **Popup:** with one match, the primary action is "Sign in" (fill and submit), preselected; Enter
  picks it. With several, each row's primary action is "Sign in", a secondary action "Fill only".
- **`fill-login` command:** with exactly one non-blocked match: fill and submit. With several:
  today it fills the first match. A shortcut with several matches is not a pick of one login, so it
  must not submit. It fills only (as today), or opens the popup (Q8).
- **Context menu:** "Fill and sign in" next to "Fill login".
- **Submit algorithm** (content script, after the fill, deterministic, no text heuristics):
  1. Re-detect the form at submit time. If the fields or their identities differ from the ones
     just filled (the M4 rule in `fill.ts`), do not submit.
  2. The password field's form owner must exist. Its resolved `action` must be the frame's own
     origin (or absent). A form that posts credentials to another origin is filled, not submitted.
  3. The **default button** in the HTML sense: the first submit button of the form in tree order.
     It must pass `isFillable` (visible, on top at its points, not tiny, not transparent) and be
     enabled. Then call its `click()`, which is what implicit submission does.
  4. No submit button: `form.requestSubmit()` (it runs validation and fires `submit`, like a
     person pressing Enter).
  5. No form owner (some single-page apps): do not submit. Leave focus on the password field and
     report "filled, not submitted".
  6. At most once per pick, within 2 s of the fill.
- **Code step** on the web: within the session, the same tab and the same exact origin, a code
  field filled with the TOTP or the mail code. **Not verified:** whether `activeTab` access survives
  the same-origin navigation that a submit causes in Chrome, Edge and Firefox. If it does not, the
  popup shows "Fill code" as a one-click action, which is a new gesture and always works.
- Blocked: everything the M4 hostile-page suite blocks stays blocked (`autofill-on-load`,
  synthetic events, cross-origin frames, hidden fields, navigation after the probe).

An **inline web dropdown** (the website experience the owner described) is not part of part 2's
core. Brief §4.4 allows it only with its own ADR and red-team pass; ADR-0035 deferred it. See Q9.

### 5.3 Part 3: one-button Renew (websites)

Renew is a rotation attempt (ADR-0066) driven through to the end. It never bypasses the attempt
model: the new password is in the item before anything reaches the site.

#### 5.3.1 Where the button is

- Desktop: item view of a login with an `https` URL, and each row of the rotate-now queue
  (ADR-0069), next to "Change password".
- Extension popup on a tab whose exact origin matches the login: "Renew password".
- Not for logins without an `https` URL, not in shared vaults where the user is a viewer, and not
  while another attempt of the same item is open (the result of that one comes first).

#### 5.3.2 End-to-end flow

1. **Begin.** The click is the pick for this login and its origin. The desktop starts an attempt:
   generate a password with the site's rules (generator, `password-rules.json`), store it in the
   item (`Session::begin_rotation`, committed locally before it returns, ADR-0066).
2. **Open the change page.** URL from `change_url::resolve` (ADR-0070: stored, quirks, well-known).
   - From the popup: the extension navigates the current tab with `tabs.update`, under ADR-0070's
     existing rules (https, the tab origin's site).
   - From the desktop: the desktop sends a new event over the paired channel (`renewOpen {attemptId,
     changeUrl}`, NATIVE_MESSAGING §5.1 has events) and the extension opens a tab. **Not verified:**
     a tab the extension opens gets no `activeTab` grant, so the extension cannot inject into it
     without host access. The user then clicks the toolbar button once ("Continue renewal"), which
     grants `activeTab`. Users who turned on save capture already granted host access and skip this
     click. See Q5.
3. **Not signed in?** If the page shows a login form instead of a change form (the probe's
   detection), Renew signs in first with the current password (part 2 rules: the Renew click is the
   pick for this login and this exact origin), then returns to the change URL once.
4. **Classify** the change form (ADR-0068: `currentAndNew`, `newOnly`, `ambiguous`, `none`). Only
   `currentAndNew` or `newOnly` continue. Origin: exact and https (the eTLD+1 and HTTP overrides do
   not apply, ADR-0068).
5. **Fill and submit.** `fillChange` (one message, two values of one item, one frame and document),
   marks the attempt delivered, then submits with the §5.2.4 algorithm. This is the ADR-0068
   amendment.
6. **Mail step, if the site asks.** If after submit the page shows a code field on the same origin,
   or the site's flow is known to send a confirmation link (Q6), Mosaic opens a mail window (§5.4)
   of kind `RenewCode` or `RenewLink` for the login's service domain. The code goes into the code
   field (same tab, exact origin); a link is opened in the same tab only under rule (c).
7. **Result.** The desktop's result dialog (ADR-0068: 1 s guard, 300 s timeout) asks "Did the
   change work?". Mosaic preselects an answer from signals, which only choose the default button:
   - the change form is gone and the tab is still on the exact origin, with no change form or
     password field shown again;
   - a verified "your password was changed" mail from the service domain arrived in the window
     (informational: it is shown, never acted on, rule (b));
   - the save-capture hook saw the new password submitted on a login form of the exact origin
     (ADR-0068 review round).
   One click confirms (brief §5 tier 1, ADR-0066). "It worked": the new value becomes the password,
   the old one goes to the history (`replaced`). "It didn't work": the new value goes to the history
   (`unconfirmed`). "Not sure": the attempt stays open, and "Fill new password to sign in"
   (`fillPending`) keeps working.

#### 5.3.3 When the site's form differs

Renew stops automating, and says why, whenever any of these happens. The attempt stays open, so
nothing is lost. The fallback is today's guided change (M7b), with the same attempt:

- the change URL lands on another host, or on SSO at another registrable domain;
- the form is `ambiguous` or `none`, or changes shape between probe and fill;
- extra fields Mosaic cannot fill: security questions, a captcha, a second factor that is neither a
  TOTP of this login nor a mail code;
- the site reports an error (the change form shows again after submit);
- no mail fits within 15 minutes.

The message: "Mosaic couldn't finish this. Your new password is saved in the change. Finish by
hand:" with "Fill change form", "Copy new password" and "Open change page".

#### 5.3.4 Rollback through the history

Unchanged from ADR-0066, which is the point: every value that was ever an attempt's new password
is kept (attempt, then history), and **Restore** in the item's history makes any entry the password
again. If the site applied the change but the user answered "It didn't work", the new value is in
the history as `unconfirmed` and "Restore" brings it back. R100's limits (32 entries, 30 days for
unconfirmed values) apply and are shown in the result dialog.

#### 5.3.5 Reset by mail (later step)

When a user cannot sign in (old password unknown), a reset flow is the only route: request a reset,
receive a link, set a new password. v1 supports it only when the user starts it on the site's reset
page from the popup ("Reset with Mosaic"): Mosaic starts an attempt and a `ResetLink` window, opens
the link under rule (c), and fills the new-password form as a `newOnly` change form. Mosaic does not
find or fill reset-request forms by itself. See Q6.

### 5.4 Part 4: the mail assistant

#### 5.4.1 Module boundaries

- **New crate `crates/mail` (`mosaic-mail`)**, `#![forbid(unsafe_code)]`, no UI, no vault access:
  - `auth`: DMARC-aligned sender domain from `Authentication-Results` or Mosaic's own DKIM check.
  - `classify`: the rule-based classifier (§5.4.7).
  - `extract`: code and link extraction (§5.4.8).
  - `window`: the expectation state machine (§5.4.9), with an injected clock.
  - `imap`: the session (connect, authenticate, select, IDLE or poll, fetch, store keyword), over a
    stream it is handed.
  - Pure parts are host-tested and fuzzed (MIME, header and classifier inputs are attacker-chosen).
- **`crates/origin`** gives the domain rules (`registrable_domain`, exact-origin checks for links).
- **`crates/sync`** gives the TLS config and the fixed-host HTTP client for OAuth token calls (the
  `RangeClient` pattern of ADR-0067).
- **Desktop** `src-tauri/src/mail/`: the service that holds expectations, runs the IMAP session on
  the blocking pool or Tokio, reads the account item from the vault, and emits events
  (`mailWaiting`, `mailHandled`, `mailLeftForYou`, `mailWindowEnded`) to the webview with no mail
  content. Codes go from the service straight to the filler (extension `fill` path or the program
  filler), never through the webview.
- **Extension:** no mail code. It only receives a code to fill, as it receives a TOTP today.
- **Server:** no change.

#### 5.4.2 Libraries to evaluate

Each needs a one-line justification; the TLS and crypto ones need an ADR.

| Need | Candidates | What to verify |
|---|---|---|
| IMAP protocol | `async-imap` (Tokio-compatible; IDLE; `AUTHENTICATE` with a custom authenticator for XOAUTH2); `imap-codec` with `imap-next` (sans-I/O, fuzzed parser) | Maintenance; parser robustness on hostile servers (fuzzing); IDLE; `UID STORE`, `PERMANENTFLAGS`, Gmail `X-GM-EXT-1`; stream-agnostic so `tokio-rustls` (already in the tree) supplies TLS; `unsafe` count; license |
| MIME parsing | `mail-parser` | Fuzzing history; limits (depth, parts, size); HTML-to-text; encoded-word and charset handling; `unsafe` count |
| DKIM | `mail-auth` (DKIM, ARC, DMARC) | Which RSA backend it uses: `rsa` (RustCrypto) carries RUSTSEC-2023-0071, which concerns private-key operations, not verification, but `cargo audit --deny warnings` flags it and needs a documented ignore; a `ring`-only API is banned outside `rustls` (CRYPTO_SPEC §2) and would need a spec change. Its DNS resolver dependency (`hickory-resolver` or a pluggable trait) |
| DNS (own DKIM check only) | `hickory-resolver`, or a resolver trait over the OS resolver | New network dependency; queries go to the OS-configured resolver; no DoH to a third party |
| OAuth2 | `oauth2` crate with Mosaic's own HTTP client adapter (no `reqwest` async, no extra TLS) | PKCE S256 and state handling; that it can run on the existing blocking `reqwest`; or write the two requests by hand against RFC 6749/7636/8252, which is protocol code, not new cryptography (Q7) |
| Loopback redirect listener | `std::net::TcpListener` on `127.0.0.1:0` | No new dependency |

**Not verified:** the current versions, maintainers and `unsafe` counts of all of these.

#### 5.4.3 Connection and TLS

- Implicit TLS on port 993 only. STARTTLS on 143 is refused (a downgrade by stripping the
  capability is the classic attack).
- TLS through the existing `rustls` config: TLS 1.3, OS trust store (ADR-0046). Some IMAP servers
  may support only TLS 1.2 (Q10).
- The connection exists only while at least one window is open: connect at the first window,
  `LOGOUT` when the last one ends. No connection while locked: a lock ends every window.
- One connection per account. `SELECT INBOX`. Record `UIDVALIDITY` and `UIDNEXT` at window start;
  only mail with a UID at or above that `UIDNEXT` is ever looked at. A `UIDVALIDITY` change ends the
  window with "left for you".
- IDLE (RFC 2177), re-issued at least every 9 minutes. Without IDLE, poll with `NOOP` every 10 s.
- New mail: `UID FETCH <range> (UID INTERNALDATE RFC822.SIZE BODY.PEEK[HEADER.FIELDS (FROM DATE
  SUBJECT MESSAGE-ID DKIM-SIGNATURE AUTHENTICATION-RESULTS)])`. `BODY.PEEK` never sets `\Seen`.
- The `From` domain prefilters (untrusted, only to skip mail). Candidates get `BODY.PEEK[]` if
  `RFC822.SIZE` is at most 2 MiB (larger: left for the user).
- Mail bytes go into zeroizing buffers, are parsed, classified and dropped. Parser-internal
  allocations are not zeroized (residual, like R40).

#### 5.4.4 Credential storage: the mail account item

- A new item type `mail_account` (`ItemType::Other("mail_account")` on older builds: kept, merged,
  shown as unsupported).
- Text fields: `mail.address`, `mail.imap_host`, `mail.imap_port`, `mail.auth`
  (`password` | `oauth_google` | `oauth_microsoft`), `mail.authserv_id` (see §5.4.5).
- Secret fields: `mail.password` (an IMAP password or app password), `mail.oauth_refresh_token`. No
  history for the refresh token (`keeps_history` false).
- Access tokens live in memory only (`SecretString`), dropped on lock and at the end of a window.
- Refused in shared vaults: sharing the item would share mailbox access. Allowed in the personal
  vault and hardware-protected vaults.
- The item never matches a page or a program (it has no `url:` elements) and the extension never
  sees it (`listForOrigin` and `search` skip the type).
- Per-device tokens versus one synced token: Q11.

#### 5.4.5 Sender verification (rule (a))

A mail's **verified domain** is the DKIM signing domain `d=` of a passing signature, aligned with the
`From` header domain (DMARC relaxed alignment: equal registrable domains, `crates/origin`
`registrable_domain`). Both must hold. A pass for `d=attacker.example` with `From: github.com` is
not a pass.

Source of the DKIM verdict:

1. **Known providers** (Gmail, Outlook.com and Microsoft 365): the provider adds an
   `Authentication-Results` header on receipt. Trust only the topmost such header, and only when its
   `authserv-id` equals the account's `mail.authserv_id` preset (for Gmail `mx.google.com`). A
   header the sender wrote sits below the provider's and is ignored.
   **Not verified:** that Microsoft's header carries an `authserv-id` at all. If it does not, the
   Microsoft preset uses step 2.
2. **Other providers, or a missing or unparseable provider header:** Mosaic verifies DKIM itself
   (`mail-auth`), with the key fetched over DNS. Accepted algorithms: `rsa-sha256` with keys of
   1024 to 4096 bits (RFC 8301) and `ed25519-sha256` (RFC 8463). `rsa-sha1` is refused. The `l=`
   body-length tag is refused (it lets an attacker append content under a valid signature).
3. ARC is not trusted (it vouches for forwarders, which is not this use).

This adds a CRYPTO_SPEC §2 "interoperability only" row (DKIM verification, RFC 6376, 8301, 8463,
7489) and an ADR, before the code.

Domain fit: the verified domain's registrable domain must equal the registrable domain of one of
the login's `https` URLs, or exactly equal a sender domain the user added on that login (an
optional `mail.sender:<id>` element, for services that mail from another brand domain; Q12).

#### 5.4.6 OAuth2 for Gmail and Outlook

- Authorization code with PKCE (S256) and a loopback redirect `http://127.0.0.1:<port>/`
  (RFC 8252 §7.3). `state` and the verifier come from the OS CSPRNG (`getrandom`, as elsewhere).
- The consent page opens in the system browser through a **second opener call site** that builds
  the URL in Rust from constants (authorisation endpoint, client ID, scope, redirect, state,
  challenge) and takes nothing from the webview. `opener_has_one_call_site` becomes "two call
  sites, both from constants", with a hostile-input test like
  `the_handoff_opener_takes_only_a_canonical_web_origin`.
- The listener accepts one request on the exact path, checks `state` in constant time, times out
  after 5 minutes, answers with a static page, and closes.
- Token exchange and refresh: fixed hosts only (Google's and Microsoft's token endpoints), through
  the existing blocking `reqwest` on `rustls`, body cap, no redirects.
- XOAUTH2: `user=<address>\x01auth=Bearer <token>\x01\x01`, sent over TLS, from a zeroizing buffer.
- Scopes: Gmail IMAP needs `https://mail.google.com/` (full mail access). Microsoft:
  `https://outlook.office.com/IMAP.AccessAsUser.All offline_access`. **Not verified** for 2026.
- **Owner actions, outside the code:** register a Google OAuth client and a Microsoft Entra app.
  Google's `https://mail.google.com/` is a restricted scope: a public app needs Google's
  verification and a yearly third-party security assessment. **Not verified:** the current cost and
  terms. Until then, Gmail works through an app password (needs 2-Step Verification; **Not
  verified** that Google still allows app passwords for IMAP) or a test-mode OAuth app (limited
  users, short refresh-token life). Microsoft 365 tenants may require admin consent.

#### 5.4.7 The classifier (rule (b)): rule-based and conservative

Input: the expectation (kind, service domains, start time, login), the verified domain, headers,
the text part (or the HTML part converted to text by the MIME library), the HTML links.

Gates, in order. The first failing gate decides "not for me" (gates 1 and 2: ignored, nothing
fetched beyond headers) or "left for you" (gates 3 to 5: shown, not acted on).

1. **Sender:** verified domain fits the expectation (§5.4.5). Else ignored.
2. **Time:** `INTERNALDATE` at or after the window start minus 60 s, UID at or above the baseline,
   and `Date` not more than 5 minutes before the start. Else ignored.
3. **Deny patterns** (subject and text, case-insensitive, EN and DE, word-bounded; a list in data,
   reviewed and tested): approve or authorise a sign-in or device ("approve", "authorize", "was this
   you", "war das du", "bist du das", "new sign-in", "neue Anmeldung", "neues Gerät"); a change of
   e-mail, phone, address, recovery or second factor ("confirm your new email", "bestätige deine
   neue E-Mail", "Telefonnummer ändern", "recovery", "Wiederherstellung", "2FA deaktiviert"); account
   deletion; payments. Any hit → left for you. This applies to every kind, codes included.
4. **Kind:**
   - `LoginCode`, `RenewCode`: a code keyword ("code", "verification code", "one-time", "sign-in
     code", "security code", "Bestätigungscode", "Anmeldecode", "Sicherheitscode", "Einmalcode") in
     the subject or within 200 characters before the code; and exactly one code candidate (§5.4.8).
   - `RenewLink`, `ResetLink`: a reset keyword ("reset your password", "Passwort zurücksetzen") and
     exactly one link that passes §5.4.8.
   - `SignupVerify`: a verify keyword ("verify your email", "confirm your email address", "E-Mail
     bestätigen", "Konto aktivieren") and exactly one link that passes §5.4.8. "Confirm your email"
     here means the address of the account being created; gate 3 already refused "new email" and
     "change".
   - A `RenewCode` or `LoginCode` expectation never follows a link. A link-kind expectation never
     types a code.
5. **One action:** the first mail that passes ends the expectation. A second fitting mail in the same
   window is left for you (it may be an attacker's racing trigger, or a resend).

No scoring, no ML, no learning from the user's mail. Ambiguity always lands on "left for you".

#### 5.4.8 Code and link extraction

Codes:

- Candidates: a run of 4 to 10 digits; or 6 to 10 characters of `[A-Z0-9]` with at least one digit
  and at least one letter; or digit groups joined by one space or hyphen (`123 456`, `123-456`),
  normalised by removing the joiner.
- Excluded: 4-digit numbers from 1900 to 2099; anything inside a URL, an e-mail address or a phone
  number pattern; anything after a signature delimiter (`-- `) or in the last 20 % of the text when
  the text is long (footers: addresses, company numbers).
- Exactly one **distinct** candidate after exclusions, else left for you. The code is held in a
  `SecretString` and goes only into the pending session's code field (§5.2.3), at the exact origin
  or verified program identity of that session.

Links (rule (c)):

- From `href` attributes of the HTML part and URLs in the text part.
- Kept only if: `https`; host exactly the host of one of the login's `https` URLs; default port; no
  userinfo; not an IP literal; host not punycode unless the login's host is the same punycode. Exact
  origin through `crates/origin` (`PageOrigin` equality, not eTLD+1, not the per-URL eTLD+1 mode).
- Tracking redirectors on another host (`click.service-mail.com`) fail this test. Such mail is left
  for you.
- Exactly one distinct kept link (after dropping a fragment), else left for you.
- Following: the link is opened in the session's tab by the extension. Mosaic cannot stop a
  redirect the browser follows. After load, the extension checks that the tab's final origin is the
  expected exact origin; if not, Mosaic fills nothing and ends the session with a message. The visit
  to the redirect target already happened; that is a residual (§6.4). **Not verified:** that the
  extension can read the final URL of that tab with its current permissions (`activeTab` may not
  cover a tab after a cross-site navigation; reading `tab.url` needs `tabs` or host access).
- The desktop never fetches a mail link itself (no general HTTP client exists, and a fetch would
  spend one-time tokens outside the browser).

#### 5.4.9 The 15-minute window

- An **expectation** is created only by a Mosaic action, in Rust:
  - `LoginCode`: a sign-in pick whose site then shows a code field (part 2), or the user's click on
    "Get the code from my mail" in the popup or picker during a pick session.
  - `RenewCode`, `RenewLink`, `ResetLink`: Renew (part 3).
  - `SignupVerify`: the user saved a new login through Mosaic's save dialog right after filling a
    generated password on a sign-up form of that origin.
- It holds: login (vault, item), kind, service domains, exact origin or program identity of the
  session, start time (monotonic and wall clock), end = start + 15 minutes, account, IMAP baseline.
- At most one expectation per service domain, and at most 3 at once. A fourth is refused with a
  message.
- It ends at the first action, at 15 minutes, on Cancel, on lock, on quit, and when its session ends
  (the tab closes, the program exits).
- Nothing in the webview or the extension can create one. The IPC that starts Renew or a sign-in
  creates it in Rust as a side effect of the action, never as a free-standing command.

#### 5.4.10 Tagging

- After acting: `UID STORE <uid> +FLAGS.SILENT (Mosaic)` if `PERMANENTFLAGS` lists `\*` or `Mosaic`.
  On Gmail (`X-GM-EXT-1`): `UID STORE <uid> +X-GM-LABELS (Mosaic)`.
  **Not verified:** keyword support on Outlook.com and Exchange Online IMAP; without it, no tag.
- Nothing else changes: no `\Seen` change (Q13), no move, no delete.
- Mail left for you and ignored mail is not changed at all (`BODY.PEEK`, no `STORE`).

#### 5.4.11 What the user sees

- **Settings → Mail assistant** (off by default, under the settings MAC): add an account (Google,
  Microsoft, other IMAP), a plain explanation before consent: "Mosaic reads new mail from the
  service you are signing in to, for 15 minutes after you start, on this device only. It acts only
  on codes and links it expects, and tags those mails 'Mosaic'. Everything else stays untouched."
- **During a window:** in the picker list, the popup and the desktop: "Waiting for a mail from
  github.com · 14:21 · Stop".
- **After:** "Code from github.com entered" or "A mail from github.com arrived, but Mosaic didn't use
  it: <reason>. Check your mail." Reasons are short and fixed (asked for approval, more than one
  code, link to another address, not verified).
- **Activity:** a local, unsynced list of the last 50 events (time, domain, kind, outcome; no
  subject, no content, no code), in Settings, deletable. Stored in the local store (CRYPTO_SPEC §11
  gets a row).
- No OS notification (ADR-0069 chose none); toasts inside Mosaic only.

### 5.5 Platform scope

#### 5.5.1 Windows

Parts 1 and 2 as above. Part 3 and 4 run in the desktop and the extension, so on every desktop OS
where those run.

#### 5.5.2 Android: one-tap linking

Today (m10, ADR-0092): "No login for <app>" → `LinkActivity` (one-time token) → unlock →
`PickLoginScreen` → `LinkAppScreen` (app label, package, installer, first 8 bytes of the cert,
asset-links status) → confirm. That is several taps.

Change, keeping ADR-0092's rules (no automatic linking, an explicit choice on the login, no "fill
anyway"):

- When the fill request names an app with no linked login, the core computes **strong candidates**:
  - a login with `androidapp://<package>` without `cert` (an import, "needs linking") whose package
    equals the caller's; or
  - a login with an `https` URL whose host's `assetlinks.json` lists this package and current signer
    with `get_login_creds` (`asset_links_status` = `Verified { host }`, the G3-3 host).
- For at most 3 strong candidates, the response adds datasets "Link <title> and fill" (presentation
  only, null values, per-dataset authentication as today).
- A tap opens `FillAuthActivity`. The **biometric prompt is the confirmation**: its title and
  description name the app label, package, the signer fingerprint prefix and "Verified by <host>"
  (or "Imported for this app"). One biometric confirm links (`link_android_app`, writes the
  cert-bound URL) and fills. If the vault needs the master password instead, the existing
  `LinkAppScreen` is shown.
- Weak cases ("Not verified", "Could not check") keep the current full flow with its warning.
- **Not verified:** that `BiometricPrompt`'s text is enough for an informed choice on small screens;
  the ux-reviewer checks it. Browsers stay excluded (G3-5).

#### 5.5.3 iOS

Only what Apple allows:

- Apps with associated domains (`webcredentials:`) get QuickType suggestions for those domains;
  Mosaic's Credential Provider already serves them (ADR-0092, `ios_fill_target`). Nothing to link.
- Other apps: the user opens the provider from the keyboard's key icon (manual picker). Apple gives
  the provider no app identity there, so Mosaic cannot bind. ADR-0092 keeps "Search all" read-only;
  that stays. Q14 covers a copy action.
- iOS 18 one-time codes are already handled (ADR-0092). **Not verified:** the iOS 18 "insert text"
  provider entry point as a manual, user-driven path.
- No mail assistant on iOS in this design.

#### 5.5.4 macOS (later)

- Detection: Accessibility API. `AXObserver` with `kAXFocusedUIElementChangedNotification`;
  password fields have the `AXSecureTextField` subrole; `AXPosition`/`AXSize` for the chip.
- Needs the user's Accessibility permission (TCC). Signed builds with hardened runtime only
  (ADR-0082), or the permission attaches to an unstable identity.
- Writing: setting `AXValue` on a secure field (**Not verified** that it is allowed), else typing
  with `CGEventPost`. **Not verified:** how Secure Event Input (on while a secure field has focus)
  affects posted events.
- Identity: code signature through `SecCode` / `SecStaticCode`: Team ID plus bundle ID, from the
  designated requirement. `security-framework` is already a dependency.
- Wrapper crates to vet: `accessibility` / `accessibility-sys`, `objc2-application-services`,
  `core-graphics` for events.
- Scheme: `macapp://<team id>/<bundle id>`.

#### 5.5.5 Linux (later)

- Detection: AT-SPI2 over D-Bus (`atspi` crate, on `zbus`, which is already a dependency). Focus
  through `object:state-changed:focused`; role `PASSWORD_TEXT`; writing through `EditableText`.
- Identity: the D-Bus sender → PID (`GetConnectionUnixProcessID`) → `/proc/<pid>/exe`. No code
  signing exists. Flatpak (app ID from `.flatpak-info` in the process's root) and Snap names are the
  closest thing to a signed identity. Plain binaries: path plus hash, as unsigned Windows programs.
- X11: chip positioning works; typing through XTest; every X11 client can read input anyway.
- Wayland: clients get no global coordinates (a chip next to another app's field is not possible
  outside layer-shell compositors), no input synthesis without the RemoteDesktop portal or `libei`
  and a user consent, and AT-SPI coordinates may be meaningless. Wayland gets the list in Mosaic's
  own window from the hotkey (already "Unsupported" for global hotkeys in `shortcut.rs`) and
  `EditableText` writes only.

---

## 6. Security model and threats

### 6.1 Trust model

- **Trusted:** the desktop process, the OS's answers about processes and signatures (Windows
  `WinVerifyTrust`, package identity), the user's pick in Mosaic's own windows, the provider's
  topmost `Authentication-Results` header for known providers, DNS-fetched DKIM keys (with the
  residual below).
- **Untrusted:** every other program's UI tree and window contents (captions, version resources,
  titles), every web page, every mail's headers and content apart from the verified signature, the
  visible `From`.
- **Out of scope** (as today): same-user malware (P5) that injects into processes, synthesises
  input, reads Mosaic's memory or the keystore. The design does not claim to stop it.

### 6.2 New THREAT_MODEL rows (descriptions; numbers assigned later)

Part 1 (program fill):

- S: A look-alike program (same exe name, other signer, or a homoglyph name) asks for a login.
  Mitigation: signer subject or PFN binding, exe name required, look-alike warnings at link time.
- S: An unsigned program at a user-writable path is replaced. Mitigation: path plus file hash; a
  change prompts.
- S: A browser, script host or credential broker poses as a program. Mitigation: exclusion classes;
  never bindable.
- T: The target swaps its window, focus or field between the pick and the write (TOCTOU), or a PID
  is reused. Mitigation: held process handle, the eight checks of §5.1.2 before every batch.
- T: An overlay window from another process covers the field so the user's pick fills something
  else. Mitigation: occlusion check by `ElementFromPoint`; residual for a same-process overlay
  inside the target (that is the target's own UI).
- T: Injected keystrokes land elsewhere when focus moves mid-typing. Mitigation: batches of 8,
  focus events abort, modifier wait.
- I: Third-party web content inside a linked program (Electron loading a remote page, WebView2)
  receives the program's login. Mitigation: refusal of foreign-process content (WebView2 by Q2);
  residual for Electron content in the program's own process.
- I: Other same-user processes with low-level keyboard hooks see typed passwords. Residual, the same
  as a user typing.
- I: Mosaic observes focus in every program. Mitigation: local only, nothing stored, off until the
  user turns it on.
- E: A medium-integrity Mosaic would gain power over elevated UI. Mitigation: no `uiAccess`, no
  elevated fill.
- D: A hung or hostile UIA provider stalls Mosaic. Mitigation: UIA timeouts, a dedicated worker
  thread.
- Supply chain: wrapper crates with `unsafe` run in the vault process. Mitigation: ADR vetting,
  exact pins, reading on every bump.

Part 2 (fill and submit):

- T: A page or program triggers submit without a pick. Mitigation: submit only inside a pick
  session; the brief §4.1 suite stays.
- T: A form posts credentials to another origin. Mitigation: no submit when the resolved `action` is
  another origin.
- T: The form changes between fill and submit (a different default button, new fields). Mitigation:
  re-detect at submit time; differ → no submit.
- T: A two-step continuation fills a second screen of another program or site. Mitigation: same
  process handle and window, or same tab and exact origin, 60 s, once.
- Residual: a site's own "first submit button" may be a different action; Mosaic follows HTML's
  definition only.

Part 3 (Renew):

- T: The change page redirects to another host or SSO. Mitigation: exact https origin before fill
  (ADR-0068 rules), stop otherwise.
- I: The new password syncs before the site accepts it. Existing R101.
- T: The site applied the change but reports failure, or the reverse. Existing R100; the result
  dialog states it.
- T: A desktop-initiated Renew opens a tab the extension cannot verify. Mitigation: the fill needs
  the toolbar click in that tab (§5.3.2 step 2).

Part 4 (mail assistant):

- S: A forged mail claims the service's domain. Mitigation: DKIM pass with DMARC alignment; topmost
  provider header with a known authserv-id; `l=` and `rsa-sha1` refused.
- S: DNS spoofing of a DKIM key (own check only). Residual: no DNSSEC requirement; a network
  attacker on the resolver path could forge keys for providers that need the own check.
- S: A weak (1024-bit) DKIM key of a service is factored. Residual, as for every mail client.
- T: An attacker triggers a real mail from the service ("approve this sign-in", "confirm new email",
  a reset) inside the window. Mitigation: deny patterns, kind rules, one action per window; codes go
  only into Mosaic's own session; residual for a racing reset (harmless: Mosaic sets its own
  password) and for sign-up verification of an account an attacker pre-registered with the user's
  address in the same 15 minutes (account pre-hijacking).
- T: A link redirects to another host. Mitigation: exact host before opening; no fill after a
  cross-host landing; residual: the one visit to the redirect target.
- I: Mailbox access leaves the device. Mitigation: no server path; fixed OAuth and IMAP hosts; a test
  that the server crate has no mail types.
- I: Refresh token or password at rest. Mitigation: vault secret field; access token in memory only;
  no shared vaults. Residual: the token gives full mail access to anyone who unlocks the vault.
- I: Mail content in memory. Mitigation: zeroizing buffers; residual for parser allocations.
- I: The local activity list shows a disk reader which services sent codes and when.
  Accepted, like R102.
- I: The OAuth consent page and token endpoints see the user's IP and that Mosaic is the client.
  Accepted, as the user chose that provider.
- D: A hostile IMAP server or a huge mail. Mitigation: size caps, fuzzed parsers, timeouts, one
  connection, window limits.
- E: The webview or extension creates an expectation to open the mailbox at will. Mitigation:
  expectations only as a side effect of Rust-side actions; no IPC command creates one.

Android linking:

- S: A one-tap link to a malicious app that a site's `assetlinks.json` lists, or that reuses an
  imported package name. Mitigation: only Verified or exact-package candidates; the biometric prompt
  names the app and signer; residual R136 (a deliberately linked malicious app) widens slightly.

### 6.3 Red-team tests that must exist

Program fill (Rust, against the `ProgramProbe` fake, plus a Windows integration suite with test
programs; extension-attacker style):

- `program_url_parse_refuses_non_canonical_forms` (fuzz target `program_url` in `crates/origin/fuzz`)
- `signed_binding_needs_equal_signer_and_exe_name`
- `same_exe_other_signer_does_not_match`, `same_signer_other_exe_does_not_match`
- `unsigned_binding_needs_equal_path_and_hash`, `signed_image_never_matches_a_path_binding`
- `browsers_hosts_brokers_and_remote_clients_never_match`
- `mosaic_never_fills_its_own_windows`
- `focus_change_between_pick_and_write_writes_nothing`
- `window_swap_or_foreground_change_aborts_typing`
- `pid_reuse_cannot_take_over_a_session` (handle held; fake reports a new process)
- `occluding_window_blocks_the_fill`
- `password_never_written_into_a_non_password_field`
- `elevated_target_gets_no_chip_and_no_fill`
- `no_fill_without_a_pick` (focus events alone, any number, write nothing)
- `picker_webview_never_receives_a_secret` (search every picker event and answer for the password,
  in the `a_passkey_private_key_reaches_no_command_and_no_item_view` style)
- `picker_window_gets_only_picker_commands` (capability test)
- Windows integration programs: a fake "Discord.exe" unsigned in `%TEMP%`, a signed test program
  re-signed with a second test certificate, a program that moves its password field under the
  cursor on focus, a program that opens a topmost click-through window over its field, a program
  that swaps its foreground window 100 ms after focus.

Fill and submit (extension Playwright hostile pages and unit tests):

- `submit-without-pick.spec.ts` (load, focus, synthetic events: no submit)
- `submit-cross-origin-action.spec.ts` (filled, not submitted)
- `submit-form-swap.spec.ts` (fields or default button replaced after fill: no submit)
- `submit-hidden-default-button.spec.ts` (hidden or covered first submit button: no click)
- `shortcut-many-matches-no-submit.spec.ts`
- `two-step-other-origin.spec.ts` (continuation never crosses origin or tab)
- Rust: `two_step_continuation_is_bound_to_window_and_time`

Renew:

- `renew-change-url-redirects-off-site.spec.ts` (stop, nothing filled)
- `renew-ambiguous-form.spec.ts` (attempt stays open, no secret requested)
- `renew-never-confirms-without-click` (Rust: no path confirms an attempt without the dialog)
- `renew_keeps_every_value_on_every_outcome` (property test over outcomes, ADR-0066 invariant)

Mail assistant (Rust, `crates/mail`, with recorded and synthetic mails; fuzz targets `mime_parse`,
`auth_results_parse`, `classify`):

- `from_header_alone_never_verifies`
- `dkim_pass_for_unaligned_domain_is_not_a_pass`
- `sender_written_authentication_results_is_ignored`
- `l_tag_and_rsa_sha1_signatures_are_refused`
- `approve_was_this_you_and_change_mails_are_never_acted_on` (EN and DE corpus)
- `mail_before_window_or_below_baseline_uid_is_ignored`
- `window_ends_after_15_minutes_and_on_lock` (injected clock)
- `two_code_candidates_leave_the_mail_for_the_user`
- `link_to_other_host_or_tracking_redirector_is_never_followed`
- `link_with_userinfo_port_ip_or_lookalike_host_is_refused`
- `second_fitting_mail_in_a_window_is_left_for_the_user`
- `untouched_mail_gets_no_store_command` (IMAP transcript test: only `BODY.PEEK`, no `STORE`, no
  `\Seen`)
- `handled_mail_gets_only_the_mosaic_keyword`
- `no_ipc_command_creates_an_expectation`
- `imap_refuses_starttls_and_plain_port`
- `server_crate_has_no_mail_types` (static check)
- `oauth_opener_takes_only_constants` and the extended `opener_has_one_call_site`
- `oauth_listener_refuses_wrong_state_path_and_second_request`
- `no_mail_secret_reaches_logs_errors_or_the_webview` (`Debug` and event scans)

Android:

- `one_tap_link_offered_only_for_verified_or_exact_package_candidates`
- `browser_package_never_gets_a_link_offer`
- Instrumented: `oneTapLinkNeedsBiometricConfirm`, `noValuesBeforeConfirm`

### 6.4 Accepted residuals (to be written as rows)

- Same-user malware (P5): injection into a signed process, synthesised input, keyboard hooks.
- Electron programs that render remote web content in their own process.
- The visit to a cross-host redirect target of an exact-host mail link.
- DNS-forged DKIM keys and factored 1024-bit keys for the own-check path.
- Sign-up verification of a pre-registered account within the window.
- The OAuth scope gives Mosaic full mailbox access, though Mosaic uses a tiny part of it.

---

## 7. Open questions (builder's call)

Each has a recommendation. The builder decides, records it in the part's ADR, and moves on.

- **Q1. Default of `programFill`.** Builder's call. Recommendation: off until the user turns it on
  from the first-unlock card; revisit after the Chromium accessibility cost is measured (§5.1.1).
- **Q2. WebView2 content inside a program.** Builder's call. Recommendation: refuse in v1. Then allow
  it when the focused element's process is `msedgewebview2.exe` with a valid Microsoft signature,
  its parent process is the host (read from the OS), and the element is inside the host's top-level
  window; record the third-party-content residual.
- **Q3. Unsigned bindings: path plus hash, or path only.** Builder's call. Recommendation: path plus
  hash (stricter than D3 requires, catches silent swaps), with a one-click "Link again" after an
  update that shows old and new hash prefixes.
- **Q4. Fallback hotkey default.** Builder's call. Recommendation: no default until a collision survey
  of common programs; the setting offers one.
- **Q5. Desktop-started Renew in the browser.** Builder's call. Recommendation: Renew starts from the
  popup on the site's tab (one click, `activeTab` granted); from the desktop it opens the tab and
  asks for one toolbar click ("Continue renewal"). Do not add host permissions for this.
- **Q6. Reset-by-mail and sites that confirm a change by link.** Builder's call. Recommendation: v1
  supports `RenewCode` and the user-started "Reset with Mosaic" only; a per-site quirk list for
  confirm-by-link changes waits until real sites are tested.
- **Q7. OAuth library or hand-written requests.** Builder's call. Recommendation: the `oauth2` crate
  with a blocking adapter over the existing `reqwest`, if it adds no TLS or async HTTP stack;
  otherwise two hand-written requests with tests against RFC 7636 vectors.
- **Q8. `fill-login` with several matches.** Builder's call. Recommendation: open the popup if the
  browser allows it from a command (**Not verified** across Chrome, Edge and Firefox); else fill only,
  as today, never submit.
- **Q9. An inline web dropdown.** Builder's call, as its own ADR under brief §4.4. Recommendation: a
  part 2b after part 2 ships: an extension-origin frame (one `web_accessible_resources` page with
  `use_dynamic_url`) instead of a closed shadow root, because a page cannot script into it; with
  visibility checks from inside the frame (`IntersectionObserver` v2 where available, **Not
  verified** on Firefox) and a full extension-attacker pass. Until then the popup, the command and
  the context menu stay the only web gestures.
- **Q10. TLS 1.2 for IMAP.** Builder's call. Recommendation: TLS 1.3 only at first; add 1.2 for IMAP
  only if a real provider needs it, through a CRYPTO_SPEC change and an ADR.
- **Q11. One synced refresh token or one per device.** Builder's call. Recommendation: per device
  (`mail.oauth_refresh_token:<device id>` secret elements in the same item), so devices do not race
  token rotation and a device removal can drop its token; the account settings stay shared.
- **Q12. Extra sender domains per login.** Builder's call. Recommendation: allow them, added only in
  the item editor (never from a mail), shown on the item, exact-domain match only.
- **Q13. Mark handled mail as read.** Builder's call. Recommendation: no; only the keyword. The
  user's mail program decides.
- **Q14. iOS "Search all" copy action.** Builder's call. Recommendation: add "Copy password" (with
  the clipboard rules of ADR-0094) for apps without associated domains; never a fill without an
  identifier.
- **Q15. Junk folder.** Builder's call. Recommendation: INBOX only in v1; mention in the "left for
  you" text that a code may be in Junk.
- **Q16. Mail assistant on Android.** Builder's call, after desktop. Recommendation: only while a
  Mosaic fill session is in the foreground; no background service.

## 8. Builder check (2026-11-14)

A fresh builder read this document. Its questions fall into two groups.

### For the owner (they change what gets built)

- **O1. Mail providers and OAuth in v1.**
  - Which provider holds most of the owner's accounts?
  - Will the owner register the OAuth apps?
    - Google: a restricted `https://mail.google.com/` scope, verification, and a yearly paid security assessment.
    - Microsoft Entra: possibly admin consent for work tenants.
  - Without OAuth, part 4 v1 is IMAP with an app password only, and Outlook.com cannot be used at all.
  - Until answered, the plan builds the IMAP core first and keeps OAuth as a separate, later step.
- **O2. Windows UI Automation without a vetted wrapper.**
  - If no wrapper crate passes vetting (§5.1.1), does the owner accept an ADR exception: one small, isolated, audited shim crate with `unsafe`? Or do program fills stop?
  - Until answered, the plan starts part 1 with the vetting spike as a go/no-go gate. Nothing after it is built if it fails.
- **O3. The Renew click count.**
  - Renew started from the desktop app needs one extra click in the browser toolbar (Q5). Avoiding it needs broader extension host permissions, which is a security trade-off.
  - Until answered, keep the extra click.

### Calls made by the orchestrator (the owner can still override)

- **C1. Android one-tap linking (§5.5.2)** is a standalone item. It is built right after M10 merges into main and before part 1. It is small and the owner named Android apps explicitly.
- **C2. Chromium and Electron accessibility cost.**
  - The vetting spike measures the CPU and memory of a UIA client against Chrome, an Electron app and a WPF app, all idle.
  - If the client forces Chromium accessibility on with a noticeable cost (over 5% CPU idle or 150 MB extra), Chromium and Electron programs are opt-in per program.
  - Part 1 still ships for Win32, WPF and UWP.
- **C3. Windows test infrastructure.**
  - No self-hosted runner: the repository is public, and a self-hosted runner on a public repository runs untrusted pull-request code.
  - Matching, identity and policy logic is unit-tested in CI.
  - Interactive UIA, focus and `SendInput` tests run locally from a documented manual checklist before each release, the same way as the existing device checks in MOBILE.md.
