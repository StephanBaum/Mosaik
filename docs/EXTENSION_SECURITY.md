# Extension security

Status: v4 (M5 track T3: the opt-in passkey interception, §11, on top of v3 from the M4
close, whose §10 maps every item of brief §4 to its mitigation and test). Normative
companions: `THREAT_MODEL.md` §4.3, `NATIVE_MESSAGING.md`, ADR-0035 (architecture and
gestures), ADR-0036 (channel), ADR-0037 (fill policy), ADR-0038 (save prompts) and ADR-0040
(passkeys).

The extension is the highest-risk part of Mosaic: it runs next to hostile pages. It holds no
vault, no master password and no key of the vault. It sees a credential only when the user asks
for a fill, for one item, one frame and one moment.

## 1. Attack, mitigation, test

Tests are under `apps/extension/` (`src/**/*.test.ts` run with `pnpm exec vitest run --project
extension`; `e2e/hostile/*.spec.ts` run in Chromium with the e2e build, see §7).

| Attack | Mitigation | Test |
|---|---|---|
| A page fills itself: autofill on load, focus tricks, polling for values | No content script on load. Injection happens in the click handler of a gesture (`activeTab` + `scripting`); the desktop is asked only then | `e2e/hostile/autofill-on-load.spec.ts` |
| A page fakes the gesture: synthetic keys (Ctrl+Shift+L), clicks, `contextmenu`, `submit`, `postMessage` | The three gestures (popup click, `commands.onCommand`, `contextMenus.onClicked`) are delivered by the browser from its own UI. Nothing in the extension listens to page events | `e2e/hostile/synthetic-events.spec.ts` |
| Clickjacking: a page restyles or covers an injected button or overlay | There is no injected UI at all (ADR-0035). A page that covers its own fields makes them unfillable (hit test) | `e2e/hostile/overlay-occlusion.spec.ts` |
| Cross-origin iframe next to or instead of the real form | Target frame is chosen from the frame the user focused or clicked, else the top frame, else the only frame with fields. A frame other than the top frame is refused unless its origin equals the tab's, every ancestor is same-origin and every `<iframe>` element on the way is visible (`decideFrame`, `frameAncestry`) | `src/background/frames.test.ts`, `src/content/frame.test.ts`, `e2e/hostile/cross-origin-iframe.spec.ts` |
| Same-origin iframe the user did not mean | Needs a confirmation click in the popup (`confirmFrame`); a shortcut or menu fill stops with `frameNeedsConfirmation` (badge and popup). The desktop is told `sameOriginFrame` and refuses `origin != topOrigin` | `e2e/hostile/same-origin-iframe.spec.ts` |
| A-B-A nesting: our login form inside a hostile frame inside our page | Every ancestor must have the frame's origin; the middle frame fails that, so the inner frame is `crossOriginFrame` even when confirmed | `e2e/hostile/nested-a-b-a.spec.ts` |
| Sandboxed or `srcdoc` frame, opaque origin | Origin `"null"` and non-http(s) origins are `frameOpaque`; the browser may also keep the script out | `e2e/hostile/sandboxed-srcdoc.spec.ts`, `src/background/frames.test.ts` |
| A document with an opaque origin but a site URL: a page served with `Content-Security-Policy: sandbox` (how sites host untrusted HTML on their own host), or the site's page in a sandboxed `<iframe src>` | The content script reports and re-checks the document's own origin (`window.origin`, `"null"` here), never `location.origin`, which follows the URL; ancestry compares `parent.origin` the same way. Result: `frameOpaque` | `e2e/hostile/csp-sandbox.spec.ts`, `src/content/frame.test.ts` |
| `about:blank` (or `javascript:`) child frame that inherits an origin | The browser injects into it; it carries the origin it inherited, so on the site it is a same-origin frame (confirmation needed) and inside another origin's frame it is `crossOriginFrame` | `e2e/hostile/about-blank.spec.ts` |
| Hidden same-origin frame: off screen, covered by an opaque box, `filter: opacity(0)`, in a clipped box below the fold, shrunk to 0 x 0 after the probe | Every `<iframe>` element on the way gets the full field test (`isFillable`: size, opacity, document bounds, hit test, clip) in its parent's document, realm-independent; inside a frame a field must be on screen in the frame's viewport. Re-run at fill time | `e2e/hostile/hidden-frame.spec.ts`, `e2e/hostile/race-after-probe.spec.ts`, `src/content/frame.test.ts`, `src/content/visibility.test.ts` |
| Frame `src` changed to another origin, or an https page downgraded to http, after the probe | `documentId` pinning and the origin re-check in the content script | `e2e/hostile/race-after-probe.spec.ts` |
| Tab URL that disagrees with the document: `history.pushState` to another path, the tab replaced by a `blob:` document | The top origin comes from the tab URL and must be http(s); pushState keeps the origin, `blob:` is `unsupportedPage` | `e2e/hostile/url-tricks.spec.ts` |
| Context-menu gesture naming a missing frame, another origin's frame, or a same-origin frame | The frame id comes from the browser; a missing frame is `noFields`, another origin `crossOriginFrame`, a same-origin frame `frameNeedsConfirmation` (fill and generate), before the desktop is asked | `e2e/hostile/forged-frame-id.spec.ts` |
| Rapid repeated gestures | Each gesture fills the one chosen credential into the top frame; at most four desktop requests are in flight (`MAX_IN_FLIGHT`), the rest are `busy` | `e2e/hostile/race-after-probe.spec.ts` |
| A password generated in the popup for one site filled into another (context-menu "Generate password" within two minutes) | `GeneratedHold` is bound to the tab and page origin it was generated on; any take clears it; elsewhere a fresh password is generated | `e2e/hostile/generated-hold.spec.ts`, `src/background/fill.test.ts` |
| Look-alike or homoglyph host | The desktop matches the exact origin; the extension reports the frame's own origin, never a page-supplied one. An `xn--` host is shown in Unicode (`unicodeHostOf`) with the desktop's `idn` warning; there is no "fill anyway" | `e2e/hostile/lookalike.spec.ts`, `src/shared/origin.test.ts` |
| Fill over plain HTTP | The desktop refuses an http page unless the item's URL allows it, and never matches an https URL to an http page; the extension skips `blocked` matches | `e2e/hostile/http-page.spec.ts` |
| Hidden fields: opacity 0, off screen, 0 or 1 px, clipped, `display: none`, `visibility: hidden`, transparent parent | `isFillable`: displayed (`checkVisibility`), at least 4 x 4 px, combined opacity of field and ancestors at least 0.2, inside the document, and on screen at one of five points the field (or its label) is the top element. Runs at probe and again at fill | `src/content/visibility.test.ts`, `e2e/hostile/hidden-fields.spec.ts` |
| Hidden fields the first rules missed: `filter: opacity()` on the field or a parent, a transparent `mask-image`, a field below the fold in a zero-height `overflow: hidden` box or a `clip-path` box (off screen, where no hit test runs) | Opacity includes every `filter: opacity()`; any `mask-image` counts as opacity 0 (masks are not measured: conservative). Off screen a field is fillable only in the top document and when neither it nor an ancestor has a clip shape (`clip-path`, `clip`) and no ancestor's `overflow: hidden/clip` box leaves less than 4 x 4 px of it (the root element and the body excepted) | `e2e/hostile/hidden-css-tricks.spec.ts`, `src/content/visibility.test.ts` |
| Field changed or injected between the probe and the fill | The content script detects again before writing; fields that changed type, were renamed, or are not fillable now receive nothing; an empty result is `noFields`, not success | `src/content/detect.test.ts`, `e2e/hostile/type-swap-and-late-inject.spec.ts` |
| Navigation between the probe and the fill | Delivery is pinned to the probed `documentId` (Chromium); the content script compares `location.origin` with the expected origin (`originChanged`), re-checks ancestry and visibility | `e2e/hostile/navigate-after-probe.spec.ts` |
| DOM clobbering: `<input name="elements">`, `name="type"` and the like | Fields are found with `querySelectorAll("input")`; attributes are read with `getAttribute`; the form of a field is `input.form`, never `form.elements` | `src/content/detect.test.ts` |
| DOM clobbering of globals: an element with id or name `mosaicContent` (window named properties do reach the isolated world in Chromium) made the script think it was installed, so it never answered; `<img name="documentElement">` and the like | The script installs itself unless `mosaicContent` is an own property of the global, and defines it as one. Document named properties were measured not to reach the isolated world | `e2e/hostile/clobber-and-pollution.spec.ts` |
| Prototype pollution in the page's world: replaced `value` setter, `checkVisibility`, `getComputedStyle`, `getBoundingClientRect`, `elementFromPoint`, `getAttribute`, `dispatchEvent`, `Object.prototype` keys | The content script runs in the isolated world; none of the page's replacements runs for it | `e2e/hostile/clobber-and-pollution.spec.ts` |
| Spoofed extension messages from a page or another extension | See §3 | `src/background/router.test.ts`, `e2e/hostile/spoof-messages.spec.ts` |
| Another installed extension: `runtime.sendMessage` and `runtime.connect` (as `popup` and other names) to Mosaic, a fill-shaped `tabs.sendMessage` to the tab, reading Mosaic's content-script state | `externally_connectable: {ids: []}` and no external listeners: refused and disconnected; `tabs.sendMessage` reaches only the sender's own content scripts; each extension has its own isolated world | `e2e/hostile/other-extension.spec.ts` (with `test/evil-extension/`) |
| A page reads the extension's files or pages | No `web_accessible_resources`; `externally_connectable: {ids: []}` | `scripts/manifest.test.ts`, `e2e/hostile/spoof-messages.spec.ts` |
| A framework ignores a programmatic value (React, Vue) | The value is set with the DOM's own setter from the isolated world, then `input` (composed `InputEvent`) and `change` fire | `src/content/fill.test.ts`, `e2e/hostile/react-controlled.spec.ts` |
| Test code in the shipped package | `excludeE2e` replaces `src/background/e2e.ts` outside `--mode e2e`; `check-dist.mjs` fails the build on `mosaicE2E`, `e2eTabId`, `FakeDesktop`, `FakePasskeys`, `passkeyDecide`, `passkeyPending`, `pauseAfterProbe`, external listeners, `setAccessLevel` (content scripts keep no access to `storage.session`, where pending saves wait), `host_permissions`, `content_scripts`, `web_accessible_resources`, `optional_permissions`, `key`, a changed permission or optional-host list, or an `externally_connectable` other than `{ids: []}` (Chromium) / absent (Firefox) | `scripts/check-dist.mjs`, `scripts/manifest.test.ts` |
| Save capture (opt-in) reads what the user types, or a page makes it report a login | Off by default. On: the popup asks for the optional site access in the click, the background registers `capture.js` (never in the manifest; `check-dist.mjs` fails if `content_scripts` exist). The script reacts to trusted clicks on submit controls and the Enter key, not to `submit` (a script's `requestSubmit()` raises a trusted `submit`), reads only the submitted form, at most once per 2 s, and listens to no page message (`check-dist.mjs` greps for it). Origin and tab come from the browser; cross-origin, opaque and hidden-ancestry frames are dropped | `src/capture/capture.test.ts`, `src/background/save.test.ts`, `e2e/save.spec.ts` |
| Clickjacking the capture: a same-origin frame holding a login the page filled in, laid invisibly (`opacity: 0`, `filter: opacity(0)`) over a harmless button, so the user's real click submits it. Found by the T4 attacker pass: the script checked only that the ancestors were same-origin, not that the frame was visible | The script reports only when `mayReport` holds: the top document, or a frame whose ancestors all have its origin and whose `<iframe>` elements all pass `isFillable` in their parents (`frameAncestry().frameVisible`), checked in the click handler | `e2e/hostile/capture-frames.spec.ts`, `src/capture/capture.test.ts` |
| A login submitted in an opaque document with the site's URL: a page served with `Content-Security-Policy: sandbox`, the site's page in a sandboxed frame | Chromium's `sender.origin` is `"null"` there and the background drops it; the script itself also refuses when `window.origin` is `"null"` or differs from `location.origin`, for browsers that give the background only the URL (Firefox) | `e2e/hostile/capture-frames.spec.ts`, `src/capture/capture.test.ts` |
| Clobbering the capture's marker (`<img name="mosaicCapture">`, an element with that id, `Object.prototype.mosaicCapture` in the page's world) so the script never installs | The script installs unless `mosaicCapture` is an own property of its global, and defines it as one | `e2e/hostile/capture-frames.spec.ts` |
| A page feeds the capture a login to save | Scripted `submit`, `click` and key events are ignored. A real click leaves the page free to fire a scripted one afterwards; the result is a pending save whose host and user name the popup shows and whose password stays hidden, and nothing is written until the desktop's own dialog is allowed (R44) | `e2e/save.spec.ts` ("a page cannot make the capture report a login"), `src/background/save.test.ts` |
| A captured or generated password shown or sent for another site or tab | A pending save is bound to its tab and the tab's origin: another origin, five minutes, a closed tab and a tab that left the site drop it. The popup shows the host, the user name and a hidden password; "Save" sends the stored login, never anything the popup supplies | `src/background/save.test.ts`, `src/background/popup.test.ts`, `e2e/generator.spec.ts` |
| A frame confirmation clicked for one frame is used for another (the page swaps the focused frame between the popup's state and the click) | `confirmFrame` names the frame id and the document id the popup showed when the dialog opened; the background fills only when the target at fill time is that frame and document, else `frameNeedsConfirmation`. Where the browser gives no document id (Firefox) only the frame id is compared | `src/background/fill.test.ts`, `e2e/popup.spec.ts` |
| The page reloads or replaces the confirmed frame between the popup's state and the click on Fill or Fill code, or sends the confirmed frame to another site between the probe and the delivery | The confirmation names the frame id and the document id; a reloaded frame has a new document, a replaced one a new frame id: `frameNeedsConfirmation`. Delivery is pinned to the probed document | `e2e/hostile/popup-binding.spec.ts` |
| The tab moves to another site between the popup's listing and the click (Fill, Fill code, Fill of a generated password, Save) | Every popup operation reads the front tab's URL at the click: the desktop matches the item against the new origin, the generated hold and the pending save are bound to the old one (the hold is cleared by the attempt, the pending save dropped when the tab leaves) | `e2e/hostile/popup-binding.spec.ts` |
| The site's real login page framed by another site, so the popup lists its logins for the framed form | The frame is `crossOriginFrame`: every Fill in the popup is disabled, and a forced fill or code fill is refused by the background before the desktop is asked | `e2e/hostile/popup-binding.spec.ts` |
| The popup's generated password filled into another tab or site, or a different password than the one shown | The hold is bound to the tab and origin, is taken once, and the popup's Fill requires it (`generatedExpired` otherwise, never a new password); "this site" generation takes the host and the page's rules from the tab, not from the popup | `src/background/fill.test.ts`, `src/background/popup.test.ts`, `e2e/generator.spec.ts` |
| The popup shows a vault password or a code | It never does: lists hold titles and user names, a fill or copy goes through the desktop, a code is filled or copied there, the save card hides the password, and the only secret on screen is the generated password (`data-secret`) | `src/popup/logins.test.tsx`, `src/popup/save-generator.test.tsx`, `e2e/popup.spec.ts` |
| Inline UI or page-visible traces from the popup or capture | None: nothing is inserted into pages; the popup is an extension page under the extension CSP (no inline script or style: `src/popup/strings.test.ts` checks styles use tokens and no `style=`) | `scripts/check-dist.mjs`, `src/popup/strings.test.ts` |
| Passkey interception (opt-in): a page tampers with the shim, forges its messages, asks for another site's passkey, or uses a frame | See §10: the shim is page-controlled and not trusted; origin and frame come from `MessageSender` and the bridge's `window.origin`, the desktop checks the rp ID and shows a dialog for each use | `e2e/hostile/passkey-*.spec.ts`, `src/background/passkey.test.ts`, `src/passkey/*.test.ts` |
| Malformed or hostile replies from the host | Every message is validated with exact keys and the fixture samples of the Rust side; unknown keys, versions and events are dropped | `src/shared/protocol.test.ts` |

## 2. Gestures and the fill

A fill starts only from:

1. a click in the toolbar popup (`fill`, `fillOtp`, `fillGenerated`),
2. the `fill-login` command (Ctrl+Shift+L, Command+Shift+L on macOS),
3. the context-menu items "Fill login" and "Generate password" on editable fields.

The sequence (`src/background/fill.ts`): the top origin comes from the tab URL (anything but
http(s) is `unsupportedPage`); the content script is injected into all frames and each frame
reports a probe (`origin`, ancestry, visibility, field counts); the target frame is chosen and
`decideFrame` decides; the item is the one the user chose, or for the shortcut and the menu the
first non-blocked match the desktop lists; the desktop returns the credential for exactly that
origin and frame kind; the background sends it to that frame (and that `documentId`) in one
message, then drops every reference. Nothing is copied to the clipboard and no form is
submitted. A failed shortcut or menu fill is shown as the toolbar badge `!`, the popup and
`storage.session` (`lastError:<tabId>`).

## 3. Who may talk to whom

- The background accepts popup operations only on a port named `popup`, from our own
  extension id, with `sender.url` equal to our `popup.html` and no `sender.tab`
  (`router.ts`). Each message is validated with exact keys; anything else is dropped without a
  reply.
- The messages a content script may send are `captured` (from `capture.js`), from our id with a
  tab, with exactly the keys `type`, `username`, `password`, `newPassword` and
  `ancestorsSameOrigin` (a password of 1 to 1024 characters), at most 10 messages a second and
  5 captures a minute per tab, and, with passkeys on, the two passkey messages of the bridge
  (`passkey`, `passkeyAbort`; exact keys and limits, §10), at most 10 messages a second per tab
  in all. Where a capture or a passkey request came from is read from the sender, never the
  message; a passkey request is answered through `sendResponse` and a request that is not
  well-formed gets no answer.
- The content script accepts `runtime` messages only from our background (`sender.id` equals our
  id and no `sender.tab`) and validates them. `content.js` and `capture.js` never listen to
  `message`. Only the passkey bridge does (§10), and only for its own window
  (`event.source === window`); the page cannot reach `runtime` at all.
- No `onMessageExternal` or `onConnectExternal`; `externally_connectable` is `{ids: []}` on
  Chromium and absent on Firefox; no `web_accessible_resources`.

## 4. Permissions

Production manifest (`scripts/manifest.ts`, enforced by `manifest.test.ts` and `check-dist.mjs`):
`activeTab`, `scripting`, `nativeMessaging`, `contextMenus`, `storage`. `optional_host_permissions`
(`https://*/*`, `http://*/*`) exist only for the opt-in save capture and the opt-in passkey
scripts (https only) and are not granted by default; the popup gives the access back when
neither switch needs it. There are no `host_permissions` and no static `content_scripts`. The extension pages
run under `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src
'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`.

The `e2e` build adds `host_permissions` for `*.test` (and for the two patterns of the optional
permission, which a headless browser cannot grant by clicking) and `webNavigation`, so the tests
can run without a physical click; the `dev` build adds only the dev key (fixed extension id for the
native host). Neither is published.

## 5. Storage

IndexedDB in the extension's origin, read by the background only: the pairing (`pairing`, id and
`K_pair`) and `neverSave:<origin>` (sites where saving is never offered). `chrome.storage.session`
(memory): `lastFilled:<tabId>`, `lastError:<tabId>` (shown once), `pendingSave:<tabId>` (a captured
or generated login, five minutes, with the password). Never
`storage.local` or `storage.sync`. Passkey requests are kept nowhere: the worker holds only the IDs of the requests that are open (memory), and no passkey value, key or challenge is stored. A secret leaves the desktop only inside a `fill` or
`generate` response and lives in locals of one call; JavaScript cannot wipe it (THREAT_MODEL R40).

