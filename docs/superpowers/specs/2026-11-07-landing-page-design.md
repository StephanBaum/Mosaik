# Landing page design concept

Status: concept, awaiting the owner's approval; nothing is built yet. Date: 2026-11-07.

- **Scope:** `apps/web`, the M9 landing page with verifiable downloads.
- **Inputs:**
  - `marketing/` (positioning, landing copy EN/DE)
  - `docs/design/`
  - `apps/desktop/src/{holo,reveal}`
  - `packages/design-tokens`
  - ADR-0014, ADR-0027, ADR-0032
- **Name:** "Mosaic" is a working title. Every name and domain string goes through `packages/i18n`.

## 1. Concept and narrative

**Big idea: colour means only you can read it.** The vault is a mosaic of tiles. Each tile is one item, encrypted on its own, as CRYPTO_SPEC says.

- **On your device:** the tiles are the app's **holo foil**, vivid and reacting to light.
- **Leaving the device:** a tile turns into **ciphertext**, a crisp monochrome block of Crockford Base32 glyphs.
- **On the server:** only grey blocks, plus the labels listed in spec §12.
- **Coming back:** colour returns only when both secrets meet on your device, the master password and the Secret Key.

One rule runs through every section: **foil means plaintext on your device; mono means ciphertext anywhere else.** The hero teaches this rule without words. Because foil carries meaning, it is never used as decoration.

**The scroll story** is an arc: the tiles leave, they are stored, and they come back, or never do. It reorders section 2 of the copy:

| # | Chapter | Feeling | Copy |
|---|---|---|---|
| 1 | Hero | A living foil mosaic answers the cursor: *this is mine.* | §1 |
| 2 | Locked before it leaves | A login breaks into tiles that lose their colour on the way out. | §2 "Everything is locked…" |
| 3 | What a stolen server holds | Noise, and a glass lens showing what only the owner sees. | §2 "A stolen server…", §4 |
| 4 | Two secrets | Both keys meet; a foil ripple re-colours the tiles and the item reassembles. | §2 "Two secrets…" |
| 5 | No backdoor. On purpose. | Stillness: big type, no motion. | §2 "No backdoor…" |
| 6–10 | What works today · pre-audit · pricing and teams · download · FAQ and footer | Honest, calm. | §3–§9 |

**Copy tightening (claims unchanged):**

- **H1:** "Two secrets. Only you hold both." / "Zwei Geheimnisse. Beide hast nur du."
  - It is true today and it carries the concept.
  - "Your vault. Your cloud. Your keys." waits until M6 sync ships, then sits above Download.
- **Subhead.** The current one offers folder sync without the **(coming)** label, which breaks the copy's own rule. Proposed:
  - EN: "A password manager with a published crypto spec and threat model. Sync through your own cloud folder or ours, arriving with M6. No telemetry."
  - DE: "Ein Passwortmanager mit veröffentlichter Kryptospezifikation und Bedrohungsmodell. Synchronisierung über deinen Cloud-Ordner oder unseren Server kommt mit M6. Keine Telemetrie."
- **Trust strip:** stays in the hero, unchanged.
- **Chapter heads (EN / DE):**
  - "Locked before it leaves." / "Verschlüsselt, bevor es dein Gerät verlässt."
  - "What a stolen server holds." / "Was ein gestohlener Server enthält."
  - "One you remember. One your device keeps." / "Eins merkst du dir. Eins behält dein Gerät."
  - "No backdoor. On purpose." / "Keine Hintertür. Mit Absicht."
  - "What works today." / "Was heute schon geht."
  - "Pre-audit. Here is everything we publish." / "Noch ohne Audit. Das veröffentlichen wir."
- **No competitor table.** It invites claims that positioning §7 forbids.

**References (learn, never copy):**

1. **Igloo Inc** (Awwwards Site of the Year 2024): one procedural material carries the whole identity. Ours is the foil. We leave out the sound and the heavy 3D.
2. **Lando Norris by OFF+BRAND** (listed as Site of the Year 2025): type is the layout and timing is mapped to scroll. We leave out the chaos.
3. **Linear's marketing site**: near-monochrome, with the live product UI as the hero.
4. **Codrops, "Scroll-Revealed WebGL Gallery"** (Feb 2026): shader reveals tied to DOM sections. We use a fixed canvas in a sticky stage, which needs no smooth-scroll library.
5. **2026 Site of the Day trend reports** (directional): scroll chapters with one idea each. No camera fly-throughs.

