# Extension security

Status: v2 (M4 track T4: save capture and the popup added to T3's core and hostile-page
harness). A mapping of every brief §4 item follows in T5. Normative companions:
`THREAT_MODEL.md` §4.3, `NATIVE_MESSAGING.md`, ADR-0035 (architecture and gestures), ADR-0036
(channel) and ADR-0037 (fill policy).

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
| Test code in the shipped package | `excludeE2e` replaces `src/background/e2e.ts` outside `--mode e2e`; `check-dist.mjs` fails the build on `mosaicE2E`, `e2eTabId`, `FakeDesktop`, `pauseAfterProbe`, external listeners, `setAccessLevel` (content scripts keep no access to `storage.session`, where pending saves wait), `host_permissions`, `content_scripts`, `web_accessible_resources`, `optional_permissions`, `key`, a changed permission or optional-host list, or an `externally_connectable` other than `{ids: []}` (Chromium) / absent (Firefox) | `scripts/check-dist.mjs`, `scripts/manifest.test.ts` |
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
- The single message a content script may send is `captured` (from `capture.js`), from our id
  with a tab, with exactly the keys `type`, `username`, `password`, `newPassword` and
  `ancestorsSameOrigin` (a password of 1 to 1024 characters), at most 10 messages a second and
  5 captures a minute per tab. Where the capture came from is read from the sender, never the
  message.
- The content script accepts `runtime` messages only from our background (`sender.id` equals our
  id and no `sender.tab`) and validates them. It never listens to `message`, and the page
  cannot reach `runtime` at all.
- No `onMessageExternal` or `onConnectExternal`; `externally_connectable` is `{ids: []}` on
  Chromium and absent on Firefox; no `web_accessible_resources`.

## 4. Permissions

Production manifest (`scripts/manifest.ts`, enforced by `manifest.test.ts` and `check-dist.mjs`):
`activeTab`, `scripting`, `nativeMessaging`, `contextMenus`, `storage`. `optional_host_permissions`
(`https://*/*`, `http://*/*`) exist only for the opt-in save capture and are not granted
by default. There are no `host_permissions` and no static `content_scripts`. The extension pages
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
`storage.local` or `storage.sync`. A secret leaves the desktop only inside a `fill` or
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
string and the HTTP rule only; the real matching is `mosaic-origin` and is tested in Rust.
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
- A page can stop fills on itself (for example `<img name="querySelectorAll">` or a focused field
  in a cross-origin ad frame); that is denial of service against itself, not a leak.