## 6. Field detection

`src/content/detect.ts` (pure; tests with a stubbed layout environment in jsdom). Candidates
are enabled, editable, fillable `input` elements that are not hidden, buttons, checkboxes,
radios, files, images, ranges or colors. A password is `type=password` or an
`autocomplete` of `current-password`/`new-password`; it is new when `autocomplete` says so, when
the form has two or more, or when its name matches new/confirm/repeat (also German). The
username is the field declared `username`/`email`, else the nearest text field before the first
password (within five levels when there is no `<form>`); search boxes are excluded. A page that
asks for the username first has one lone, plausible field. A one-time code is
`autocomplete=one-time-code`, or a code-like name with a numeric hint, or four to eight adjacent
one-character boxes (filled one digit each). The form is the one with focus, else the first with a
current password, then username-only, code, sign-up. Open shadow roots are searched to depth 8.

## 7. Running the hostile-page suite

```sh
pnpm --filter @mosaic/extension build:e2e
pnpm --filter @mosaic/extension exec playwright install chromium   # once
pnpm --filter @mosaic/extension exec playwright test e2e/hostile
```

The fixtures start Chromium with the e2e build, `--host-resolver-rules="MAP *.test 127.0.0.1"`
and `--ignore-certificate-errors`, and the hostile-site server
(`test/hostile-site/server.mjs`: `https://*.test:4843` with the test-only certificate in
`test/hostile-site/certs/`, `http://*.test:4880`). The extension's service worker exposes
`globalThis.mosaicE2E` in this build only: `reset` (the fake desktop's state), `log` (ops and
origins, never a secret), `gesture` (runs the real shortcut, menu or popup handler on a tab),
`popupState`, and a pause between probe and delivery. The fake desktop decides by exact origin
string and the HTTP rule only; the real matching is `mosaic-origin` and is tested in Rust. For
passkeys it also answers `passkeyCreate`, `passkeyGet` and `passkeyCancel` (§10.4): `passkeyDecide`,
`passkeyLog`, `passkeyPending` and `setPasskeys` are the extra hooks, and the hostile-site server
serves `/passkey/rp.html`, a single-use `/passkey/challenge` and `/passkey/verify` (the
independent verifier). Run only the passkey specs with `playwright test e2e/hostile/passkey-`.
Every page starts with a comment naming its attempts and records the values its scripts see
arrive in `window.__mosaicLeaks`.