## 2. Art direction

**Palette: zero new hex values.** Everything comes from `packages/design-tokens`.

| Role | Dark (primary) | Light |
|---|---|---|
| Page / stage | `bg` #09090B / `bgSubtle`, `surface` | #FFFFFF / `bgSubtle` |
| Text | `text`, `textSecondary`, `textMuted` | same tokens |
| Ciphertext tile | `surface` fill, `textMuted` glyphs | `bgSelected` fill, `textSecondary` glyphs |
| "Coming" tiles | dashed `faint` outline | same |
| Avatars, `::selection` | `vaultColors`; selection #A5B4FC on #09090B text | same |
| Foil | shader only, never a CSS gradient (ADR-0014) | with the ADR-0027 still lift |

**Dark is primary:** it is designed, reviewed and used for the OG image first, because foil and glass are most vivid on near-black. The page still follows `prefers-color-scheme` like the app, with no toggle, storage or inline script. The light theme is fully designed and passes AA.

**Type:** self-hosted `.woff2`, latin and latin-ext only.

- **DM Sans variable** (wght 100–1000, opsz 9–40), from `@fontsource-variable/dm-sans` (new dependency).
  - License: OFL 1.1 with no Reserved Font Name, so self-hosting and subsetting are allowed.
  - Use: the kinetic type needs the variable axes.
- **IBM Plex Mono** 400/500, from `@fontsource/ibm-plex-mono` (already in the workspace).
  - License: OFL 1.1 with the Reserved Font Name "Plex".
  - Ship fontsource's files unchanged and never re-subset them.
- **Loading:** preload only DM Sans latin. Use `font-display: swap` with a metric-matched local fallback (`size-adjust` plus ascent and descent overrides), so the swap moves nothing.

| Style | Size | Line / tracking / weight |
|---|---|---|
| display | clamp(3rem, 1.6rem + 6.2vw, 10rem); DE min 2.75rem | 0.95 / −0.035em / 640, opsz 40 |
| h2 | clamp(2.25rem, 1.5rem + 3.2vw, 5rem) | 1.0 / −0.03em / 600 |
| h3 | clamp(1.5rem, 1.2rem + 1vw, 2rem) | 1.15 / −0.015em / 600 |
| lead / body | clamp(1.125rem, 1rem + 0.4vw, 1.375rem) / 18 px | 1.5 / 1.6 / 400 |
| label | 12 px Plex Mono, uppercase | 16 px / +0.08em / 500 |
| data | 14 px Plex Mono, tabular | 20 px |

**German** runs about 30 % longer.
- `hyphens: auto`, with `lang` set.
- Balanced headings and `text-wrap: pretty` on paragraphs.
- Body width capped at 68ch.
- Buttons grow; they never truncate.
- Every headline is tested in DE at 360 px. "Geheimnisse." must fit 328 px at 44 px.

**Kinetic type, five moments only:**
1. The H1 lines rise from a mask, and "both" / "beide" settles from weight 400 to 720 as the mosaic lands.
2. The field values in chapter 2 dissolve into Base32. Headlines never scramble.
3. The Secret Key resolves character by character.
4. In chapter 5, each word goes from weight 300 to 700 as it crosses the viewport centre.
5. The 404 path scrambles once.

**Grid:**
- **Columns:** 12 from 900 px, 8 from 600 px, 4 below. Gutters 24 px (16 px on phones). Margins `clamp(16px, 4vw, 64px)`.
- **Widths:** content max 1280 px, stages max 1440 px. The hero mosaic bleeds to the edge up to 2560 px.
- **Spacing and radii:** the 8 px token scale; radius 12 for tiles and cards, 14 for panels, pill for buttons.

**Imagery:** no stock photos, screenshots, padlocks or shields. Product visuals are live HTML built from tokens and the app's icons.

## 3. Signature moments

The mosaic, the holo buttons, a baked foil strip and the glass band share one WebGL2 context (ADR-0027 item 6).

### M1. The living mosaic (hero)

**What you see:**
- Rounded foil tiles bleed off the right edge:
  - 8×7 tiles of 72 px with 8 px gaps at 1440 px wide;
  - 9×9 tiles of 96 px at 2560 px;
  - an 8×3 strip of 40 px tiles on phones.
