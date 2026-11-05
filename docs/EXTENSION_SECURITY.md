# Extension security

Status: v1 (M4 track T3: the core and the hostile-page harness). Save capture, the popup
screens and a mapping of every brief §4 item follow in T4 and T5. Normative companions:
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
| Look-alike or homoglyph host | The desktop matches the exact origin; the extension reports the frame's own origin, never a page-supplied one. An `xn--` host is shown in Unicode (`unicodeHostOf`) with the desktop's `idn` warning; there is no "fill anyway" | `e2e/hostile/lookalike.spec.ts`, `src/shared/origin.test.ts` |
| Fill over plain HTTP | The desktop refuses an http page unless the item's URL allows it, and never matches an https URL to an http page; the extension skips `blocked` matches | `e2e/hostile/http-page.spec.ts` |
| Hidden fields: opacity 0, off screen, 0 or 1 px, clipped, `display: none`, `visibility: hidden`, transparent parent | `isFillable`: displayed (`checkVisibility`), at least 4 x 4 px, combined opacity of field and ancestors at least 0.2, inside the document, and on screen at one of five points the field (or its label) is the top element. Runs at probe and again at fill | `src/content/visibility.test.ts`, `e2e/hostile/hidden-fields.spec.ts` |
| Field changed or injected between the probe and the fill | The content script detects again before writing; fields that changed type, were renamed, or are not fillable now receive nothing; an empty result is `noFields`, not success | `src/content/detect.test.ts`, `e2e/hostile/type-swap-and-late-inject.spec.ts` |
| Navigation between the probe and the fill | Delivery is pinned to the probed `documentId` (Chromium); the content script compares `location.origin` with the expected origin (`originChanged`), re-checks ancestry and visibility | `e2e/hostile/navigate-after-probe.spec.ts` |
| DOM clobbering: `<input name="elements">`, `name="type"` and the like | Fields are found with `querySelectorAll("input")`; attributes are read with `getAttribute`; the form of a field is `input.form`, never `form.elements` | `src/content/detect.test.ts` |
| Spoofed extension messages from a page or another extension | See §3 | `src/background/router.test.ts`, `e2e/hostile/spoof-messages.spec.ts` |
| A page reads the extension's files or pages | No `web_accessible_resources`; `externally_connectable: {ids: []}` | `scripts/manifest.test.ts`, `e2e/hostile/spoof-messages.spec.ts` |
| A framework ignores a programmatic value (React, Vue) | The value is set with the DOM's own setter from the isolated world, then `input` (composed `InputEvent`) and `change` fire | `src/content/fill.test.ts`, `e2e/hostile/react-controlled.spec.ts` |
| Test code in the shipped package | `excludeE2e` replaces `src/background/e2e.ts` outside `--mode e2e`; `check-dist.mjs` fails the build on `mosaicE2E`, `e2eTabId`, external listeners, `host_permissions`, `web_accessible_resources`, `key` or a changed permission list | `scripts/check-dist.mjs`, `scripts/manifest.test.ts` |
| Malformed or hostile replies from the host | Every message is validated with exact keys and the fixture samples of the Rust side; unknown keys, versions and events are dropped | `src/shared/protocol.test.ts` |

## 2. Gestures and the fill

A fill starts only from:

1. a click in the toolbar popup (`fill`, `fillOtp`, `fillGenerated`; the popup is T4),
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
- The single message a content script may send is `captured` (T4), from our id with a tab,
  at most 10 messages a second and 5 captures a minute per tab.
- The content script accepts `runtime` messages only from our background (`sender.id` equals our
  id and no `sender.tab`) and validates them. It never listens to `message`, and the page
  cannot reach `runtime` at all.
- No `onMessageExternal` or `onConnectExternal`; `externally_connectable` is `{ids: []}` on
  Chromium and absent on Firefox; no `web_accessible_resources`.

## 4. Permissions

Production manifest (`scripts/manifest.ts`, enforced by `manifest.test.ts` and `check-dist.mjs`):
`activeTab`, `scripting`, `nativeMessaging`, `contextMenus`, `storage`. `optional_host_permissions`
(`https://*/*`, `http://*/*`) exist only for the opt-in save capture (T4) and are not granted
by default. There are no `host_permissions` and no static `content_scripts`. The extension pages
run under `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src
'self'; connect-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`.

The `e2e` build adds `host_permissions` for `*.test` and `webNavigation`, so the tests can run
without a physical click; the `dev` build adds only the dev key (fixed extension id for the
native host). Neither is published.

## 5. Storage

IndexedDB in the extension's origin, read by the background only: the pairing (`pairing`, id and
`K_pair`). `chrome.storage.session` (memory): `lastFilled:<tabId>`, `lastError:<tabId>`. Never
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

Not automated: `activeTab` gating (the e2e build has host permissions; check by hand that, in
the production build, the shortcut and the popup fill work and nothing is injected before a
gesture), and Firefox (Playwright cannot load Firefox extensions).