`test.use({ evilExtension: true })` starts a separate browser that also loads
`test/evil-extension/` (another installed extension, for `other-extension.spec.ts`). The server
sends `Content-Security-Policy: sandbox allow-scripts allow-forms` for pages named
`csp-sandbox*`.

Not automated: `activeTab` gating (the e2e build has host permissions; check by hand that, in
the production build, the shortcut and the popup fill work and nothing is injected before a
gesture), and Firefox (Playwright cannot load Firefox extensions; there the capture script's own opaque-origin check stands in for `sender.origin`, which is tested in Chromium and in `src/capture/capture.test.ts` only).

## 8. The popup

A page of the extension under the CSP of §4, in `src/popup/` (React, tokens, `ext.*` strings in
German and English). It reaches the background over the port `popup` only and shows what the
background reports for the front tab. It shows no vault password and no code; the one secret
on screen is a generated password, in an element with `data-secret`. Frames: the popup names the
frame it shows (`frameTarget`), a same-origin frame needs the "Fill in the frame" click, and
the click sends that frame's id and document id back (§1); a frame of another site offers no
button. Copies and fills are announced in a live region ("Copied. Clears in 30 seconds.").
Keys: Enter fills the first match (or the row with focus), Ctrl+C, Ctrl+Shift+C and Ctrl+Alt+C
copy the user name, password and code, Esc closes the top layer (menu, then dialog).

## 9. Known limits

Found by the extension-attacker pass and not closed, because the DOM offers no reliable test or
the impact stays within the page's own origin:

- An opaque overlay with `pointer-events: none` over a field: `elementFromPoint` (and
  `elementsFromPoint`) skip such elements, so the field passes the hit test while the user sees the
  overlay. Clicks reach the field, and only that same origin gets the credential.
- Colour tricks: transparent text, background and border, or `mix-blend-mode`, make a field
  invisible without changing anything the checks read.
- Opacity is not multiplied across frame levels: a field at 0.5 in a frame at 0.5 passes
  (each level is checked on its own).
- Off screen, any `clip-path` counts as hiding, and any `mask-image` on a field or ancestor
  counts as opacity 0 everywhere: such forms fill only once they are scrolled into view (or not
  at all with a mask).
- A field in a closed shadow root is not seen and not filled.
- Save capture reacts to any trusted click on a submit-like control of a form (`button`, submit
  and button inputs, `[role=button]`) and to Enter in its boxes, and takes what the visible fields
  hold. A page can therefore make a user's real click (on a "Show password" toggle, or on anything it
  marked `role=button`) report the values it put there, or a password typed only in part; the
  next submit within 2 s is then dropped by the throttle. Nothing is saved without the user's
  click in the popup and the desktop's own dialog (R44), and only for the page's own origin.