- One tile in five carries a dark etched item glyph (key, card, note, Wi-Fi, terminal).
- Each tile has its own seed, so the colour varies across the mosaic.

**Triggers:**
- **Load:** a diagonal sweep starts from the logo corner. Each tile animates opacity 0→1 and scale 0.96→1 over 400 ms with `ease-enter`, 18 ms after the previous one. The whole sweep takes at most 900 ms.
- **Pointer:** tiles within about 2.5 tile pitches (Gaussian σ) shift their foil `tilt` and specular toward the cursor, eased as in `holo/motion.ts`. **Tiles never rotate; only the light moves.**
- **Click or tap:** the owner's press ripple crosses the mosaic, with 1 unit = 1 tile pitch. The values are `RIPPLE` from `holo/effects.ts`: speed 4.7, wavelength 1.3, runout 0.7, duration 1.4 s, outline 0.05, refraction 0.032.
- **Idle:** the ambient sheen crosses about every 12.5 s. The `max(sheen, 0.55)` floor keeps the foil from going grey.

**Technique:**
- One full-canvas fragment pass. Per pixel it finds the tile cell and local coordinates, applies a rounded-square SDF mask, then runs `foil()` and `wavelet()`.
- Those functions are **copied verbatim from `apps/desktop/src/holo/shader.ts`**. A vitest parity test pins them to the desktop source.
- The canvas reserves its aspect box and starts after first paint. LCP is the H1 text.

**Fallbacks:** flat `surface` tiles with glyphs (no WebGL2, lost context, no JS); a cached still (reduced motion). `?tune` (dev only) shows sliders with "copy values", like the study pages.

### M2. Locked before it leaves (pinned 320 vh, 260 vh on phones, sticky stage 100 svh)

**What you see:**
- Left, "This device": a login card in the app's detail style, 440 px wide. The data is fictional: "Northwind Mail", `ada@northwind.example`, a password, a TOTP code.
- Right, "Server": an empty column, linked to the card by a hairline wire.

Progress `p` is a pure function of scroll, so scrolling back reverses the sequence exactly.

| p | Beat |
|---|---|
| 0–0.15 | Fields highlight one by one (`bgSelected`). |
| 0.15–0.40 | A 2 px baked foil line sweeps left to right (`ease-standard`). Behind it the card splits into 6×4 tiles. Each tile dissolves into Base32 through a **4×4 Bayer ordered dither**: orderly, crisp, no dust, no blur. |
| 0.40–0.70 | Tiles fly in reading order along shallow cubic paths, 0.012 p apart, each over 0.18 p with `ease-enter`. They shrink from 56 to 28 px and stack into one record. No trails, no particles. |
| 0.70–0.90 | The §12 metadata types in: ID, size, time, device. Four more records slide in. Caption: "This is all a stolen server would hold." |
| 0.90–1 | Hold, then unpin. |

**Technique:** Canvas 2D. The card is drawn from known data after `document.fonts.ready`, and the glyph tiles are pre-rendered `ImageBitmap`s, about 60 sprites moved by transforms. The rAF loop runs only while the stage is visible.

**The bytes** come fresh from `crypto.getRandomValues` and carry the label: "Illustration. The bytes are random, generated in your browser. The real record format is in the crypto spec."

**Fallback** (reduced motion and no JS): a designed four-frame storyboard (card → tiles → ciphertext → server record), with the metadata as a real `<table>`.

### M3. What a stolen server holds (glass-lens toggle)

**What you see:**
- A 600 px server panel with five noise records and their metadata.
- A `radiogroup` switch: "Server's view | Your view".
  - **Your view:** grows the app's **liquid-glass lens** from the click point (from the control's centre after a key press). It reveals the same records decrypted: avatars, titles, URLs and usernames.
  - **Server's view:** closes the lens back into the click point.
- Below the panel:
  - mono tags for "What the server sees" and "What it never sees";
  - the folder-provider paragraph, linked to §12.1;
  - a polite live region: "The server stores this. It cannot turn it into that."

**Timing:**
- The owner's `GLASS` values from `reveal/timing.ts`, unchanged: duration 1.9 s, push 3.8, bevel 0.57 (max 110 px), refraction 6.8, dispersion 0.235, film 0.15, wobble 4.
- `unlockR` opens the lens and `lockR` closes it (0.75 × 1.9 s).
- A new choice reverses from the current radius.