- A capture replaces a pending save of the same tab, also one of a generated password (same
  origin only). A page can stop its own capture (a capturing `click` listener on `window` that
  calls `stopImmediatePropagation` before ours runs): denial of service against itself.
- "Never for this site" is stored per exact origin: another scheme, port or subdomain, or the
  trailing-dot form of the host (`bank.example.`), is another origin and is offered again.
- With save capture on, the granted `https://*/*` and `http://*/*` also lift the `activeTab`
  bound of the gesture-time injection (every frame of every site is reachable). Nothing is
  injected or filled without a gesture all the same.
- When the target is a frame of another site, the popup lists that frame's logins (titles and
  user names, never a secret) with every Fill disabled; only the user sees them.
- With passkeys on, a page that is already open keeps its shim after the switch is turned off,
  until it reloads; the background then answers a fallback to every request. A page can learn from
  timing whether Mosaic holds a passkey for its own rp ID (R50), and can open a desktop dialog
  for its own origin without a user gesture (the user can say no; one at a time, rate limited).
- Firefox registers MAIN-world scripts from version 128 (documentation only, not covered by the
  Playwright suite).
- A page can stop fills on itself (for example `<img name="querySelectorAll">` or a focused field
  in a cross-origin ad frame); that is denial of service against itself, not a leak.

## 10. Brief §4 map

Every item of brief §4 with its mitigation and tests. "Auto" means the test runs in CI (the
`extension` job, or the Rust jobs for `crates/*` and `apps/desktop/src-tauri`); "manual" means no
automated test can show it and the check is on the list in `docs/PROGRESS.md` ("M4 manual checks").

| Brief §4 item | Mitigation | Tests |
|---|---|---|
| 1. No autofill on page load; a trusted gesture only | No static content script and no host permissions; the script is injected inside the click, shortcut or menu handler of the browser's own UI (§2). Nothing listens to page events, so no page can fake the gesture | Auto: `e2e/hostile/autofill-on-load.spec.ts`, `e2e/hostile/synthetic-events.spec.ts`, `e2e/hostile/race-after-probe.spec.ts`, `scripts/manifest.test.ts`, `scripts/check-dist.mjs`. Manual: `activeTab` gating on the production build (the e2e build has host access), Firefox |
| 2. Exact origin (scheme, host, port); eTLD+1 only per item and off by default; punycode and homoglyph warning | `mosaic-origin` in the desktop decides: exact origin string, eTLD+1 only for a URL marked so, `xn--` and look-alike warnings; the extension reports the frame's own origin and shows the Unicode host with the desktop's warning, with no "fill anyway" | Auto: `crates/origin/src/tests.rs` (exact, http/https, eTLD+1, IP and single labels, look-alikes, proptest), `apps/desktop/src-tauri/src/browser_tests.rs` (`fill_returns_a_credential_only_for_a_matching_origin`, `list_for_origin_sorts_caps_and_warns`), `e2e/hostile/lookalike.spec.ts`, `src/shared/origin.test.ts`, `e2e/hostile/url-tricks.spec.ts` |
| 3. No fill into cross-origin iframes; same-origin only after confirmation | `decideFrame` and `frameAncestry` (every ancestor and every `<iframe>` element on the way); `confirmFrame` bound to frame id and document id; the desktop refuses `origin != topOrigin` and a `sameOriginFrame` without the flag | Auto: `src/background/frames.test.ts`, `src/content/frame.test.ts`, `src/background/fill.test.ts`, `e2e/hostile/cross-origin-iframe.spec.ts`, `same-origin-iframe.spec.ts`, `nested-a-b-a.spec.ts`, `sandboxed-srcdoc.spec.ts`, `csp-sandbox.spec.ts`, `about-blank.spec.ts`, `hidden-frame.spec.ts`, `forged-frame-id.spec.ts`, `popup-binding.spec.ts`, `browser_tests.rs` (frame kinds in `fill_returns_a_credential_only_for_a_matching_origin`) |
| 4. Clickjacking: no injected UI a page can overlay; opacity, overlay and pointer-events tricks | The choice is "no inline UI at all, toolbar popup only" (ADR-0035 item 2). The field test `isFillable` (size, combined opacity, `filter`, `mask-image`, clip, hit test at five points) runs at probe and at fill time; capture only reports from visible frames | Auto: `e2e/hostile/overlay-occlusion.spec.ts`, `hidden-fields.spec.ts`, `hidden-css-tricks.spec.ts`, `hidden-frame.spec.ts`, `capture-frames.spec.ts`, `src/content/visibility.test.ts`, `src/popup/strings.test.ts`. Open, not detectable: `pointer-events: none` overlays and colour tricks (§9) |
| 5. Content scripts receive only the single credential being filled, never the vault or item lists | Lists, searches and matches are answered to the popup only (titles and user names, never a secret). A fill is one message with one credential for one frame and one document; the background drops every reference afterwards. The popup and content script cannot reach the native port; the capture script sends one message type | Auto: `src/shared/protocol.test.ts`, `src/background/router.test.ts`, `src/background/fill.test.ts`, `src/content/fill.test.ts`, `src/popup/logins.test.tsx`, `e2e/hostile/spoof-messages.spec.ts`, `e2e/hostile/other-extension.spec.ts`, `browser_tests.rs` (`search_returns_at_most_twenty_logins_without_secrets`, `only_fill_and_copy_count_as_activity_and_only_fill_generate_and_otp_carry_secrets`) |
| 6. Minimal permissions; host permissions at runtime where possible | `activeTab`, `scripting`, `nativeMessaging`, `contextMenus`, `storage`; no `host_permissions`; the only site access is the optional pair for the opt-in save capture, requested in the popup click (§4) | Auto: `scripts/manifest.test.ts`, `scripts/check-dist.mjs` (fails the production build on any change). Manual: the browser's permission prompt for the capture on Chrome, Edge and Firefox 128+ |
| 7. Native-messaging channel authenticated and bound to the installed desktop; unknown origins and extension IDs rejected | Host manifests list only our extension IDs (`allowed_origins`, `allowed_extensions`); the host checks the caller argument against the pinned IDs; pairing is approved once in the desktop (code, 1 s guard); the channel is a mutual HMAC proof over `K_pair` and AEAD frames with strict sequence numbers (CRYPTO_SPEC §15); the endpoint is per user, with a lease and peer checks (ADR-0036) | Auto: `crates/core/src/browser_channel.rs` (unit tests), `crates/native-messaging/src/ids.rs` (`chromium_callers`, `firefox_callers`), `src/handshake/tests.rs`, `src/endpoint/lease.rs`, `src/relay/tests.rs`, `crates/native-messaging/tests/host_binary.rs`, `browser_tests.rs` (`the_handshake_needs_the_right_key_and_a_known_pairing`, `a_request_before_the_handshake_closes_the_connection`, `every_op_outside_the_allowlist_is_an_invalid_request`, the pairing tests), fuzz targets `envelope`, `preauth_message`, `channel_open`. Manual: the registry install and pairing on Windows Chrome and Edge, Firefox 128+, macOS and Linux Chrome; Flatpak and Snap browsers cannot run native hosts (unsupported) |
| 8. Refuse HTTP fills unless the user overrides per item | The desktop refuses an http page unless the item's URL allows it (loopback excepted), never matches https to http, and the extension skips `blocked` matches | Auto: `crates/origin/src/tests.rs` (`http_and_https_never_match_each_other`, `http_pages_need_the_override_unless_loopback`), `e2e/hostile/http-page.spec.ts`, `browser_tests.rs` |