**Technique** (ADR-0032):
- The decrypted layer is real DOM, revealed through a `clip-path` polygon from `lens.ts`.
- `reveal/shader.ts`, copied verbatim and covered by the parity test, draws only the rim and bevel.
- The band samples a Canvas-2D texture painted from the known demo data.

**Fallbacks:** the clip without the rim (no WebGL); a 250 ms crossfade (reduced motion or transparency); both views side by side (no JS).

### M4. Two secrets (pinned 200 vh)

**What you see:**
- Two key tiles start at the left and right edges:
  - "Master password" shows dots and "Only in your head."
  - "Secret Key" shows a live key and "Only on your devices. 128 random bits."
- Behind them lies a dim grid of M2's ciphertext tiles.

| p | Beat |
|---|---|
| 0–0.35 | The tiles slide together (`ease-standard`). |
| 0.35–0.45 | They meet. A mono label reads "Argon2id + HKDF → account key", linked to spec §5. |
| crossing 0.45 forward | **One-shot ripple** with the `RIPPLE` values, time-based. Each grey tile turns to foil over 200 ms as the wavefront passes. It does not replay backwards. |
| 0.60–1 | The foil tiles reassemble the card: M2's dither in reverse, scrubbed. Caption: "Readable again. Only here." |

**The live Secret Key** uses the real spec §5.2 format:
- `M1-` plus 7 groups of 4 Crockford Base32 characters. They encode 16 random bytes plus the first byte of `SHA-256("mosaic/v1/secret-key-checksum" || SK)`.
- Built with `crypto.getRandomValues` and `crypto.subtle.digest`; no dependency.
- Characters resolve left to right, 24 ms apart, each cycling through 6 random glyphs at 16 ms. Tabular mono widths mean no layout shift.
- A "Make another" button; no copy button.
- Caption: "A sample, made in your browser. Never use a key from a website: the app makes yours on your device and never sends it."

**Fallback:** a static tile equation, [Master password] + [Secret Key] → [account key] → card. The key still generates.

**There is no password field anywhere on the page.**

## 4. Motion system

| Token | Value | Use |
|---|---|---|
| `--motion-micro` / `--motion-panel` | 120 / 200 ms | app tokens: hover, press, menus, details |
| `--motion-reveal` | 600 ms | line reveals, section entrances |
| `--motion-sequence` | ≤ 900 ms | hero sweep |
| `--motion-ripple` / `--motion-glass` | 1400 / 1900 ms | owner values |
| `--ease-standard` | cubic-bezier(0.2, 0, 0, 1) | app token, default |
| `--ease-enter` / `--ease-exit` | cubic-bezier(0.16, 1, 0.3, 1) / (0.7, 0, 0.84, 0) | arriving / leaving |

**No springs and no overshoot.** The owner rejected after-bounce.

**Scroll:**
- Native scroll only: no smooth-scroll library, no scroll-jacking.
- Pinned chapters use `position: sticky` with `svh` units.
- Heading line reveals (mask, 600 ms, `ease-enter`, 60 ms per line) and the header's solid state use CSS scroll-driven animations inside `@supports`. They are Baseline since Firefox 159; without support, content simply shows.
- Canvases share one rAF scroll reader, active only while they are on screen.

**Micro-interactions:**
- **Holo button:** the app's port, with hover sheen, press ripple and focus waves. **At most one is visible at a time** (ADR-0014). The header CTA is an outline pill and turns holo only while neither the hero CTA nor the download CTA is in view.
- **Secondary buttons:** 1 px `borderStrong` outline, `bgSelected` fill on hover (120 ms), scale 0.98 on press.
- **Links:** 1 px `faint` underline at 0.18em offset, 2 px `text` on hover. Spec links carry a mono `§12` tag.
- **Cursor:** the system cursor, untouched.
- **Focus:** 2 px `focusRing` with a 2 px gap.

**Page load:**
1. HTML and CSS paint the header, the H1 mask reveal and the trust strip. These are the LCP elements.
2. At +120 ms, the subhead and the CTAs rise.
3. After first paint (at the latest on `load`), WebGL starts. The canvas fades in over 200 ms and the sweep runs.
4. "both" settles as the sweep ends.

There is no preloader.

**Reduced motion is a designed version:** the hero is a cached still with a flattering light angle; M2 becomes the storyboard, M4 the equation and M3 a crossfade; chapter 5 shows final weights; demos swap instantly. Content and order stay identical.

## 5. Tech choices and budgets