Verified only by hand (also listed in `docs/PROGRESS.md`):

- `activeTab` gating on the production build: the shortcut and the popup fill work, nothing is
  injected before a gesture.
- Firefox 128+ as a temporary add-on: pairing, fill, one-time code, save (Playwright cannot load
  Firefox extensions).
- The Windows registry install (`reg.exe`), and the Unix manifest paths on macOS and Linux.
- Keyboard-only and screen-reader use of the popup (axe runs automatically in light and dark).

## 11. Passkeys

ADR-0040 (decision), ADR-0039 (authenticator), `NATIVE_MESSAGING.md` §5.2 (the ops),
`THREAT_MODEL.md` §4.3 and R47, R50, R51. Off by default; the switch "Use Mosaic for passkeys"
is in the popup's More menu.

### 11.1 Trust analysis

Two scripts exist only while the switch is on: `passkey-main.js` runs in the page's own
JavaScript world and replaces `CredentialsContainer.prototype.create` and `.get`;
`passkey-bridge.js` runs in the isolated world and is the only extension code that listens to
page messages. Both are registered with `scripting.registerContentScripts` after the user granted
the optional access to `https://*/*` (never in the manifest; `check-dist.mjs` fails if the
manifest names them), at `document_start`, top frame only (`allFrames: false`).

**The shim is not trusted.** It lives in the page's world: the page can replace it, route around
it, read its messages and forge requests and answers. This is acceptable because everything the
page can do through the shim it can already do with `navigator.credentials` for its **own**
origin, and a forged answer only fools the page that forged it. The trust boundary is
**bridge, background, desktop**:

- **Origin and frame come from the browser.** The background takes the origin from
  `MessageSender.origin` (Chromium) or the origin of `MessageSender.url` (Firefox); it must equal
  the `window.origin` the bridge read in the isolated world (a view the page cannot redefine), be
  https, and be the origin of the tab's current URL. Only `frameId` 0 is served. A cross-origin
  child frame cannot post into the top window as `event.source === window`, and subframes get no
  script at all.
- **The request ID is made in the background** (16 random bytes as hex). The page's counter is
  only a handle, stored with tab, document and frame, so one page cannot abort another's request.
- **The desktop decides.** It checks the relying party ID with the public suffix list
  (`webauthn_rp_id`: at or under the origin's registrable domain, ADR-0040), builds
  `clientDataJSON` itself from the origin the browser reported, and shows a dialog for every
  create and sign-in.
- **Anything not handled goes to the browser's own WebAuthn**, with the page's original options
  object and `this`: setting off, locked, not paired, no desktop, no matching passkey, unsupported
  options, "Use another device", a request the desktop's rate windows refuse (`rateLimited`: the
  windows belong to the pairing, so one site could spend them for every other), a page that is
  not focused and visible, and a bridge that does not answer within 1 s. In the last case the shim
  also posts `mosaic-passkey-abort`, so a bridge that was only late closes the desktop dialog it
  may already have asked for.
- **A request is held from the moment it arrives.** The background enters it under
  `tab:document:pageRequestId` before it checks the tab and the link; an abort, a navigation or a
  closed tab that comes meanwhile marks it, and the desktop is then never asked.

The bridge listens only to its own window (`event.source === window`), accepts only the shim's
exact message shapes within the shared limits (`src/shared/passkey-limits.ts`), keeps at most 2
requests per page in flight, and sends the background `{kind: "passkey", op, pageRequestId,
origin, options}` or `{kind: "passkeyAbort", pageRequestId}`. The router takes them only from our
content scripts (a tab, our id), at most 10 messages a second per tab.

### 11.2 Attack, mitigation, test