**Stack: `apps/web` with Vite, React and strict TS. React runs at build time only.**
- **Static pages:** `scripts/prerender.ts` renders `/`, `/de/` and `/404.html` with `react-dom/server`, which is already in the lockfile.
- **Client code:** no React runtime ships. Interactive parts are vanilla-TS islands, loaded with `import()` as they near the viewport.
- **Strings:**
  - formatted at build time from a new `web` module in `packages/i18n/src/catalog/{en,de}/`;
  - islands read their labels from `data-*` attributes;
  - no ICU code reaches the client.
- **Dependencies:**
  - *New:* only `@fontsource-variable/dm-sans`.
  - *Rejected:*
    - GSAP/ScrollTrigger: about 45 KB for what a 2 KB progress mapper does.
    - Lenis: scroll hijacking hurts accessibility, and sticky stages don't need it.
    - Three.js: the shaders already exist in raw WebGL2.
- **Shared code:** copy `holo/{shader,effects,motion,gl}` and `reveal/{lens,timing,shader}` into `apps/web/src/fx/`. Each copy gets a provenance header and a parity test (§10).

**Budgets:**

| Metric | Target |
|---|---|
| LCP (mid hardware, fast 4G) | < 2.0 s, on the H1 text |
| CLS | ≤ 0.01 (reserved canvas boxes, metric-matched fonts) |
| INP | < 150 ms |
| JS | ≤ 30 KB gz initially, ≤ 90 KB gz with every island |
| CSS / fonts | ≤ 25 KB gz / ≤ 110 KB |
| Frame rate | **60 fps on Intel Iris Xe at 1080p, DPR 1.25** (explicit target). UHD 620: at least 30 fps, otherwise a still. |

**GPU and battery:**
- DPR is capped at 2, and the hero at 2.1e6 pixels (the app's budgets).
- Resolution steps down 1.0 → 0.75 → 0.5 → still after 3 consecutive frames over 50 ms.
- Rendering is on demand:
  - only while the canvas and the tab are visible;
  - 60 fps during interaction, 30 fps for the idle sheen;
  - it stops after 20 s with no pointer.
- Save-Data or reduced motion: still only.

**Accessibility (WCAG 2.2 AA):**
- Canvases are `aria-hidden`; adjacent text carries their meaning.
- Everything works without JS and from the keyboard (radiogroup, `<details>`, `<dialog>` menu).
- Skip link, one H1, targets ≥ 24 px.
- AA contrast in both themes, including the holo label on its brightest frame.
- Demos play once when in view, with a replay button. Nothing loops (2.2.2).

**Privacy:**
- Zero third-party requests. No analytics, no cookies, no storage, so no cookie banner.
- Headers:
  - `Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'none'; manifest-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'`
  - `Referrer-Policy: no-referrer`
  - a deny-all `Permissions-Policy`
  - COOP `same-origin`
  - `nosniff`
  - HSTS
- No inline scripts or styles.
- A CI check fails the build on any external origin.

**i18n:**
- Static pages at `/` (EN) and `/de/`, linked with `hreflang` and a link in the header. No auto-redirect.
- Layouts are proven with the DE strings.
- Numbers and dates are formatted with `Intl` at build time.

## 6. Sections and responsive behaviour

Breakpoints: 360, 600, 900, 1200, 1600 and 2560 px. Above 1600 px:
- type stops growing;
- stages widen to 1440 px;
- the mosaic adds tiles instead of scaling.

```
HEADER (64px, sticky; transparent over the hero, solid + hairline after 8px)
[▪▪▪◈ Mosaic]  How it works  Security  Pricing  Download   EN|DE  (Download)
<900: logo · EN|DE · Menu → full-screen <dialog>, links in h2 size

1 HERO (100svh, min 640px)
≥900                                           <600
ZERO-KNOWLEDGE PASSWORD MANAGER  ▢▢▢▢▢▢▢▢|     LABEL
Two secrets.                     ▢◈▢▢▢▢▢▢|     Two secrets.
Only you hold both.              ▢▢▢▢◈▢▢▢|     Only you hold both.
Subhead (max 52ch)               ▢▢◈▢▢▢▢▢|     Subhead
(Download ◈)  Read how it works  ▢▢▢▢▢▢▢▢|     (Download ◈)
Pre-audit · Spec · Threat model  cols 7–12     Read how it works
                                 bleed →       ▢▢▢▢▢▢▢▢ strip
600–899: mosaic as a 10×3 band above the text   Trust strip

2 LOCKED BEFORE IT LEAVES (pinned 320vh)
H2 + body (cols 1–5)
THIS DEVICE                · · · wire · · ·              SERVER
┌ Northwind Mail ┐                                   ┌────────────────┐
│ url user pass  │   ▦▦ ▦▦ ▦▦  →  →  →                │ ▦▦▦ 01J9… 1.2KB│
│ TOTP           │                                   │ ▦▦▦ …          │
└────────────────┘                                   └────────────────┘
<600: card on top, vertical wire, server below; tiles fly downward

3 WHAT A STOLEN SERVER HOLDS
H2 + body (cols 1–6)                     [ Server's view | Your view ]
┌─────────────── server panel 600px (lens target) ───────────────┐
│ ▦▦▦▦▦▦  01J9…   1.2 KB   14:02:11   device 3f…   (×5)           │
└────────────────────────────────────────────────────────────────┘
SEES  [email] [device IDs] [sizes]…     NEVER SEES  [titles] [URLs]…
Folder-provider paragraph · §12.1
<600: full-width panel, metadata wraps to 2 lines, tag columns stack

4 TWO SECRETS (pinned 200vh)
[Master password ••••] → Argon2id + HKDF ← [Secret Key M1-7KQ4-X29H-…]
            ripple re-colours grey grid → card reassembles
            (Make another)   "A sample, made in your browser…"
<600: key tiles enter from top and bottom; key wraps after group 4

5 NO BACKDOOR (one viewport, static)
        If you lose both, nobody can recover
        your data, including us.          (display, 10 cols)
        Copy paragraph, 40ch

6 WHAT WORKS TODAY (bento, 12 cols)
┌ Desktop app: live mini vault 6×2 ┐┌ Extension 3×2 ┐┌ Passkeys 3×1 ┐
│ (⌘K opens a mini palette)        ││ click → fill  │├ TOTP ring 3×1 ┤
└──────────────────────────────────┘└───────────────┘└───────────────┘
┌ SSH approve ┐┌ Import formats ┐┌ Generator (live) ┐┌ Security report ┐
COMING ┆Folder sync M6┆ ┆Hosted M6┆ ┆Teams M6b┆ ┆Rotation M7┆ ┆CLI M8┆ ┆Mobile M10┆
Status chips: "Development build" / "Not in a store yet".
600–899: 2 cols; <600: 1 col, "coming" items as wrapped dashed chips

7 PRE-AUDIT
Pre-audit statement (h3, warning-token left rule) — never hidden
[Crypto spec §1…§12]  [Threat model R1…R68]  [Decisions ADR log]  paper cards
No-telemetry line

8 FREE AND PAID · TEAMS (coming)
[Free]  [Hosted sync {PRICE}]          dashed band: Teams · "Tell me when…"

9 DOWNLOAD AND VERIFY
Platform cards (local OS detection reorders them, no request) · (Download ◈)
Verify: 1 · 2 · 3 with mono code blocks  — (coming with M9)

10 FAQ (sticky title cols 1–4, <details> cols 5–12) · FOOTER
Links · EN|DE · "Pre-audit software" · "No cookies, so no banner" · big wordmark in faint

404: "This page is ciphertext." The path scrambles to Base32; a small mosaic
with one missing tile; link: "Back to the readable part".
```

The mini-UIs in section 6 are live HTML built from tokens and the app's CSS patterns. Their data is labelled "sample".

## 7. Quality bar: what "Awwwards-level" means here

- [ ] **Mosaic:** the light follows the cursor at 60 fps on Iris Xe. The ripple matches the app's, and the foil is still vivid after 10 minutes.
- [ ] **Pinned chapters:** they scrub backwards cleanly under fast flings, mid-phase stops and off-centre clicks.
- [ ] **Fallbacks as designs:** the reduced-motion, no-JS and no-WebGL versions are each reviewed in their own right.
- [ ] **Hand-set type:** optical sizes, no widows, tabular mono figures. DE checked at 360, 768 and 1440 px.
- [ ] **Small details:** custom `::selection`, designed focus rings, animated underline thickness, `scrollbar-color` in both themes.
- [ ] **Loading:** no FOUC, no font jump; the canvas fades in.
- [ ] **No sound.**
- [ ] **Favicon:** the 4-tile mark as SVG, plus 32 px and 180 px PNGs whose fourth tile is a baked foil still.
- [ ] **OG images:** 1200×630 in EN and DE, captured with the repo's Playwright from the live page at `?still=og`, then committed.
- [ ] **Pages and meta:** designed 404, `hreflang`, canonical, JSON-LD without ratings, print stylesheet.
- [ ] **Scores:** Lighthouse 100 (accessibility, best practices, SEO), ≥ 95 performance; axe clean in both themes and languages; zero external requests.
- [ ] **Claims:** every claim passes positioning §7, and unreleased items keep **(coming)**.

## 8. Non-goals and anti-goals

**Non-goals:** a web vault or any login (the site holds only downloads and docs), a blog, a docs portal, a CMS, A/B tests.

**Anti-goals.** Each of these makes a security product look cheap or untrustworthy:
- **Security clichés:** padlocks, shields, hooded hackers, Matrix rain, green "hacking" terminals, glitch effects (they read as *broken*), alarm statistics.
- **Fake proof:** user counters, testimonials, "trusted by" logos, ratings, third-party star badges, and the words "audited", "bank-grade" or "unhackable".
- **Hijacking:** custom cursors, smooth-scroll libraries, horizontal scroll-jacking, percentage preloaders, autoplay video or sound, chat widgets.
- **Asking for secrets:** any field that invites a real password, including "test your password strength".
- **The owner's no-list:** rotating foil, colour dust, haze, soft gradient blobs, grey or silvery foil, spring overshoot.
- **Glass as a style:** the lens is one moment that means "decrypt", not a theme.
- **Persuasion tricks:** dark patterns, competitor bashing, fear marketing.

**Rule of thumb:** an effect that does not explain the product within 3 seconds goes.

## 9. Build plan

**Working rules:**
- One agent at a time.
- `.agent-scope` covers `apps/web`, `packages/i18n` and the lockfile.
- Checks run as `pnpm --filter @mosaic/web`.
- Every checkpoint is **the live page opened in the owner's browser**, never a screenshot.

**Steps:**
1. **Scaffold:** Vite, prerender, token CSS, fonts with metric fallbacks, the `web` i18n catalog, CSP and headers, the external-origin check, the GLSL parity tests.
2. **Header, hero (M1) and the holo button port**, with `?tune`.
   → **Checkpoint A:** the owner tunes the mosaic live and copies the values. Nothing continues until the hero is approved.
3. **M2** and its storyboard fallback.
   → **Checkpoint B:** scrubbing, timing and the ciphertext look.
4. **M3, M4 and chapter 5.**
   → **Checkpoint C:** the full story, in both themes.
5. **Content sections** (bento, pre-audit, pricing, teams, download, FAQ, footer), then the DE pass.
   → **Checkpoint D:** EN and DE at 360, 768, 1440 and 2560 px.
6. **Hardening:**
   - the reduced-motion, no-JS and no-WebGL reviews;
   - the Iris Xe check on the owner's laptop;
   - Lighthouse and axe;
   - 404, OG images, favicons, print.

   → **Checkpoint E:** one `ux-reviewer` pass, then the owner's sign-off and the commit.

## 10. Decisions for the owner

1. **Hero headline.** Recommended: "Two secrets. Only you hold both." Switch to "Your vault. Your cloud. Your keys." after M6 ships.
2. **Demo ciphertext.** Recommended: labelled random bytes.
   - The alternative is real XChaCha20-Poly1305 via a WASM build of `crates/core`.
   - It is truer to "no fakery", but costs a WASM pipeline, about 40 KB and `'wasm-unsafe-eval'` in the CSP.
   - Visitors still cannot verify it.
3. **Theme.** Recommended: dark primary, following the OS. Alternative: always dark.
4. **Story order.** Confirm the reorder of copy section 2: leave → stored → two secrets → no backdoor.
5. **Teams waitlist without third parties.** Options: `mailto:` (recommended for now), GitHub Discussions, or our own endpoint later.
6. **Host.** It must set response headers.
   - GitHub Pages cannot send `frame-ancestors`.
   - Vercel, Netlify, Cloudflare Pages and our own server can.
   - On Vercel, `.vercelignore` must list `.claude/worktrees/`.
7. **Shared foil code.** Recommended: copies with parity tests now, `packages/holo` later, because extracting it touches the desktop app.
8. **Placeholders.** Name, domain, price and license stay placeholders; the layout does not depend on them.