| Attack | Mitigation | Test |
|---|---|---|
| A page asks for a passkey of another site (`rpId: "bank.test"` on `evil.test`) | The desktop is told the browser's origin and checks the rp ID in Rust; `rpIdMismatch` is a fallback, the browser's own WebAuthn then throws `SecurityError`; no credential is made. (The fake desktop of the e2e build checks host equality or dot-suffix only, labelled as a fake; the PSL rule is tested in Rust.) | `e2e/hostile/passkey-rpid-spoof.spec.ts`, `browser_tests.rs` |
| A page forges a shim request with an origin of its choice, or for another site's rp ID | An extra member (`origin`) is not the shim's shape and is dropped without a reply; a well-formed forgery reaches the desktop as the page's own origin and is refused or needs the user's click | `e2e/hostile/passkey-shim-tamper.spec.ts`, `src/passkey/bridge.test.ts` |
| A page replaces `postMessage`, `addEventListener` or `create`/`get` before or after the shim | The shim takes the originals at `document_start`, before page scripts run; replacing `get` afterwards only reaches the page's own function and never the desktop | `e2e/hostile/passkey-shim-tamper.spec.ts` |
| A page (or another tab) forges an abort or an answer for another request | Aborts and answers are scoped to the sender's tab, document and frame; the background looks a request up by `tab:document:pageRequestId` from the sender; a forged answer only settles the forging page's own promise | `e2e/hostile/passkey-shim-tamper.spec.ts`, `src/background/passkey.test.ts` |
| A frame (same-origin or cross-origin) calls WebAuthn hoping Mosaic answers | No script in subframes; the background refuses `frameId` other than 0; the frame's call goes to the browser | `e2e/hostile/passkey-iframe.spec.ts`, `src/background/passkey.test.ts` |
| An http page, an opaque origin, or a tab that moved to another site gets a passkey | Only https origins are served, the sender origin must equal the bridge's and the tab's current origin; navigation cancels the open request | `src/background/passkey.test.ts` |
| A captured assertion is replayed at the relying party | The challenge is the relying party's single-use value; two sign-ins never share one; Mosaic builds `clientDataJSON` per request with the challenge it was given | `e2e/hostile/passkey-replay.spec.ts` |
| A request the user cannot see (a background tab) opens a dialog | The bridge answers a fallback unless the document has focus and is visible | `src/passkey/bridge.test.ts` |
| A flood of requests | 2 per page in the bridge, 2 per tab and 10 messages a second per tab in the background, 4 requests in flight (`passkeyCancel` excepted), and the desktop's own limits and one dialog at a time (`busy`) | `src/passkey/bridge.test.ts`, `src/background/router.test.ts`, `src/background/passkey.test.ts` |
| The page aborts, or the tab goes away, with a dialog open | The shim sends `mosaic-passkey-abort`; the background sends `passkeyCancel` for exactly that request ID; a closed, replaced or navigating tab cancels its open requests | `e2e/hostile/passkey-excluded-abort.spec.ts`, `src/background/passkey.test.ts` |
| Registration of a credential Mosaic already has | `excludeCredentials` that hit open an informing dialog and end with `InvalidStateError` | `e2e/hostile/passkey-excluded-abort.spec.ts` |
| Mosaic breaks a site it cannot serve (conditional mediation, security keys, PRF) | These and any size beyond the limits are left to the browser, and "Use another device" is in the dialog | `e2e/hostile/passkey-fallback.spec.ts`, `src/passkey/main-world.test.ts` |
| The switch is off but a page keeps its shim | The background checks the setting on every request and answers a fallback; a reload removes the shim | `e2e/hostile/passkey-fallback.spec.ts` |
| A page spends the desktop's per-pairing passkey windows (3 creates, 10 sign-ins a minute) with forged requests it refuses silently, so that another site's call fails for a minute | `rateLimited` is a fallback, not `NotAllowedError`: the other site's call goes to the browser's own WebAuthn | `e2e/hostile/passkey-budget.spec.ts`, `src/background/passkey.test.ts` |
| A page aborts in the same task that started the call (or the tab moves, or closes) before the background has asked the desktop, so the abort finds nothing and a dialog opens for a call nobody waits for | The request is held from its arrival; a cancel before the desktop was asked marks it, and it is never sent | `e2e/hostile/passkey-lifecycle.spec.ts`, `src/background/passkey.test.ts` |
| A page keeps its main thread busy past the shim's 1 s wait: the shim hands the call to the browser while the bridge has already passed it on, and two prompts are open (approving Mosaic's would store a passkey the site never gets) | On the timeout the shim posts `mosaic-passkey-abort` before it falls back; the bridge passes it on and the request is cancelled | `e2e/hostile/passkey-lifecycle.spec.ts`, `src/passkey/main-world.test.ts` |
| A frame of another site, a sandboxed `srcdoc` frame, or a popup's opener posts well-formed requests into the window the bridge serves (directly, with a transferred `MessagePort`, or by reaching into its realm) | `event.source` is the posting window, not the bridge's; a cross-origin realm is out of reach | `e2e/hostile/passkey-forge-frames.spec.ts` |
| The tab reloads or moves to another site while the desktop dialog is open, hoping the answer reaches the new page | The navigation cancels the request; the answer belongs to the old document's message channel, and the new page hears nothing | `e2e/hostile/passkey-lifecycle.spec.ts`, `src/background/passkey.test.ts` |
| The scripts reach more than the top frame of https pages | Registered with `matches: ["https://*/*"]`, `allFrames: false`, `document_start`, no `matchOriginAsFallback` or `matchAboutBlank`; only with the site access, gone when the switch goes off or the access is taken back | `src/background/passkey-registry.test.ts` |
| Test code in the shipped package | `FakePasskeys`, `passkeyDecide` and `passkeyPending` join the strings `check-dist.mjs` forbids; both passkey scripts must be shipped, absent from the manifest, check `event.source` wherever they listen, and use no extension API but `runtime.sendMessage` | `scripts/check-dist.mjs` |

### 11.3 Limits

- **`busy` is still an error.** While a page's Mosaic dialog is open (the desktop shows one at a
  time), another site's call ends with `NotAllowedError`. The dialog is visible and names the page,
  and the user can close it.
- **The `passkeyCancel` window (20 a minute per pairing) can be spent** by a page that cycles
  requests and aborts. Another site's abort or navigation then leaves its dialog open until the user
  closes it or 300 s pass; the answer goes nowhere, but a registration approved there is stored.
- **A rate-limited call goes to the browser.** After a page spent the windows, a user whose passkey
  is in Mosaic sees the browser's own prompt for up to a minute.
- **A page can break its own path**: block the bridge's acknowledgements with a capturing
  `message` listener and `stopImmediatePropagation`, or clobber `document.hasFocus` (the DOM is
  shared with the isolated world). Its own calls then go to the browser; nothing reaches another
  origin.
- **Same-origin frames** can call the top window's `navigator.credentials` through `parent`; that
  call is the top's, served as the top's origin, as the browser's own WebAuthn treats it.
- **Not covered by the suite:** Firefox (no `documentId`: requests are told apart by tab and page
  number only, and a navigation cancels them first), the back/forward cache (the message channel
  closes when the page is stored; the restored page's call then falls back), prerendered pages
  (hidden, so the bridge falls back), and real desktop dialogs (the fake desktop mirrors the
  rate windows and the one-dialog rule).

### 11.4 The e2e fake desktop

`src/background/e2e-passkey.ts` answers `passkeyCreate`, `passkeyGet` and `passkeyCancel`, within
the desktop's per-pairing rate windows (`NATIVE_MESSAGING.md`), with a software authenticator in `test/webauthn/authenticator.ts` (WebCrypto P-256, the encodings of
CRYPTO_SPEC §16 written separately). `src/passkey/authenticator.test.ts` checks its output with
the independent verifier (`test/webauthn/verifier.ts`), which also stands behind the hostile
site's `/passkey/verify`. The hooks `passkeyDecide` (the answer in the pretend dialog),
`passkeyLog` (request IDs, origins, rp IDs, results) and `passkeyPending` exist in the e2e build
only. A CDP virtual authenticator (`WebAuthn.addVirtualAuthenticator`) stands for the browser's
own WebAuthn, so a fallback is observable.
