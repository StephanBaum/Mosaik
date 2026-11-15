# Website Launch Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rebuild the multi-page marketing site `apps/web` with the visual system and interactions of the approved prototype, as a static, prerendered, CSP-strict site that paints from HTML and CSS alone and loads its WebGL, data layer, holo and demo code lazily, only on capable devices, inside hard size and runtime budgets.

**Architecture:** React still renders every route and language at build time (`scripts/prerender.ts`); the browser gets HTML, one stylesheet, a small entry module and on-demand chunks. The prototype's GSAP code becomes a small in-house timeline (`src/anim`). The tower-city field, the transition veil, the data layer and the panel glyph rings become one lazy chunk (`src/field/bootField.ts`) that boots after `load` plus idle, behind a capability tier. Cross-page transitions use cross-document View Transitions in CSS; the veil wipe is an extra on capable devices. Budgets, a claims gate and a shader compile test run in the build and in CI.

**Tech Stack:** Vite 8, React 19 (build time only), TypeScript 6 strict, vitest 5 (node + jsdom per file), Playwright with Chromium (SwiftShader WebGL2), axe-core, `subset-font` (new, build time only), `packages/i18n`, `packages/design-tokens`.

**Spec (design source):** the approved prototype `mosaic-website.html` (owner: "this version is cool… lets build a turbo optimized super fast marketing page"), kept as a line-stable reference copy at `C:/Users/Stephan/AppData/Local/Temp/mosaic-web-proto/mosaic-website.ref.html` (Task 1, Step 1). Page structure, routes and funnel: `docs/superpowers/specs/2026-11-08-website-v2.md` on `feat/web` (its 2026-11-08 owner update is binding: no generator, no secret inputs, Linear calm). Security challenge: `docs/DECISIONS.md` ADR-0087, `docs/BUG_BOUNTY.md`, `docs/CHALLENGE.md`. Feature claims: the plans in `docs/superpowers/plans/2026-11-14-*` and `2026-11-15-*`.

**Prototype line map** (reference copy, line numbers unchanged from the original):

| Part | Lines |
|---|---|
| CSS tokens / reduced motion | 8–115 / 116–122 |
| base, type | 123–234, 235–328 |
| field, veil, panel-fx, still, transit hint | 329–400 |
| header, nav underline, menu | 401–612 |
| layout, buttons, micro glow | 613–719, 720–809, 810–939 |
| hero, frames/crops, grids, message sections | 940–1054, 1055–1154, 1155–1396, 1397–1481 |
| demo surfaces: mock, fill, sync, agent, import, phone, terminal, tools | 1482–1603, 1604–1841, 1842–1938, 1939–2101, 2102–2267, 2268–2431, 2432–2532, 2533–2566 |
| security how, plans + final CTA, download, next steps, footer, reduced motion | 2567–2756, 2757–2863, 2864–2999, 3000–3116, 3117–3246, 3247–3294 |
| JS Field (shaders, GL) | 3332–3586 |
| JS Holo | 3587–4047 (a port of `apps/web/src/{fx,holo}`: do not re-port) |
| JS Micro | 4048–4114 |
| JS Views markup: helpers, shared blocks, home, features, security, download, next, chrome | 4126–4268, 4269–4300, 4304–4393, 4395–4443, 4445–4479, 4481–4505, 4507–4524, 4526–4558 |
| JS Demos: decode/type, fill, sync, agent, import, phone, cli, cipher, steps, mount | 4570–4603, 4613–4688, 4690–4719, 4721–4741, 4743–4797, 4799–4834, 4836–4855, 4857–4873, 4875–4886, 4889–4928 |
| JS app: field+veil init, lens, data layer, panel rings, still, field loop, pointer | 4953–4980, 4981–4989, 4990–5296, 5297–5436, 5437–5446, 5448–5500, 5501–5537 |
| JS app: mount, transitions (veil frame, wipe, soft, jump), router | 5583–5648, 5664–5776, 5777–5891 |
| Prototype copy (EN/DE) | `C:/Users/Stephan/AppData/Local/Temp/mosaic-web-proto/i18n.json`; only the `x.*` keys are new, every `web.*` value equals the repo catalog |

## Global Constraints

**Workspace and process**
- All work happens in the worktree `C:/Users/Stephan/Documents/DEV/Mosaic/.claude/worktrees/web` on branch `feat/web`. The orchestrator writes `.agent-scope` there: `apps/web/`, `packages/design-tokens/`, `packages/i18n/src/catalog/`, `.github/workflows/ci.yml`, `pnpm-lock.yaml`, `vitest.config.ts`, `docs/` (Task 18 only).
- Agent guard (`~/.claude/hooks/agent-guard.mjs`): no searches outside the repo, `node_modules` or the cargo registry; no writes outside the project, the system temp dir or `~/.claude/projects`; at most 3 full-workspace runs per agent per hour (prefix a deliberate final gate with `FULL_RUN_OK=1`); never a raw `git worktree remove`; worktrees only via `~/.claude/hooks/worktree.sh`.
- One agent at a time (7.4 GB RAM, 6 CPUs). No background builds. After a stopped task, end leftover `node` processes.
- Per-task checks are scoped: `pnpm --filter @mosaic/web lint`, `pnpm --filter @mosaic/web typecheck`, one test file via `pnpm -w exec vitest run --project web <path>`, `pnpm --filter @mosaic/web build` only where a task says so. Token and catalog tests: `pnpm -w exec vitest run --project packages packages/design-tokens` / `packages/i18n`.
- Each task ends with a local commit on `feat/web` (Conventional Commits, attribution trailer). The full CLAUDE.md gate runs once, in Task 18, before the push; the squash merge into `main` is the milestone commit. Never push before Task 18.
- pnpm only. Every new dependency gets a one-line justification in its commit message.

**Product rules**
- All UI strings through `packages/i18n` (`packages/i18n/src/catalog/{en,de}/web/*.ts`), German informal "du", EN and DE with identical keys. Placeholders that render: `{name}`, `{domain}`, `{PRICE}`, `{TEAM_PRICE}` only (plus message arguments filled by the page).
- All colours through `packages/design-tokens` (Task 2). No hex, `rgb()` or `rgba()` literal in `apps/web/src/styles/*.css` or in canvas code; canvas colours come from `fieldColors`.
- No "audited", "geprüft von", "bank-grade", "unhackable", "unknackbar", "pre-audit" in web copy (existing catalog test). README stays labeled pre-audit.
- The site never generates, accepts or checks a password, however locally: no `type="password"`, no generator route, no random secret-looking strings; demos use fixed literal values only.
- No telemetry, no analytics, no third-party request of any kind, no CDN. GSAP does not ship; no runtime framework ships.
- Every claim about a feature that ships after the site is registered in `src/pages/claims.ts` (Task 16) and blocks a production build until verified.

**Performance budgets** (gzip level 9 with `node:zlib`, measured on `dist/` by `scripts/budget.ts`, enforced by the build, Task 3)

| Budget | Limit |
|---|---|
| HTML per page | ≤ 40 000 B |
| The one stylesheet | ≤ 30 000 B |
| Entry JS (`src/main.ts` chunk plus its static imports) | ≤ 8 000 B |
| Lazy chunk `src/field/bootField.ts` (+ imports not in the entry) | ≤ 16 000 B |
| Lazy chunk `src/holo/index.ts` | ≤ 12 000 B |
| Lazy chunk `src/demos/bootDemos.ts` | ≤ 12 000 B |
| Lazy chunk `src/micro/index.ts` | ≤ 3 000 B |
| Any other lazy chunk (islands) | ≤ 4 000 B |
| All JS a page can load (entry + field + micro + its islands) | ≤ 50 000 B |
| Preloaded font (DM Sans subset, the only preload) | ≤ 40 000 B raw woff2 |
| Any other font file | ≤ 16 000 B raw woff2 |
| Eager image (`loading="eager"`), any format | ≤ 80 000 B raw; at most one `fetchpriority="high"` per page |
| Initial transfer per page (HTML + CSS + entry JS + preloaded font + eager image) | ≤ 200 000 B |

**Runtime budgets** (Playwright Chromium, `vite preview`, cold cache; "mobile" = 390×844, DPR 3, CPU throttle 4×, 1.6 Mbit/s down, 750 kbit/s up, 150 ms RTT; median of 3 runs; Task 17)
- LCP ≤ 2 500 ms mobile, ≤ 1 200 ms desktop (1440×900, no throttle).
- CLS ≤ 0.02 measured to 6 s after load, on every route in EN.
- TBT (sum of long-task time above 50 ms from FCP to 6 s) ≤ 150 ms mobile.
- No lazy chunk request starts before the final LCP entry's `startTime`.
- Field: DPR cap 1.5 (full) / 1 (lite), at most 2 400 000 canvas pixels, 30 fps when lite or scrolled past 1.9 viewport heights, paused when the tab is hidden. Governor: after 30 warm-up frames, the mean frame interval over the next 90 frames > 25 ms → lite; in lite > 45 ms → stop and show the still.

**Capability tiers** (`fxTier`, Task 6; evaluated in order)
- `none` (CSS still only; no field, no veil, no data layer, no rings, no micro, no decode; holo keeps its token gradient): `prefers-reduced-motion: reduce`, motion toggle off, `navigator.connection.saveData`, `effectiveType` `slow-2g` or `2g`, no WebGL2 context with `failIfMajorPerformanceCaveat: true`, `hardwareConcurrency <= 2`, `deviceMemory < 2`.
- `lite` (field at DPR 1 and 30 fps, panel rings; no data layer, no veil wipe): `(pointer: coarse)`, `hardwareConcurrency <= 4`, `deviceMemory <= 4`, `effectiveType` `3g`, viewport width < 700.
- `full`: everything else.
- Holo buttons load on every tier except Save-Data; their own reduced-motion handling stays (ADR-0014).

**Security headers and CSP**
- Every page (meta tag) and `_headers`: `default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self'; font-src 'self'; connect-src 'none'; manifest-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'; require-trusted-types-for 'script'; trusted-types 'none'; upgrade-insecure-requests`. `form-action 'self'` on capture routes once capture is on; `frame-ancestors` only in the header (a meta tag cannot carry it). No `'unsafe-inline'`, no hashes needed: no inline script runs (JSON-LD data blocks only).
- Runtime code never assigns `innerHTML`/`outerHTML`/`insertAdjacentHTML`, never `setAttribute("style", …)`; it builds DOM with `createElement`/`textContent` and writes styles with `el.style.setProperty`.
- Existing headers stay (Referrer-Policy, Permissions-Policy, COOP, nosniff, HSTS). New: `Cache-Control` per path class and `Speculation-Rules` (Task 3).

**Accessibility (WCAG 2.2 AA)**
- Visible `:focus-visible` ring in `--paper`, 2 px, never tinted; targets ≥ 24×24 px; skip link stays; menu stays a `<dialog>`.
- Auto-playing motion: demos loop at most `MAX_LOOPS = 3` times and rest on their final frame; a footer "Pause animations" toggle stops the field, data layer, demos, holo ambient motion and CSS animations (WCAG 2.2.2). `prefers-reduced-motion` gives the same static result without the toggle.
- Every demo root is `role="img"` with an `aria-label`; its inside is `aria-hidden`. Without JS, and with reduced motion, every demo shows its finished state.

**Decisions taken in this plan** (recorded as one ADR in Task 18)
1. No Lighthouse CI: `@lhci/cli` pulls a large dependency tree for numbers Playwright already measures; budgets are enforced by `scripts/budget.ts` (bytes) and Playwright with CDP throttling (LCP, CLS, TBT). Lighthouse may be run by hand once per milestone.
2. Cross-page transitions: CSS cross-document View Transitions (old page fades 200 ms, new page wipes in diagonally 460 ms, top-left to bottom-right like the veil). The veil wipe-in (320 ms) plays before navigation only for pointer clicks on `data-nav` links on tier `full`; its covered screen becomes the old snapshot. Browsers without the API navigate plainly.
3. Same-origin prefetch through the `Speculation-Rules` header (`prefetch`, `moderate`), never `prerender` (prerender would boot WebGL for a page nobody sees).
4. DM Sans is subset at build time to the characters of the web catalogs (keeps the `wght` and `opsz` axes of the approved look). IBM Plex Mono is not subset: its OFL carries the Reserved Font Name "Plex", so a modified file could not keep the name; the unmodified fontsource latin files are used and never preloaded.
5. Trusted Types enforced (`trusted-types 'none'`): no runtime code path needs an HTML sink.
6. Two new routes for the launch extras: `/report/` (`/de/melden/`) and `/security/challenge/` (`/de/sicherheit/challenge/`), linking to GitHub; the site itself still takes no input.
7. The prototype's "Design exploration, not built yet" badges are dropped; the described features are gated by the claims registry instead.

## Review Focus

1. **JS fails, is slow or is off:** every section must still be visible. Only JS adds `reveal-pending`, and only to elements below the fold at boot. Test: Task 6 (no rendered page contains `reveal-pending`) and Task 17 (JS disabled, every `[data-reveal]` has computed opacity 1).
2. **Back/forward after a veil wipe:** a page restored from the bfcache must not stay covered. Test: Task 9 (`pageshow` with `persisted` calls `veil.reset()`), Task 17 (`goBack()` after a wipe, veil hidden).
3. **Ctrl/Cmd/Shift/middle click, keyboard Enter, `target="_blank"`, `download` on nav links:** these open normally and never wipe. Test: Task 9 `shouldWipe` table.
4. **German copy at 320 px:** long German words must not cause horizontal scrolling. Test: Task 17, every route in DE and EN at 320×640, `scrollWidth <= clientWidth`.
5. **WebGL context loss mid-session** (GPU reset, mobile backgrounding): the field must drop to the still, with no black screen and no thrown errors. Test: Task 7 (fake context, `webglcontextlost` → still shown, loop stopped), Task 17 (`WEBGL_lose_context` on a forced field).

---

## File Structure

```
apps/web/
  scripts/
    prerender.ts          modify: font regex, external link allow list, claims gate, speculation file, budget run
    budget.ts             create: measureBuild, budgetViolations
    fonts.ts              create: webCharset, subsetDmSans, fontSubset() plugin
  src/
    main.ts               modify: entry (menu, motion toggle, reveals, nav, jumps, islands, idle field/micro)
    budgets.ts            create: BUDGETS
    islandSources.ts      create: ISLAND_SOURCES (island id -> source path, for budgets)
    anim/                 create: ease.ts, ticker.ts, timeline.ts, tween.ts (+ tests)
    runtime/              create: capability.ts, motion.ts, idle.ts, reveal.ts, fxRegistry.ts (+ tests)
    field/                create: shaders.ts, gl.ts, atlas.ts, lens.ts, size.ts, governor.ts, veil.ts,
                                  bootField.ts, towers.ts, dataLayer.ts, panelRings.ts, glsl.test.ts (+ tests)
    nav/                  create: intercept.ts, wireNav.ts, jump.ts (+ tests)
    micro/index.ts        create: bootMicro
    demos/                create: text.ts, fill.ts, sync.ts, agent.ts, importer.ts, phone.ts, cli.ts,
                                  cipher.ts, nextSteps.ts, bootDemos.ts, decode.ts (+ tests)
    islands/foil.ts, islands/steps.ts   delete
    pages/
      routes.ts, security.ts, site.ts, render.tsx, Header.tsx, Footer.tsx, ui.tsx   modify
      claims.ts           create: CLAIMS, unverifiedClaims
      demos/              create: FillDemo, SyncDemo, AgentDemo, ImportDemo, PhoneDemo, CliDemo (TSX markup)
      views/*.tsx         modify; views/help.tsx create (Report, Challenge)
    styles/               rewrite from the prototype (file list in Task 11)
  e2e/                    create: shaders, pages, perf, fallbacks, transitions specs
  playwright.config.ts    create
packages/design-tokens/src/web.ts (+ web.test.ts)   create
packages/i18n/src/catalog/{en,de}/web/{launch.ts,help.ts}   create; landing.ts, site.ts, index.ts modify
.github/workflows/ci.yml   modify: job "Website"
```

---

### Task 1: Base the branch on `main`, keep the prototype, baseline green

**Files:**
- Modify: whatever the merge touches (expected: `vitest.config.ts`, `.github/workflows/ci.yml`, `pnpm-lock.yaml`, `docs/*`)
- Create (outside the repo): `C:/Users/Stephan/AppData/Local/Temp/mosaic-web-proto/{mosaic-website.ref.html,i18n.json}`

**Interfaces:**
- Produces: `feat/web` contains `main` (9843e6c or later); `apps/web` builds and its tests pass; the reference copy exists for every later task.

- [ ] **Step 1 (orchestrator, before any agent): save the prototype with stable line numbers**

The session scratchpad is deleted with the session. Run once:

```js
// node C:/Users/Stephan/AppData/Local/Temp/mosaic-web-proto-strip.mjs
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
const src = "C:/Users/Stephan/AppData/Local/Temp/claude/38bbc6ddbaaf723a/55dc3723-0b89-4c58-8bbd-70c6091f6d48/scratchpad/landing-artifact/mosaic-website.html";
const out = "C:/Users/Stephan/AppData/Local/Temp/mosaic-web-proto";
mkdirSync(out, { recursive: true });
const lines = readFileSync(src, "utf8").split("\n");
writeFileSync(`${out}/i18n.json`, lines[3326].replace(/^window\.MOSAIC_I18N=/, "").replace(/;\s*$/, ""));
for (const i of [3, 4, 5, 6]) lines[i] = "/* font blob removed (self-hosted fonts: Task 4) */";
lines[3326] = "window.MOSAIC_I18N = {}; /* see i18n.json */";
lines[3328] = "window.MOSAIC_IMG = {}; /* screenshots: apps/web/public/img */";
writeFileSync(`${out}/mosaic-website.ref.html`, lines.join("\n"));
```

Expected: `mosaic-website.ref.html` has 5 891 lines and is under 300 KB; `i18n.json` parses and has 267 keys per language.

- [ ] **Step 2: merge `main` into `feat/web`** (in the web worktree): `git fetch` is not needed (shared repo); `git merge main`. Resolve conflicts keeping both sides: `vitest.config.ts` projects list gains `"apps/web/vitest.config.ts"`; `ci.yml` keeps `main`'s jobs; regenerate the lockfile with `pnpm install` (never hand-edit it).
- [ ] **Step 3: baseline**

Run: `pnpm install --frozen-lockfile && pnpm --filter @mosaic/web lint && pnpm --filter @mosaic/web typecheck && pnpm --filter @mosaic/web test && pnpm --filter @mosaic/web build`
Expected: all pass; `apps/web/dist/index.html` exists.

- [ ] **Step 4: Commit** `chore(web): merge main into feat/web`

---

### Task 2: The web palette as design tokens

**Files:**
- Create: `packages/design-tokens/src/web.ts`, `packages/design-tokens/src/web.test.ts`
- Modify: `packages/design-tokens/src/index.ts` (re-export), `apps/web/vite.config.ts` (`tokensStylesheet` serves `cssVariables() + webCssVariables()`), `apps/web/src/styles/base.css` (drop its `:root` palette)
- Test: `apps/web/src/styles/tokens.test.ts`

**Interfaces:**
- Produces:
  - `webColors` (an `as const` object, so every key stays typed): every hex token of prototype lines 12–55 and 66–71 (`ink #0a0a0f`, `inkRaised #111118`, `inkDeep #07070b`, `paper #f5f2ea`, `paper2 #c9c6d6`, `paper3 #9693a8`, `white #ffffff`, `cCobalt #2b4bff`, `cUltramarine #1a1f7a`, `cYellow #ffc21a`, `cTangerine #ff7a1a`, `cCoral #ff5a4e`, `cTeal #18a3a3`, `cMint #4de3b0`, `cMintSoft #7bebc4`, `cViolet #8a5cff`, `cSky #5ec8ff`, `cPink #ff7ac8`, `cAubergine #2a1240`, `tint #8a7dff`, `tintText #b9b0ff`, `selection #6b4dff`, `maskSolid #000000`).
  - `webAlphas: Readonly<Record<string, string>>`: every `rgb(… / a)` token of lines 22–64 by its CSS name without `--` (`ink-glass`, `ink-veil`, `rule`, `rule-strong`, `rule-bright`, `grid-line`, `tint-14` … `scrim`), values verbatim.
  - `fieldColors: { link: [150,140,255], node: [190,182,255], label: [205,198,255], ring: [170,160,255], cover: [10,10,15] }` (the RGB triples the data layer and rings use, prototype 5200–5290, 5395).
  - `webCssVariables(): string` → one `:root{…}` block; CSS names: camelCase → kebab with digits split (`paper2` → `--paper-2`, `cCobalt` → `--c-cobalt`, `inkRaised` → `--ink-raised`), alphas as `--<name>`.

- [ ] **Step 1: Write the failing tests**

```ts
// packages/design-tokens/src/web.test.ts
import { contrastRatio } from "./index.ts";
import { webAlphas, webColors, webCssVariables } from "./web.ts";
it("keeps text contrast on the dark ground", () => {
  expect(contrastRatio(webColors.paper, webColors.ink)).toBeGreaterThanOrEqual(15);
  expect(contrastRatio(webColors.paper2, webColors.ink)).toBeGreaterThanOrEqual(7);
  for (const bg of [webColors.ink, webColors.inkRaised]) {
    expect(contrastRatio(webColors.paper3, bg)).toBeGreaterThanOrEqual(4.5);
    expect(contrastRatio(webColors.tintText, bg)).toBeGreaterThanOrEqual(7);
    expect(contrastRatio(webColors.tint, bg)).toBeGreaterThanOrEqual(4.5);
  }
});
it("emits each variable once with prototype names", () => {
  const css = webCssVariables();
  const names = [...css.matchAll(/(--[a-z0-9-]+):/g)].map((m) => m[1]);
  expect(new Set(names).size).toBe(names.length);
  expect(css).toContain("--paper-2:#c9c6d6");
  expect(css).toContain("--c-cobalt:#2b4bff");
  expect(css).toContain(`--rule:${webAlphas["rule"]}`);
});
```

```ts
// apps/web/src/styles/tokens.test.ts
it.each(readdirSync(stylesDir).filter((f) => f.endsWith(".css")))("%s has no colour literals", (file) => {
  const css = readFileSync(join(stylesDir, file), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
  expect(css).not.toMatch(/#[0-9a-f]{3,8}\b|rgba?\(\s*\d/i);   // rgb(var(--tint-rgb) / …) stays allowed
});
```

The one literal outside the prototype's `:root` (line 774, `rgb(138 125 255 / calc(0.26 * var(--bl-glow)))`) becomes `rgb(var(--tint-rgb) / calc(0.26 * var(--bl-glow)))` with a new `webAlphas` entry `"tint-rgb": "138 125 255"` (emitted as `--tint-rgb`).

- [ ] **Step 2: Run** `pnpm -w exec vitest run --project packages packages/design-tokens` → FAIL (module missing).
- [ ] **Step 3: Implement** `web.ts` with the values above; wire `webCssVariables()` into `tokensStylesheet()` (dev and build) and remove the palette from `base.css`.
- [ ] **Step 4: Run** both tests and `pnpm --filter @mosaic/web typecheck` → PASS.
- [ ] **Step 5: Commit** `feat(design-tokens): web palette, alphas and field colours as tokens`

---

### Task 3: Build budgets, cache and prefetch headers, CSP hardening

**Files:**
- Create: `apps/web/src/budgets.ts`, `apps/web/src/islandSources.ts`, `apps/web/scripts/budget.ts`, `apps/web/scripts/budget.test.ts`, `apps/web/src/pages/headers.test.ts`
- Modify: `apps/web/src/pages/security.ts`, `apps/web/scripts/prerender.ts` (emit `speculation-rules.json`, run budgets in `closeBundle`), `apps/web/vite.config.ts` (preview headers and a `configurePreviewServer` middleware that serves `/speculation-rules.json` as `application/speculationrules+json`)

**Interfaces:**
- Produces:
  - `BUDGETS` (exact values of the Global Constraints table): `{ htmlGzip: 40_000, cssGzip: 30_000, entryGzip: 8_000, lazyGzip: { "src/field/bootField.ts": 16_000, "src/holo/index.ts": 12_000, "src/demos/bootDemos.ts": 12_000, "src/micro/index.ts": 3_000 }, lazyDefaultGzip: 4_000, pageJsGzip: 50_000, fontPreload: 40_000, fontOther: 16_000, eagerImage: 80_000, initialGzip: 200_000 } as const`
  - `ISLAND_SOURCES: Readonly<Record<IslandId, string>>` (e.g. `holo: "src/holo/index.ts"`, `demos: "src/demos/bootDemos.ts"`).
  - `interface PageReport { file: string; html: number; initial: number; jsMax: number; eager: readonly { src: string; bytes: number }[]; highPriority: number }`, `interface BuildReport { pages: readonly PageReport[]; css: number; entry: number; lazy: Readonly<Record<string, number>>; fonts: Readonly<Record<string, { bytes: number; preload: boolean }>> }`
  - `measureBuild(outDir: string): BuildReport` (reads `.vite/manifest.json`; a page's `jsMax` = entry + `src/field/bootField.ts` + `src/micro/index.ts` + every island named by a `data-island` in that page).
  - `budgetViolations(report: BuildReport, budgets?: typeof BUDGETS): string[]` (one readable line per breach, e.g. `index.html: html 41200 B > 40000 B`).
  - `speculationRules(): string` in `security.ts` → `{"prefetch":[{"source":"document","where":{"href_matches":"/*"},"eagerness":"moderate"}]}`.
  - `cspFor` gains `require-trusted-types-for 'script'`, `trusted-types 'none'`, `upgrade-insecure-requests`.
  - `headersFile()`: per page path block adds `Cache-Control: public, max-age=0, must-revalidate` and `Speculation-Rules: "/speculation-rules.json"`; new blocks `/assets/*` → `Cache-Control: public, max-age=31536000, immutable`; `/img/*` → `Cache-Control: public, max-age=2592000`; `/speculation-rules.json` → `Content-Type: application/speculationrules+json` and `Cache-Control: public, max-age=3600`. `/*` carries no `Cache-Control` (values would stack).

- [ ] **Step 1: Write the failing tests**

```ts
// scripts/budget.test.ts
const page = (o: Partial<PageReport> = {}): PageReport => ({ file: "index.html", html: 1, initial: 1, jsMax: 1, eager: [], highPriority: 0, ...o });
const ok: BuildReport = { pages: [page()], css: 1, entry: 1, lazy: { "src/field/bootField.ts": 1 }, fonts: { "dm.woff2": { bytes: 1, preload: true } } };
it("passes a report inside every budget", () => expect(budgetViolations(ok)).toEqual([]));
it.each([
  [{ ...ok, css: 30_001 }, /css 30001 B > 30000 B/],
  [{ ...ok, entry: 8_001 }, /entry 8001 B > 8000 B/],
  [{ ...ok, lazy: { "src/field/bootField.ts": 16_001 } }, /bootField\.ts 16001 B > 16000 B/],
  [{ ...ok, lazy: { "src/islands/print.ts": 4_001 } }, /print\.ts 4001 B > 4000 B/],
  [{ ...ok, pages: [page({ html: 40_001 })] }, /index\.html: html 40001 B > 40000 B/],
  [{ ...ok, pages: [page({ highPriority: 2 })] }, /index\.html: 2 fetchpriority=high images/],
  [{ ...ok, fonts: { "dm.woff2": { bytes: 40_001, preload: true } } }, /dm\.woff2 40001 B > 40000 B/],
])("reports %#", (report, line) => expect(budgetViolations(report).join("\n")).toMatch(line));
```

```ts
// src/pages/headers.test.ts
it("gives every page path exactly one Cache-Control and none to /*", () => {
  const blocks = headersFile().split(/\n(?=\/)/);
  expect(blocks.find((b) => b.startsWith("/*\n"))).not.toMatch(/Cache-Control/);
  for (const r of ROUTES) for (const p of [r.en, r.de]) {
    const b = blocks.find((x) => x.startsWith(`${p}\n`)) ?? "";
    expect(b.match(/Cache-Control/g)).toHaveLength(1);
    expect(b).toContain('Speculation-Rules: "/speculation-rules.json"');
  }
  expect(headersFile()).toContain("/assets/*\n  Cache-Control: public, max-age=31536000, immutable");
});
it("enforces Trusted Types and never allows inline code", () => {
  expect(CSP).toContain("require-trusted-types-for 'script'; trusted-types 'none'");
  expect(CSP).not.toMatch(/unsafe-|sha256-|nonce-/);
});
it("prefetches only, moderately", () => {
  const rules = JSON.parse(speculationRules()) as Record<string, unknown>;
  expect(Object.keys(rules)).toEqual(["prefetch"]);
  expect(JSON.stringify(rules)).toContain('"eagerness":"moderate"');
});
```

- [ ] **Step 2: Run** both files → FAIL.
- [ ] **Step 3: Implement** the interfaces. In `closeBundle`, after the existing checks: `const v = budgetViolations(measureBuild(outDir)); if (v.length) throw new Error(\`Budgets exceeded:\n${v.join("\n")}\`)`. Gzip with `gzipSync(buf, { level: 9 }).length`. Add `fetchpriority="high"` to `Shot` when `eager` (React prop `fetchPriority="high"`).
- [ ] **Step 4: Run** the two tests, then `pnpm --filter @mosaic/web build` → PASS (today's site is inside every budget; if not, the failing line names what to fix in a later task: note it in the commit message, do not raise the budget).
- [ ] **Step 5: Commit** `feat(web): build budgets, cache and prefetch headers, Trusted Types`

---

### Task 4: Subset and preload DM Sans

**Files:**
- Create: `apps/web/scripts/fonts.ts`, `apps/web/scripts/fonts.test.ts`
- Modify: `apps/web/package.json` (devDependency `subset-font`, exact pin via `pnpm --filter @mosaic/web add -D -E subset-font`), `apps/web/vite.config.ts` (add `fontSubset()` first in `plugins`), `apps/web/src/styles/fonts.css`, `apps/web/scripts/prerender.ts` (`buildAssets` font regex `/^dm-sans-sub-.*\.woff2$/`)

**Interfaces:**
- Produces:
  - `EXTRA_CHARS = "‘’‚“”„–—…·→←✓✕●›‹×€°§0123456789abcdef"`
  - `webCharset(): string` – sorted unique characters of U+0020–U+007E, `EXTRA_CHARS`, and every `web.*` value of both catalogs with `{name}` and `{domain}` filled (`formatMessage` placeholders removed: strip `{…}` argument names, keep literal text).
  - `subsetDmSans(text: string): Promise<Buffer>` – input `@fontsource-variable/dm-sans/files/dm-sans-latin-opsz-normal.woff2` (it covers Basic Latin, Latin-1 incl. äöüß, and U+2000–206F incl. „“–…; the latin-ext file is dropped), `targetFormat: "woff2"`, variation axes kept. Characters DM Sans has no glyph for (arrows, ✓, ●) fall back to the metric-matched fallback face, as today.
  - `fontSubset(): Plugin` – `buildStart` (dev and build) writes `apps/web/node_modules/.cache/mosaic-fonts/dm-sans-sub.woff2` when the cached charset hash (`sha256` of `webCharset()`, stored beside it as `charset.sha256`) differs.
- `fonts.css`: one DM Sans `@font-face` (`font-weight: 100 1000`, `font-display: swap`, `src: url("../../node_modules/.cache/mosaic-fonts/dm-sans-sub.woff2") format("woff2")`, no `unicode-range`); Plex Mono 400/500 imports unchanged; the metric-matched `DM Sans Fallback` face unchanged.

- [ ] **Step 1: Write the failing tests**

```ts
it("covers every character the site prints", () => {
  const set = new Set(webCharset());
  for (const c of "äöüÄÖÜß„“”–…€→") expect(set.has(c), c).toBe(true);
  for (const L of locales) for (const [k, v] of Object.entries(catalogs[L])) if (k.startsWith("web."))
    for (const c of v.replace(/\{[a-zA-Z]+\}/g, "")) expect(set.has(c), `${k}: ${c}`).toBe(true);
});
it("subsets DM Sans under the preload budget", async () => {
  const buf = await subsetDmSans(webCharset());
  expect(buf.length).toBeLessThanOrEqual(40_000);
  expect(buf.subarray(0, 4).toString("latin1")).toBe("wOF2");
}, 30_000);
```

- [ ] **Step 2: Run** `pnpm -w exec vitest run --project web apps/web/scripts/fonts.test.ts` → FAIL.
- [ ] **Step 3: Implement**; add `apps/web/node_modules/.cache/` needs no ignore entry (`node_modules/` is ignored).
- [ ] **Step 4: Run** the test, then `pnpm --filter @mosaic/web build`; Expected: `dist/assets/dm-sans-sub-*.woff2` ≤ 40 000 B and preloaded in every page head; no `dm-sans-latin-*` file in `dist/assets`.
- [ ] **Step 5: Commit** `perf(web): subset DM Sans at build time and preload it` with body line `subset-font: build-time woff2 subsetting (harfbuzz wasm), keeps the opsz/wght axes; ~40 % less on the only preloaded font.`

---

### Task 5: In-house animation core (replaces GSAP)

**Files:**
- Create: `apps/web/src/anim/{ease.ts,ticker.ts,timeline.ts,tween.ts}`, tests `apps/web/src/anim/{ease,timeline,tween}.test.ts`

**Interfaces:**
- Produces:
  - `ease.ts`: `type Ease = (t: number) => number`; `linear`, `power1Out`, `power1InOut`, `power2In`, `power2Out`, `power2InOut`, `power3In`, `power3Out`, `power3InOut` (GSAP naming: powerN in = t^(N+1), out = 1 − (1−t)^(N+1), inOut symmetric); `backOut(overshoot = 1.70158): Ease` = `1 + (s+1)(t−1)³ + s(t−1)²`; `elasticOut(amplitude = 1, period = 0.3): Ease` = t for t ∈ {0, 1}, else `a·2^(−10t)·sin((t − p/(2π)·asin(1/a))·2π/p) + 1` with `a = max(amplitude, 1)`.
  - `ticker.ts`: `interface Ticker { add(fn: (now: number, dt: number) => void): () => void }`; `createTicker(raf: (cb: FrameRequestCallback) => number, caf: (id: number) => void): Ticker` (runs only while it has subscribers; `dt` in seconds, clamped to 0.05); `ticker` – lazily bound to `window`.
  - `timeline.ts`:
    ```ts
    export type Target = Element | Record<string, number>;
    export type Position = number | "<" | `+=${number}` | `-=${number}`;
    export interface Motion { duration?: number; ease?: Ease; stagger?: number; repeat?: number; yoyo?: boolean; onUpdate?: () => void }
    export interface TimelineOptions { repeat?: number; repeatDelay?: number; onComplete?: () => void; ticker?: Ticker }
    export class Timeline {
      to(targets: Target | readonly Target[], props: Readonly<Record<string, number>>, motion?: Motion, at?: Position): this;
      set(targets: Target | readonly Target[], props: Readonly<Record<string, number | string>>, at?: Position): this;
      call(fn: () => void, at?: Position): this;
      get duration(): number;   // one iteration, seconds
      get loops(): number;      // completed iterations
      seek(time: number): void; // whole run incl. repeats; fires `call`s crossed forward only
      progress(p: number): void;// seek(p * duration)
      play(): void; pause(): void; kill(): void;
    }
    export function timeline(opts?: TimelineOptions): Timeline;
    ```
    Defaults: duration 0.5, ease `power1Out`. Positions as GSAP: omitted = end; `"<"` = start of the previous child; `"+=x"`/`"-=x"` relative to the end; number = absolute. Stagger shifts child i by `i·stagger`. `repeat: -1` = forever; `repeatDelay` holds the end state between iterations. Start values of `to` are recorded once, at the child's first render, and reused on every loop; seeking before a child's start renders its recorded start.
    Element props (anything else throws `Error("timeline: unsupported prop <name>")`): `opacity` → `style.opacity`; `x`, `y` (px), `xPercent`, `yPercent`, `scale`, `scaleX`, `scaleY`, `rotateX`, `rotateY` (deg) → `style.transform = \`translate(${xp}%, ${yp}%) translate(${x}px, ${y}px) rotateX(${rx}deg) rotateY(${ry}deg) scale(${scale*scaleX}, ${scale*scaleY})\``; `strokeDashoffset` → `style.strokeDashoffset`; `--custom` → `style.setProperty("--custom", String(v))`; `set` also accepts string values for these. Untracked element defaults: opacity 1, translations/rotations 0, scales 1, custom props and dash offset 0 (no `getComputedStyle`). Plain-object targets read and write numeric fields.
  - `tween.ts`: `tween(o: { from: number; to: number; duration: number; delay?: number; ease?: Ease; onUpdate: (v: number) => void }, t?: Ticker): { cancel(): void; done: Promise<boolean> }` (`true` on completion, `false` on cancel).

- [ ] **Step 1: Write the failing tests** (plain objects and a fake element `{ style: { opacity: "", transform: "", strokeDashoffset: "", setProperty: vi.fn() } }` cast to `Element`)

```ts
it.each([linear, power1Out, power1InOut, power2In, power2Out, power2InOut, power3In, power3Out, power3InOut, backOut(2), elasticOut(1, 0.42)])("ease %# maps 0→0 and 1→1", (e) => {
  expect(e(0)).toBeCloseTo(0, 9); expect(e(1)).toBeCloseTo(1, 9);
});
it("matches GSAP curves", () => {
  expect(power2In(0.5)).toBeCloseTo(0.125); expect(power3Out(0.5)).toBeCloseTo(0.9375);
  expect(power2InOut(0.25)).toBeCloseTo(0.0625); expect(backOut(2)(0.5)).toBeCloseTo(1.125);
  expect(Math.max(...Array.from({ length: 50 }, (_, i) => elasticOut(1, 0.42)(i / 50)))).toBeGreaterThan(1.05);
});
it("places children like GSAP", () => {
  const a = { v: 0 }, b = { v: 0 };
  expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, "<").duration).toBe(1);
  expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, "+=0.5").duration).toBe(2.5);
  expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, "-=0.5").duration).toBe(1.5);
  expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, 3).duration).toBe(4);
});
it("eases, staggers and restores on seek back", () => {
  const xs = [{ v: 0 }, { v: 0 }, { v: 0 }];
  const tl = timeline().to(xs, { v: 1 }, { duration: 1, ease: linear, stagger: 0.2 });
  expect(tl.duration).toBeCloseTo(1.4);
  tl.seek(0.2); expect(xs.map((x) => x.v)).toEqual([expect.closeTo(0.2), 0, 0]);
  tl.seek(0); expect(xs[0]!.v).toBe(0);
});
it("fires calls forward only, and all of them on progress(1)", () => {
  const fn = vi.fn(); const tl = timeline().to({ v: 0 }, { v: 1 }, { duration: 1 }).call(fn, 0.5);
  tl.progress(1); tl.seek(0.2); tl.seek(0.4); expect(fn).toHaveBeenCalledTimes(1);
});
it("repeats with a delay and completes once", () => {
  const done = vi.fn(); const tl = timeline({ repeat: 2, repeatDelay: 1, onComplete: done }).to({ v: 0 }, { v: 1 }, { duration: 1 });
  tl.seek(5); expect(tl.loops).toBe(3); expect(done).toHaveBeenCalledTimes(1);
});
it("yoyos a repeated child", () => {
  const o = { v: 0 }; const tl = timeline().to(o, { v: 100 }, { duration: 1, ease: linear, repeat: 1, yoyo: true });
  tl.seek(1.5); expect(o.v).toBeCloseTo(50);
});
it("writes one transform string", () => {
  const el = fakeEl(); timeline().to(el, { x: 10, scale: 2 }, { duration: 1 }).progress(1);
  expect(el.style.transform).toBe("translate(0%, 0%) translate(10px, 0px) rotateX(0deg) rotateY(0deg) scale(2, 2)");
});
it("rejects unknown props", () => expect(() => timeline().to(fakeEl(), { width: 3 }).progress(1)).toThrow(/unsupported prop width/));
it("tween resolves false on cancel", async () => { const h = tween({ from: 0, to: 1, duration: 1, onUpdate: () => {} }, manualTicker()); h.cancel(); await expect(h.done).resolves.toBe(false); });
```

- [ ] **Step 2: Run** the three files → FAIL.
- [ ] **Step 3: Implement.** `play()` subscribes to the ticker and advances `seek(playhead + dt)`; `pause()`/`kill()` unsubscribe; `kill()` also drops recorded state.
- [ ] **Step 4: Run** → PASS.
- [ ] **Step 5: Commit** `feat(web): small timeline, tween and GSAP-compatible eases`

---

### Task 6: Runtime: capability tiers, motion toggle, idle loading, reveals

**Files:**
- Create: `apps/web/src/runtime/{capability.ts,motion.ts,idle.ts,reveal.ts}`, tests `apps/web/src/runtime/{capability,motion,reveal}.test.ts`, `apps/web/src/styles/motion.css`
- Modify: `apps/web/src/main.ts`, `apps/web/src/pages/routes.ts` (`IslandId = "holo" | "downloads" | "selfcheck" | "print" | "demos" | "decode"`), `apps/web/src/islandSources.ts` (drop `foil`, `steps`; `demos` and `decode` are added in Task 12), `apps/web/src/pages/Footer.tsx` (toggle button), `apps/web/src/pages/pages.test.ts`
- Delete: `src/islands/foil.ts`, `src/islands/steps.ts`, the `Foil` component and its CSS, the `steps` sticky markup in `views/security.tsx` (replaced in Task 14)

**Interfaces:**
- Produces:
  - `type FxTier = "full" | "lite" | "none"`; `interface CapabilityEnv { reducedMotion: boolean; motionOff: boolean; saveData: boolean; effectiveType: string | undefined; cores: number | undefined; memoryGb: number | undefined; coarsePointer: boolean; viewportWidth: number; webgl2: boolean }`; `fxTier(env): FxTier` (rules exactly as Global Constraints, missing `cores`/`memoryGb` count as 8); `readEnv(win: Window, storage: Pick<Storage, "getItem"> | null): CapabilityEnv` (WebGL2 probe with `failIfMajorPerformanceCaveat: true`, then `WEBGL_lose_context`).
  - `MOTION_KEY = "mosaic:motion"`, `MOTION_EVENT = "mosaic:motion"`; `safeStorage(win): Storage | null` (null when the accessor throws); `isMotionOff(storage): boolean` (value `"off"`, false on any error); `setMotionOff(off: boolean, storage, doc): void` (writes or removes the key in try/catch, sets/removes `html[data-motion="off"]`, dispatches `CustomEvent<{ off: boolean }>(MOTION_EVENT)` on `doc`); `motionAllowed(win, storage): boolean` = not reduced and not off; `wireMotionToggle(doc, storage): void` (unhides `button[data-motion-toggle]`, keeps `aria-pressed` = off, swaps its text between `data-label-pause` / `data-label-play`).
  - `afterLoadIdle(fn: () => void, timeoutMs = 3000): void` (after `load`, or at once when `readyState === "complete"`, then `requestIdleCallback(fn, { timeout })`, fallback `setTimeout(fn, 200)`).
  - `wireReveals(doc, allowed: boolean, IO: typeof IntersectionObserver = IntersectionObserver): void` – when allowed, `[data-reveal]` elements not intersecting in the observer's first callback get `reveal-pending`; on intersection (`rootMargin: "0px 0px -3% 0px"`) `reveal-pending` → `reveal-in`, then unobserve.
  - `main.ts` order: tokens + styles; `wireMenu`; `wireMotionToggle`; `wireReveals`; (`wireNav`, `wireJumps` from Task 9); `startIslands()` after two frames (skip `holo` when Save-Data); `afterLoadIdle(() => { tier = fxTier(readEnv(window, storage)); html.dataset.fx = tier; if (tier !== "none") import("./field/bootField.ts"); if (tier !== "none" && matchMedia("(hover: hover) and (pointer: fine)").matches) import("./micro/index.ts") })`. Task 7 and 11 create those modules; until then guard with the file existing (land the imports in those tasks).
- `motion.css`: `.reveal-pending { opacity: 0; transform: translateY(28px) }`, `.reveal-in { transition: opacity 650ms var(--ease-enter), transform 650ms var(--ease-enter) }`; intro: under `@media (prefers-reduced-motion: no-preference)` and `html:not([data-motion="off"])`, `[data-intro]` animates `intro` (from `opacity: 0; translateY(22px)`) 700 ms `var(--ease-enter)` both, delays `[data-intro="1"]` 40 ms … `[data-intro="6"]` 340 ms (60 ms steps); `html[data-motion="off"] *, html[data-motion="off"] *::before, html[data-motion="off"] *::after { animation: none !important; transition: none !important }`; showcase scrub (prototype 5627–5632) as `@supports (animation-timeline: view())` keyframes `rotateX(9deg) translateY(24px)` → `none translateY(-24px)`, `animation-range: entry 5% cover 60%`, ≥ 1000 px and no-preference only.
- Footer: `<button type="button" class="motion-toggle" data-motion-toggle data-label-pause={t("web.motion.pause")} data-label-play={t("web.motion.play")} aria-pressed="false" hidden>{t("web.motion.pause")}</button>` in `.site-footer__base`. Catalog (`site.ts`): `web.motion.pause` "Pause animations" / "Animationen anhalten"; `web.motion.play` "Play animations" / "Animationen abspielen".

- [ ] **Step 1: Write the failing tests**

```ts
const base: CapabilityEnv = { reducedMotion: false, motionOff: false, saveData: false, effectiveType: "4g", cores: 8, memoryGb: 8, coarsePointer: false, viewportWidth: 1440, webgl2: true };
it.each<[Partial<CapabilityEnv>, FxTier]>([
  [{}, "full"], [{ reducedMotion: true }, "none"], [{ motionOff: true }, "none"], [{ saveData: true }, "none"],
  [{ effectiveType: "2g" }, "none"], [{ effectiveType: "slow-2g" }, "none"], [{ webgl2: false }, "none"],
  [{ cores: 2 }, "none"], [{ memoryGb: 1 }, "none"], [{ effectiveType: "3g" }, "lite"], [{ coarsePointer: true }, "lite"],
  [{ cores: 4 }, "lite"], [{ memoryGb: 4 }, "lite"], [{ viewportWidth: 699 }, "lite"], [{ cores: undefined, memoryGb: undefined }, "full"],
])("%o → %s", (patch, tier) => expect(fxTier({ ...base, ...patch })).toBe(tier));
```

```ts
// @vitest-environment jsdom   (motion.test.ts)
it("treats a throwing storage as motion on", () => expect(isMotionOff({ getItem: () => { throw new Error("blocked"); } })).toBe(false));
it("persists, marks the root and notifies", () => {
  const seen: boolean[] = []; document.addEventListener(MOTION_EVENT, (e) => seen.push((e as CustomEvent<{ off: boolean }>).detail.off));
  setMotionOff(true, localStorage, document);
  expect(localStorage.getItem(MOTION_KEY)).toBe("off"); expect(document.documentElement.dataset["motion"]).toBe("off"); expect(seen).toEqual([true]);
});
```

```ts
// reveal.test.ts (jsdom, fake IO that reports the given entries on observe)
it("hides only what starts below the fold", () => { /* two [data-reveal] nodes, first intersecting at boot: only the second gets reveal-pending; a later intersecting entry swaps it to reveal-in */ });
it("does nothing when motion is not allowed", () => { wireReveals(document, false, FakeIO); expect(document.querySelector(".reveal-pending")).toBeNull(); });
```

`pages.test.ts` additions: every page HTML `not.toMatch(/reveal-pending|reveal-in/)`; every page has exactly one `data-motion-toggle`.

- [ ] **Step 2: Run** the new files → FAIL.
- [ ] **Step 3: Implement**; remove the foil/steps islands from `ISLANDS` and their CSS.
- [ ] **Step 4: Run** the files plus `pages.test.ts`, lint, typecheck → PASS.
- [ ] **Step 5: Commit** `feat(web): capability tiers, motion toggle, idle loading and CSS reveals`

---

### Task 7: The field: tower city, still fallback, governor, veil

**Files:**
- Create: `apps/web/src/field/{shaders.ts,gl.ts,atlas.ts,lens.ts,size.ts,governor.ts,veil.ts,bootField.ts}`, `apps/web/src/runtime/fxRegistry.ts`, tests `apps/web/src/field/{glsl,lens,size,governor,bootField}.test.ts`, `apps/web/src/styles/field.css`
- Modify: `apps/web/src/pages/render.tsx` (first child of `<body>`: `<div className="field-still" aria-hidden="true" />`; last child: `<p className="transit-hint" aria-hidden="true" hidden>{t("web.transition.hint")}</p>`), catalog `site.ts` (`web.transition.hint`: "Resolving" / "Entschlüsselt"), `main.ts` (the idle import)

**Interfaces:**
- Consumes: `FxTier` (Task 6), `tween`, `power2In` (Task 5), `MOTION_EVENT` (Task 6).
- Produces:
  - `shaders.ts`: `FIELD_VERT`, `FIELD_FRAG` – prototype lines 3338–3459 verbatim except the unused `uniform sampler2D uPlain;`; `FIELD_UNIFORMS` (the list of line 3460 without `uPlain`).
  - `gl.ts`: `interface FieldUniforms { time; lensX; lensY; lensR; scroll; wipe; page; heroY; cell; clickX; clickY; clickAge: number }` (all `number`), `interface FieldGl { canvas: HTMLCanvasElement; u: FieldUniforms; dpr: number; width: number; height: number; resize(): void; setAtlas(src: TexImageSource): void; render(): void; ready(): boolean; lost(): boolean }`, `createFieldGl(canvas, opts: { overlay: boolean; dprCap: number }): FieldGl | null` – port of prototype 3477–3585 with `failIfMajorPerformanceCaveat: true`; uses `KHR_parallel_shader_compile` when present (`ready()` is false until `COMPLETION_STATUS_KHR`, checked once per frame, at most 60 frames, then the link status decides); listens for `webglcontextlost` (`preventDefault`, `lost()` → true). Cell size 12 (field) / 17 (veil).
  - `atlas.ts`: `GLYPHS = "0123456789abcdef"`, `makeAtlas(doc): HTMLCanvasElement` (512×48, prototype 3463–3475).
  - `lens.ts`: `IDLE_MS = 2600`; `interface Lens { x; y; tx; ty; lastPointer; idleT: number; wasIdle: boolean; a0: number; s0: number }`; `createLens(w, h): Lens` (at 0.72 w, 0.4 h); `lensRadius(w, h): number` = clamp(min(w, h)·0.38, 180, 360); `stepLens(lens, dt, now, w, h): void` (prototype 5455–5481: idle orbit rx 0.36 w, ry 0.34 h, angular speed 0.17, 6 s blend; follow rate 7/s, idle 2.2/s).
  - `size.ts`: `MAX_PIXELS = 2_400_000`; `canvasSize(cssW, cssH, dpr, cap, maxPixels = MAX_PIXELS): { w: number; h: number; scale: number }` (scale = min(dpr, cap), lowered so `w·h ≤ maxPixels`; w, h ≥ 2, rounded).
  - `governor.ts`: `createGovernor(): { sample(dtMs: number): "keep" | "lite" | "stop" }` (30 warm-up frames ignored, then 90-frame means; > 25 ms → "lite" once; after "lite", > 45 ms → "stop").
  - `fxRegistry.ts`: `interface VeilApi { wipeIn(seconds: number): Promise<void>; reset(): void }`, `setVeil(v: VeilApi | null): void`, `getVeil(): VeilApi | null`.
  - `veil.ts`: `createVeil(doc, atlas: HTMLCanvasElement): VeilApi | null` (canvas `.veil`, own `createFieldGl({ overlay: true, dprCap: 1 })`; `wipeIn` shows the canvas, tweens `u.wipe` 0→1 with `power2In`, fades `.transit-hint` in from 0.18 of the way, resolves at the end; `reset()` hides both and sets `wipe = 0`).
  - `bootField.ts`: `interface Click { x: number; y: number; age: number }`, `interface FieldLayer { draw(now: number, lens: Lens, gl: FieldGl): void; setClick(c: Click | null): void; dispose(): void }`, `interface FieldHandle { readonly state: "running" | "still"; addLayer(l: FieldLayer): void; stop(): void }`, `bootField(doc: Document, tier: Exclude<FxTier, "none">, deps?: { createGl?: typeof createFieldGl; raf?: (cb: FrameRequestCallback) => number }): FieldHandle`.
    Behaviour: inserts `<canvas class="field" aria-hidden="true">` before `.field-still`; DPR cap 1.5 (full) / 1 (lite); loop = prototype `fieldFrame` 5448–5500 (`uPage` 1 on `body[data-page="home"]` else 0.62; `uHeroY` 0.5 vh home / 0.42 vh else; every other frame skipped when lite or scrolled past 1.9 vh); pointer and click wiring = prototype 5501–5537; atlas after `document.fonts.load("500 38px 'IBM Plex Mono'")`; adds `is-on` to the canvas after the first rendered frame (CSS fades it in over 600 ms) and then hides `.field-still`; pauses on `visibilitychange` hidden; `MOTION_EVENT` with `off: true`, governor "stop" or context loss → `stop()`: cancel the frame, remove `is-on`, unhide `.field-still`, `state = "still"`, `html.dataset.fxState = "still"`; governor "lite" → DPR 1, 30 fps, layers disposed. On `full` it creates the veil and calls `setVeil`. Sets `html.dataset.fxState` to the tier while running.
- `field.css`: prototype 329–400 (`.field`, `.field-data`, `.panel-fx`, `.field-still`, `.veil`, `.transit-hint`), plus `.field { opacity: 0; transition: opacity 600ms var(--ease-enter) } .field.is-on { opacity: 1 }`; `main`, header and footer sit above (`position: relative; z-index: 1`).

- [ ] **Step 1: Write the failing tests**

```ts
// glsl.test.ts: GLSL ES 3.00 keywords reserved for future use, and other words that fail to compile as identifiers
const RESERVED = ["attribute","varying","coherent","volatile","restrict","readonly","writeonly","resource","atomic_uint","noperspective","patch","sample","subroutine","common","partition","active","asm","class","union","enum","typedef","template","this","goto","inline","noinline","public","static","extern","external","interface","long","short","double","half","fixed","unsigned","superp","input","output","hvec2","hvec3","hvec4","dvec2","dvec3","dvec4","fvec2","fvec3","fvec4","sampler3DRect","filter","sizeof","cast","namespace","using","image1D","image2D","image3D","imageCube","image1DArray","image2DArray","imageBuffer","iimage1D","iimage2D","iimage3D","uimage1D","uimage2D","uimage3D"];
const SOURCES = { FIELD_VERT, FIELD_FRAG, HOLO_VERT: VERT, HOLO_FRAG: FRAG };
it.each(Object.entries(SOURCES))("%s uses no reserved word and no double underscore", (_n, src) => {
  const code = src.replace(/\/\*[\s\S]*?\*\/|\/\/.*$/gm, "");
  const ids = new Set(code.match(/\b[A-Za-z_]\w*\b/g));
  expect(RESERVED.filter((w) => ids.has(w))).toEqual([]);
  expect([...ids].filter((i) => i.includes("__"))).toEqual([]);
  expect(code.startsWith("#version 300 es")).toBe(true);
});
```

```ts
it("caps 4K at 2x under the pixel ceiling", () => { const s = canvasSize(3840, 2160, 2, 1.5); expect(s.w * s.h).toBeLessThanOrEqual(2_400_000); expect(s.scale).toBeLessThan(1.5); });
it("keeps a small canvas at the DPR cap", () => expect(canvasSize(390, 844, 3, 1)).toEqual({ w: 390, h: 844, scale: 1 }));
it("clamps the lens radius", () => { expect(lensRadius(320, 568)).toBe(180); expect(lensRadius(3840, 2160)).toBe(360); });
it("orbits inside the viewport after the pointer rests", () => {
  const l = createLens(1440, 900); l.lastPointer = 0;
  for (let i = 0; i < 3600; i++) { stepLens(l, 1 / 60, 3000 + i * 16.7, 1440, 900); expect(l.x).toBeGreaterThan(-200); expect(l.x).toBeLessThan(1640); }
  expect(l.wasIdle).toBe(true);
});
it("degrades in two steps", () => {
  const g = createGovernor(); const run = (ms: number, n: number) => { let r = "keep"; for (let i = 0; i < n; i++) r = g.sample(ms); return r; };
  expect(run(16, 120)).toBe("keep"); expect(run(30, 90)).toBe("lite"); expect(run(50, 90)).toBe("stop");
});
```

```ts
// bootField.test.ts (jsdom): createGl stub returns a FieldGl fake with lost() false; raf stub queues callbacks
it("drops to the still on context loss", () => {
  document.body.innerHTML = '<div class="field-still"></div>';
  const h = bootField(document, "full", { createGl: fakeGl, raf: queue.raf });
  queue.flush(2);
  document.querySelector("canvas.field")!.dispatchEvent(new Event("webglcontextlost"));
  queue.flush(1);
  expect(h.state).toBe("still"); expect(document.querySelector(".field-still")!.hasAttribute("hidden")).toBe(false);
  expect(queue.pending()).toBe(0);
});
it("stops when the motion toggle turns off", () => { /* dispatch MOTION_EVENT {off:true}; expect state "still" */ });
it("returns a still handle when no context can be created", () => expect(bootField(document, "lite", { createGl: () => null, raf: queue.raf }).state).toBe("still"));
```

- [ ] **Step 2: Run** the five files → FAIL.
- [ ] **Step 3: Implement** by porting the named prototype ranges (TS strict, no `any`, uniforms typed). Land the idle import in `main.ts`.
- [ ] **Step 4: Run** the tests, lint, typecheck, `pnpm --filter @mosaic/web build` (budget for `src/field/bootField.ts` must hold) → PASS.
- [ ] **Step 5: Commit** `feat(web): tower-city field with still fallback, governor and veil`

---

### Task 8: Data layer and panel glyph rings

**Files:**
- Create: `apps/web/src/field/{towers.ts,dataLayer.ts,panelRings.ts}`, tests `apps/web/src/field/{towers,panelRings}.test.ts`
- Modify: `apps/web/src/field/bootField.ts` (full: `addLayer(createDataLayer(doc, gl))`; full and lite: `createPanelRings(doc)`; both disposed on `stop()`)

**Interfaces:**
- Consumes: `FieldLayer`, `Click`, `FieldGl`, `Lens` (Task 7), `fieldColors` (Task 2), `timeline`, `power2Out`, `elasticOut` (Task 5).
- Produces:
  - `towers.ts`: `h21(x: number, y: number): number`, `vn(x: number, y: number): number`, `towerH(i, j, cs, t, lx, ly, R: number, click: Click | null): number` (prototype 4999–5045, the JS mirror of the shader's `towerH`).
  - `dataLayer.ts`: `label(kind: 0 | 1 | 2, i: number, j: number, h: number): string` (prototype 5104–5108); `createDataLayer(doc, gl: FieldGl): FieldLayer | null` (canvas `.field-data`, prototype 4990–5296; colours `rgba(${fieldColors.link.join(",")},${a})` etc.; picks every 250 ms; at most 12 near and 3 far nodes).
  - `panelRings.ts`: `ringBand(dd: number, rad: number, front: number, tail: number): number` (prototype 5410: outside `exp(−((dd−rad)/front)²)`, inside `exp(−(rad−dd)/tail)·(0.35 + 0.65·exp(−((rad−dd)/front)²) + 0.25)`); `createPanelRings(doc): { dispose(): void }` (one `.panel-fx` canvas created on first use, selector `.tile, .plan, .card, .platform__link, .step-card, .see__card`, cell 11 px, 1.35 s rings, strength 1.4; the press spring of 5341–5358 as a `timeline` – 0.16 s `power2Out` to `x: −ux·2.5, y: −uy·2.5, scale: 1.012, rotateY: −ux·1.6, rotateX: uy·1.6`, then 1.1 s `elasticOut(1, 0.42)` back; ignores presses while `html[data-motion="off"]`).

- [ ] **Step 1: Write the failing tests**

```ts
it("hashes into [0, 1)", () => { for (let i = 0; i < 500; i++) { const v = h21(i * 1.7, i * 0.3); expect(v).toBeGreaterThanOrEqual(0); expect(v).toBeLessThan(1); } });
it("raises towers near the lens only", () => {
  expect(towerH(1000, 1000, 12, 6.5, 0, 0, 260, null)).toBe(0);
  const near = Array.from({ length: 25 }, (_, k) => towerH(k % 5, Math.floor(k / 5), 12, 6.5, 30, 30, 260, null));
  expect(Math.max(...near)).toBeGreaterThan(0);
});
it("formats labels", () => { expect(label(1, 0, 0, 0.5)).toBe("Δ +6.00"); expect(label(2, 0, 0, 0.5)).toBe("n=256"); expect(label(0, 3, 4, 0.5)).toMatch(/^[0-9a-f]{1,4} {2}50\.0$/); });
it("peaks on the ring and fades outside", () => {
  expect(ringBand(100, 100, 9, 70)).toBeCloseTo(1.25); expect(ringBand(109, 100, 9, 70)).toBeCloseTo(Math.exp(-1));
  expect(ringBand(200, 100, 9, 70)).toBeLessThan(1e-6);
});
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (ports). **Step 4: Run** tests, lint, typecheck, build (field budget) → PASS.
- [ ] **Step 5: Commit** `feat(web): field data layer and panel glyph rings`

---

### Task 9: Page transitions and in-page jumps

**Files:**
- Create: `apps/web/src/nav/{intercept.ts,wireNav.ts,jump.ts}`, tests `apps/web/src/nav/{intercept,wireNav}.test.ts`, `apps/web/src/styles/transitions.css`
- Modify: `apps/web/src/main.ts`, `apps/web/src/pages/Header.tsx` and `Footer.tsx` (every page link in the header nav, menu dialog, brand and footer columns gets `data-nav=""`)

**Interfaces:**
- Consumes: `getVeil` (Task 7), `tween`, `power2In`, `power3InOut` (Task 5), `motionAllowed` (Task 6).
- Produces:
  - `shouldWipe(e: Pick<MouseEvent, "button" | "metaKey" | "ctrlKey" | "shiftKey" | "altKey" | "detail" | "defaultPrevented">, a: { href: string; target: string; hasDownload: boolean; isNav: boolean }, here: { origin: string; pathname: string }): boolean` – true only for: `isNav`, `button === 0`, no modifier, `detail > 0`, not prevented, `target` `""` or `"_self"`, no `download`, same origin, different pathname.
  - `wireNav(doc: Document, win: Window, veil: () => VeilApi | null = getVeil): void` – one capturing `click` listener on `a[data-nav]`: when `shouldWipe` and `veil()` and motion allowed: `preventDefault`, ignore further clicks until navigation, `await v.wipeIn(0.32)`, `win.location.assign(href)`. `pageshow` with `persisted` → `veil()?.reset()`. `pageswap` → `event.viewTransition?.skipTransition()` while motion is off (declare a local `PageSwapEventLike` type if `lib.dom` lacks it).
  - `wireJumps(doc, win): void` – `a[data-jump]` with `href="#id"`: pointer clicks with motion allowed run the prototype `jumpTo` (5728–5776) on `tween` (lift the pressed `.holo` to `position: fixed` via `style.setProperty`, dim the section copy to 0.25 with −14 px, glide `scrollTo` over `min(1.25, 0.7 + |Δ|/2400)` s with `power3InOut` after 0.22 s, any `wheel`/`touchstart`/`keydown` stops it); afterwards `history.pushState(null, "", "#id")` and focus the target (`tabindex="-1"` set in markup). Keyboard activation and reduced motion use the native anchor jump.
- `transitions.css` (inside `@media (prefers-reduced-motion: no-preference)`): `@view-transition { navigation: auto; }`; `.site-header { view-transition-name: site-header }`; `::view-transition-old(root) { animation: vt-out 200ms var(--ease-standard) both }` (`opacity → 0`, `translateY(-18px)`); `::view-transition-new(root) { animation: vt-wipe 460ms var(--ease-enter) both; mask-image: linear-gradient(135deg, var(--mask-solid) 45%, transparent 55%); mask-size: 300% 300% }`, `@keyframes vt-wipe { from { mask-position: 100% 100% } to { mask-position: 0 0 } }`.

- [ ] **Step 1: Write the failing tests**

```ts
const click = { button: 0, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, detail: 1, defaultPrevented: false };
const link = { href: "https://mosaic.example/features/", target: "", hasDownload: false, isNav: true };
const here = { origin: "https://mosaic.example", pathname: "/" };
it("wipes a plain pointer click to another page", () => expect(shouldWipe(click, link, here)).toBe(true));
it.each<[string, Partial<typeof click>, Partial<typeof link>]>([
  ["ctrl", { ctrlKey: true }, {}], ["meta", { metaKey: true }, {}], ["shift", { shiftKey: true }, {}], ["alt", { altKey: true }, {}],
  ["middle", { button: 1 }, {}], ["keyboard", { detail: 0 }, {}], ["prevented", { defaultPrevented: true }, {}],
  ["blank", {}, { target: "_blank" }], ["download", {}, { hasDownload: true }], ["not nav", {}, { isNav: false }],
  ["other origin", {}, { href: "https://github.com/x" }], ["same page", {}, { href: "https://mosaic.example/#main" }],
])("never wipes on %s", (_n, e, a) => expect(shouldWipe({ ...click, ...e }, { ...link, ...a }, here)).toBe(false));
```

```ts
// wireNav.test.ts (jsdom)
it("wipes, then navigates", async () => { /* a[data-nav] to /features/, fake veil.wipeIn resolving; MouseEvent("click", { bubbles: true, cancelable: true, detail: 1 }); expect defaultPrevented, wipeIn called with 0.32, then win.location.assign("/features/" absolute) */ });
it("lets a ctrl-click through", () => { /* ctrlKey: true → not prevented, wipeIn not called */ });
it("uncovers a page restored from the back-forward cache", () => { const reset = vi.fn(); wireNav(document, window, () => ({ wipeIn: vi.fn(), reset })); window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true })); expect(reset).toHaveBeenCalled(); });
it("navigates plainly without a veil", () => { /* veil() → null: not prevented */ });
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**; wire both into `main.ts`; add `data-nav` attributes.
- [ ] **Step 4: Run** tests, lint, typecheck → PASS.
- [ ] **Step 5: Commit** `feat(web): view transitions, veil wipe for nav links, watched in-page jumps`

---

### Task 10: Report and security challenge pages

**Files:**
- Create: `apps/web/src/pages/views/help.tsx`, `packages/i18n/src/catalog/{en,de}/web/help.ts`
- Modify: `routes.ts`, `views/index.ts`, `site.ts`, `Footer.tsx`, `views/security.tsx` (challenge block), `catalog/{en,de}/web/index.ts`, `vite.config.ts` (`sources: [...SOURCE_URLS, ...Object.values(LINKS)]`), `pages.test.ts`

**Interfaces:**
- Produces:
  - Routes: `{ id: "report", en: "/report/", de: "/de/melden/", index: true, meta: "report", tint: "coral" }`, `{ id: "challenge", en: "/security/challenge/", de: "/de/sicherheit/challenge/", index: true, meta: "challenge", tint: "teal", parent: "security" }`; `RouteId` and `MetaKey` gain `"report"` and `"challenge"`.
  - `site.ts`: `REPO = "https://github.com/StephanBaum/Mosaik"`; `LINKS = { issues: \`${REPO}/issues/new/choose\`, advisory: \`${REPO}/security/advisories/new\`, policy: \`${REPO}/blob/main/SECURITY.md\`, rules: \`${REPO}/blob/main/docs/BUG_BOUNTY.md\`, targets: \`${REPO}/blob/main/docs/CHALLENGE.md\`, hallOfFame: \`${REPO}/blob/main/docs/HALL_OF_FAME.md\` } as const`; `site.challengeOpen: false` (true once `{CONTEST_HOST}` is published, gate G10).
  - `Report({ ctx })`: `PageHero` (eyebrow, title, sub), three `card`s (bug: text + `ArrowLink` to `LINKS.issues`; vuln: text + links to `LINKS.advisory` and `LINKS.policy`; never: text), then an `ArrowLink` to the challenge. External links get `rel="noopener noreferrer"`.
  - `Challenge({ ctx })`: `PageHero` with `LineButton`s to `LINKS.rules` and `LINKS.targets`; the status line (`open` or `soon` by `site.challengeOpen`); "How it runs" `Checks` (how1–how4); "Seven targets" ruled list `C1`…`C7` (mono id + text); out-of-scope `fineprint`; reward paragraph; links to `LINKS.hallOfFame` and `/report/#vuln`. Ids: `report` page sections `bug`, `vuln`, `never`.
  - Footer `.site-footer__legal` gains `web.footer.report` → report and `web.footer.challenge` → challenge. Security page gets a block (`web.security.challenge.*`) linking to the challenge.
- Copy (EN / DE), exact:

| key | EN | DE |
|---|---|---|
| `web.report.meta.title` | Report a problem · {name} | Problem melden · {name} |
| `web.report.meta.description` | Found a bug or a security hole in {name}? Here is where to report it, and what to leave out. | Du hast einen Fehler oder eine Sicherheitslücke in {name} gefunden? Hier meldest du sie, und das lässt du dabei weg. |
| `web.report.eyebrow` | Report a problem | Problem melden |
| `web.report.title` | Found something? Tell us. | Etwas gefunden? Sag es uns. |
| `web.report.sub` | Every report gets read. Security reports go through a private channel and get an answer within seven days. | Jede Meldung wird gelesen. Sicherheitsmeldungen gehen über einen privaten Kanal und bekommen innerhalb von sieben Tagen eine Antwort. |
| `web.report.bug.title` | Something doesn’t work | Etwas funktioniert nicht |
| `web.report.bug.text` | Open a public issue on GitHub. Tell us what you did, what you expected and what happened, plus your system and your {name} version. | Eröffne ein öffentliches Issue auf GitHub. Schreib, was du gemacht hast, was du erwartet hast und was passiert ist, dazu dein System und deine {name}-Version. |
| `web.report.bug.link` | Open an issue on GitHub | Issue auf GitHub eröffnen |
| `web.report.vuln.title` | You found a security hole | Du hast eine Sicherheitslücke gefunden |
| `web.report.vuln.text` | Please don’t post it in public. Report it privately on GitHub. We confirm within 7 days and assess within 14. We publish when it’s fixed, or after 90 days, whichever comes first. | Bitte poste sie nicht öffentlich. Melde sie privat über GitHub. Wir bestätigen innerhalb von 7 Tagen und bewerten innerhalb von 14. Wir veröffentlichen, sobald sie behoben ist, spätestens nach 90 Tagen. |
| `web.report.vuln.link` | Report privately on GitHub | Privat über GitHub melden |
| `web.report.vuln.policy` | Read the security policy | Sicherheitsrichtlinie lesen |
| `web.report.never.title` | Never send us secrets | Schick uns nie Geheimnisse |
| `web.report.never.text` | No report needs your passwords, your master password, your Secret Key, your Emergency Kit or your vault files. If a screenshot shows any of them, cover it first. | Keine Meldung braucht deine Passwörter, dein Master-Passwort, deinen Secret Key, dein Emergency Kit oder deine Tresordateien. Wenn ein Screenshot etwas davon zeigt, deck es vorher ab. |
| `web.report.challenge` | Want to dig deeper? Join the security challenge. | Du willst tiefer graben? Mach bei der Security Challenge mit. |
| `web.challenge.meta.title` | Security challenge · {name} | Security Challenge · {name} |
| `web.challenge.meta.description` | Try to break {name}. Seven targets, a public test server, fair rules and safe harbor. | Versuch, {name} zu knacken. Sieben Ziele, ein öffentlicher Testserver, faire Regeln und Safe Harbor. |
| `web.challenge.eyebrow` | Security challenge | Security Challenge |
| `web.challenge.title` | Break it. We mean it. | Knack es. Wir meinen das ernst. |
| `web.challenge.sub` | We don’t hide behind a seal. We publish targets, hand you what an attacker would have, and thank everyone who finds a hole. | Wir verstecken uns nicht hinter einem Siegel. Wir veröffentlichen Ziele, geben dir, was ein Angreifer hätte, und danken allen, die eine Lücke finden. |
| `web.challenge.rules` | Read the rules | Regeln lesen |
| `web.challenge.targetsLink` | See the targets | Ziele ansehen |
| `web.challenge.open` | The test server is open. Its address is listed with the targets. | Der Testserver ist offen. Seine Adresse steht bei den Zielen. |
| `web.challenge.soon` | The test server opens with the first season. The rules and targets are public already. | Der Testserver öffnet mit der ersten Saison. Regeln und Ziele sind schon öffentlich. |
| `web.challenge.how.title` | How it runs | So läuft es |
| `web.challenge.how1` | Open all the time, in seasons. Each season starts with fresh data and new flags. | Immer offen, in Saisons. Jede Saison startet mit frischen Daten und neuen Flags. |
| `web.challenge.how2` | The test server is separate and holds no real data. It’s wiped every week. | Der Testserver ist getrennt und enthält keine echten Daten. Er wird jede Woche gelöscht. |
| `web.challenge.how3` | Find a flag, then report how you got it. A flag without steps doesn’t count. | Finde eine Flag und melde, wie du sie bekommen hast. Eine Flag ohne Schritte zählt nicht. |
| `web.challenge.how4` | Safe harbor: research within the rules is welcome. The rules say exactly what we promise. | Safe Harbor: Forschung im Rahmen der Regeln ist willkommen. Was wir zusagen, steht genau in den Regeln. |
| `web.challenge.targets.title` | Seven targets | Sieben Ziele |
| `web.challenge.c1` | Hostile server: you get its full data directory. | Feindlicher Server: Du bekommst sein komplettes Datenverzeichnis. |
| `web.challenge.c2` | Stolen device: a locked local store, without the keystore. | Gestohlenes Gerät: ein gesperrter lokaler Speicher, ohne Schlüsselspeicher. |
| `web.challenge.c3` | Export file: protected by a six-word passphrase. | Exportdatei: geschützt mit einer Passphrase aus sechs Wörtern. |
| `web.challenge.c4` | Second factor: the password and Secret Key are public, the second factor is not. | Zweiter Faktor: Passwort und Secret Key sind öffentlich, der zweite Faktor nicht. |
| `web.challenge.c5` | Other accounts: read or change someone else’s records. | Fremde Konten: Lies oder ändere die Daten von jemand anderem. |
| `web.challenge.c6` | Browser extension: make it fill the wrong site, or fill without a click. | Browser-Erweiterung: Bring sie dazu, die falsche Website auszufüllen, oder ohne Klick. |
| `web.challenge.c7` | Desktop app: read vault data as another user or from a sandbox. | Desktop-App: Lies Tresordaten als anderer Nutzer oder aus einer Sandbox. |
| `web.challenge.scope` | Not part of it: denial of service, social engineering, physical access, third-party services and this website. Bugs on this website are still welcome as reports. | Nicht dabei: Denial of Service, Social Engineering, physischer Zugriff, Dienste Dritter und diese Website. Fehler auf dieser Website kannst du trotzdem melden. |
| `web.challenge.reward` | By default we say thank you in the hall of fame, if you want to be listed. If a season has prizes, the rules list them. | Standardmäßig danken wir dir in der Hall of Fame, wenn du genannt werden willst. Hat eine Saison Preise, stehen sie in den Regeln. |
| `web.challenge.hallOfFame` | Hall of fame | Hall of Fame |
| `web.challenge.report` | Report a finding | Fund melden |
| `web.security.challenge.title` | Don’t take our word for it. | Glaub uns nicht einfach. |
| `web.security.challenge.text` | Try to break {name} in the open security challenge. Seven targets, a public test server, and a thank-you for every hole you find. | Versuch, {name} in der offenen Security Challenge zu knacken. Sieben Ziele, ein öffentlicher Testserver und ein Dankeschön für jede Lücke, die du findest. |
| `web.security.challenge.link` | To the challenge | Zur Challenge |
| `web.footer.report` | Report a problem | Problem melden |
| `web.footer.challenge` | Security challenge | Security Challenge |

- [ ] **Step 1: Write the failing tests** (`pages.test.ts`)

```ts
it("routes the report and challenge pages in both languages", () => {
  expect(lookup("/report/")?.route.id).toBe("report"); expect(lookup("/de/melden/")?.route.id).toBe("report");
  expect(lookup("/de/sicherheit/challenge/")?.route.id).toBe("challenge");
});
it("sends reports to GitHub, never to a form", () => {
  const html = pages.find((p) => p.page === "report" && p.locale === "en")!.html;
  expect(html).toContain(`href="${LINKS.issues}"`); expect(html).toContain(`href="${LINKS.advisory}"`);
  expect(html).not.toMatch(/<form|<input|<textarea/);
});
it("lists seven challenge targets", () => {
  const html = pages.find((p) => p.page === "challenge" && p.locale === "de")!.html;
  for (const id of ["C1", "C2", "C3", "C4", "C5", "C6", "C7"]) expect(html).toContain(`>${id}<`);
});
```

Add `web.challenge.(open|soon)` to `OFF_PAGE` for the state that is off (like the capture keys).

- [ ] **Step 2: Run** `pages.test.ts` → FAIL. **Step 3: Implement** with existing components (restyled in Task 15). **Step 4: Run** pages test, catalog test, lint, typecheck, build (link and origin checks) → PASS.
- [ ] **Step 5: Commit** `feat(web): report a problem and security challenge pages`

---

### Task 11: The visual system: stylesheet, chrome, shared components, micro-interactions

**Files:**
- Rewrite `apps/web/src/styles/` as: `index.css` (imports, in order), `base.css` (proto 123–234), `type.css` (235–328), `field.css` (Task 7), `header.css` (401–612, adapted to the existing `<dialog class="menu">`: `dialog.menu[open]` replaces `.menu.is-open`; language switch stays links: style `.lang a` like the prototype's `.lang button`, `[aria-current="page"]` like `[aria-pressed="true"]`), `layout.css` (613–719), `buttons.css` (720–939), `hero.css` (940–1054), `frames.css` (1055–1154; keep the existing `crops.css` positions), `grids.css` (1155–1396), `sections.css` (1397–1481), `footer.css` (3117–3246 plus `.motion-toggle`), `motion.css` (Task 6 + 3247–3294), `transitions.css` (Task 9). Remove `patterns.css` and `home.css` rules that nothing renders any more.
- Create: `apps/web/src/micro/index.ts`
- Modify: `Header.tsx`, `Footer.tsx`, `ui.tsx`, `views/parts.tsx`, `render.tsx`

**Interfaces:**
- Consumes: tokens (Task 2), `timeline`, `power3Out`, `power2Out`, `power2InOut` (Task 5), `motionAllowed` (Task 6).
- Produces:
  - Component signatures (existing names kept, props added): `HoloButton({ href, label, icon?, ambient?: boolean, jump?: boolean })` (`ambient` → `data-ambient="true"`; `jump` → `data-jump=""`, an in-page jump link for Task 9), `LineButton({ href, label, large?, external?: boolean })`, `ArrowLink({ href, label, className?, external?: boolean })`, `PageHero({ ctx, eyebrow?, title, sub?, actions?, visual?, crumbs?, className? })` (prototype 4271–4279; `data-intro="1"`…`"5"` on crumbs, eyebrow, h1, sub, actions, visual), `PageCta({ ctx, title, text?, secondary? })` (4281), `Tile({ icon, title, text, visual?, wide?: boolean })` (4303), `Point({ icon, title, text })`, `IconTile`, `Badges({ ctx, items, center?: boolean })` (4172). `external` adds `rel="noopener noreferrer"`.
  - `bootMicro(doc): void` – prototype 4048–4114 on `timeline` (rim fill `--bl-fill`, glow `--bl-glow`, glint `--bl-glint`/`--bl-glint-o`, pointer position `--bl-x`/`--bl-y` in px, nav underline side `--ul-pos`), selectors `RING = ".button--line"`, `GLOW = ".button--line, .button--solid, .tile, .plan, .card, .platform__link, .step-card, .see__card, .ds, .di, .db, .da, .terminal"`, `UNDERLINE = ".site-nav__links a, .lang a"`; ignores touch pointers; stops on `MOTION_EVENT` off. Without it the stylesheet's hover states still work (prototype's CSS fallbacks).
  - Header: bolder nav per prototype (`.site-nav__links a` 0.9375rem, weight 640, `--paper`); five links unchanged.
- No inline `style=` anywhere (prototype markup uses a few: turn them into classes, e.g. `.ruled--spaced`, `.section--flush`).

- [ ] **Step 1: Write the failing tests** (`pages.test.ts`)

```ts
it.each(pages.map((p) => [p.file, p] as const))("%s paints the field still and the transit hint once", (_f, p) => {
  expect(p.html.match(/class="field-still"/g)).toHaveLength(1);
  expect(p.html.match(/class="transit-hint"/g)).toHaveLength(1);
});
it.each(pages.map((p) => [p.file, p] as const))("%s marks nav links and opens external links safely", (_f, p) => {
  expect(p.html).toMatch(/<nav class="site-nav"[\s\S]*?data-nav=""/);
  for (const m of p.html.matchAll(/<a [^>]*href="https:\/\/[^"]+"[^>]*>/g)) expect(m[0]).toContain('rel="noopener noreferrer"');
});
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** the stylesheet port and the components; keep every existing view rendering (Tasks 12–15 recompose them).
- [ ] **Step 4: Run** pages test, tokens test, lint, typecheck, build (CSS ≤ 30 000 B gzip) → PASS. Open `pnpm --filter @mosaic/web dev` home and one subpage side by side with the reference copy at 1440 and 390 px; header, footer, hero and buttons match.
- [ ] **Step 5: Commit** `feat(web): prototype visual system, chrome and micro-interactions`

---

### Task 12: Interface demos (markup + timelines)

**Files:**
- Create: `apps/web/src/pages/demos/{FillDemo,SyncDemo,AgentDemo,ImportDemo,PhoneDemo,CliDemo}.tsx`, `apps/web/src/demos/{text.ts,fill.ts,sync.ts,agent.ts,importer.ts,phone.ts,cli.ts,cipher.ts,nextSteps.ts,bootDemos.ts,decode.ts}`, test `apps/web/src/demos/demos.test.ts`, `apps/web/src/styles/demos.css` (proto 1482–2566)
- Create: `packages/i18n/src/catalog/{en,de}/web/launch.ts` (demo keys below; merged in `index.ts`)
- Modify: `apps/web/src/main.ts` (`ISLANDS.demos`, `ISLANDS.decode`), `islandSources.ts`

**Interfaces:**
- Consumes: `timeline`, eases (Task 5), `ticker` (Task 5), `motionAllowed`, `MOTION_EVENT` (Task 6).
- Produces:
  - Markup: each component renders the prototype markup (fill 4179–4196, sync 4198–4207, agent 4209–4227, import 4229–4238, phone 4240–4254, cli 4256–4262) as TSX, root `data-island="demos" data-demo="<name>" role="img" aria-label=…`, inside `aria-hidden="true"`, **in its final frame** (what the timeline shows at `progress(1)`), `js-*` hooks kept. Every string the timeline swaps in (e.g. `reading`, `preview`, the look-alike status) is rendered as a `data-text-*` attribute on its element and read from there, so no copy lives in JS and the catalog coverage test finds it. The existing `CipherMock` gets `data-island="demos" data-demo="cipher"`; the Next page's step list gets `data-demo="nextSteps"`.
  - `text.ts`: `decodeTo(tl: Timeline, el: Element, text: string, duration: number, at?: Position): void` (prototype 4570–4589: resolves left to right out of hex, `power1InOut`, ends on `text`); `typeTo(tl, el, text, duration, at?)` (4592–4602).
  - Each demo module: `export function build(root: HTMLElement): Timeline | null` – the prototype builder ported; every GSAP `className` tween becomes `.call(() => el.classList.toggle(…))`; every `innerHTML` becomes DOM building; initial states are explicit `.set(…, 0)` (the timeline reads no computed style).
  - `bootDemos(doc): void` – `DEMOS: Record<string, (root: HTMLElement) => Timeline | null>`; for each `[data-demo]`: build; with motion not allowed → `progress(1)` and stop; else play while intersecting (`rootMargin: "80px"`), pause when not; after `MAX_LOOPS = 3` completed loops `progress(1)` and kill; `MOTION_EVENT` off → `progress(1)`, kill all.
  - `bootDecode(doc): void` – for `h1[data-decode]` when motion allowed, viewport ≥ 700 px and `scrollY === 0`: lock each `.line`'s height (`style.setProperty("min-height", …)`), add `is-decoding` (`white-space: nowrap`), set `aria-label` to the full text, decode each line at `0.1 + i·0.12` s for 0.5 s, then remove the lock, class and `aria-label`.
  - Demo CLI runs (fixed literals, real syntax from `docs/CLI.md`): `["mosaic totp github", "<strong>492 039</strong>"]`, `["mosaic run --env GITHUB_TOKEN=mosaic://work/github/token -- gh repo list", "12 repositories · token <strong>masked</strong>"]`, `["mosaic ssh list", "ed25519 deploy@example.com · <strong>asks before every use</strong>"]` (the `<strong>` parts built as elements). Import demo sources: `[["Chrome Passwords.csv", 184, 1], ["1Password.1pux", 312, 3], ["lastpass_export.csv", 96, 2]]`, chips `Chrome`, `1Password`, `LastPass`.
- Catalog `launch.ts` (prototype `x.*` renamed; EN / DE verbatim from `i18n.json` unless given here): `web.demo.fill.{signin,email,password,continue,lookalike,nomatch,nothing}`, `web.demo.fill.statusOk` ("Filled on {site}" / "Ausgefüllt auf {site}"), `web.demo.fill.statusNo`, `web.demo.sync.{panel,device,server,signed,checkpoint,now,checked,older}`, `web.agent.t.{session,user,plan,inject,ok,ticket,refused,done,masked,foot}`, `web.tools.import.{drop,reading,preview,dup,items,dups,button,done}`, `web.tools.phone.{app,signin,user,pass,button,unlock}`. `x.agent.badge`, `x.footer.made`, `x.nav.*`, `x.next.crumbRoot` are not ported.

- [ ] **Step 1: Write the failing tests**

```ts
// @vitest-environment jsdom
const ALL = ["fill", "sync", "agent", "import", "phone", "cli", "cipher", "nextSteps"] as const;
it.each(ALL)("%s builds a bounded story that ends on its static markup", (name) => {
  const root = mount(name);               // renders the TSX component with renderToStaticMarkup into document.body
  const before = root.innerHTML;
  const tl = DEMOS[name](root)!;
  expect(tl.duration).toBeGreaterThan(1); expect(tl.duration).toBeLessThan(20);
  tl.progress(1);
  expect(normalise(root.innerHTML)).toBe(normalise(before));   // final frame == no-JS markup (style attributes stripped)
});
it("stops after three loops on the final frame", () => { /* fake ticker; play a demo for 4 × (duration + repeatDelay); expect tl killed and final frame shown */ });
it("shows final frames without motion", () => { /* setMotionOff(true); bootDemos(document); every demo already at progress(1); no ticker subscribers */ });
it("never writes HTML strings", () => { for (const f of demoSources()) expect(f).not.toMatch(/innerHTML|outerHTML|insertAdjacentHTML/); });
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** the test, catalog tests, lint, typecheck, build (demos ≤ 12 000 B) → PASS.
- [ ] **Step 5: Commit** `feat(web): interface demos on the in-house timeline`

---

### Task 13: Home page per the prototype

**Files:**
- Modify: `apps/web/src/pages/views/home.tsx`, `packages/i18n/src/catalog/{en,de}/web/launch.ts`, `landing.ts` (one value), `src/pages/pages.test.ts`
- Create: `apps/web/src/styles/home.css` (hero home, showcase, plans/final: proto 940–1054 home parts, 2533–2566, 2757–2863)

**Interfaces:**
- Consumes: Task 11 components, Task 12 demos (`FillDemo`, `AgentDemo`, `ImportDemo`, `PhoneDemo`), existing `PlanCards`, `WhereStories`, `Crop`, `ProductFrame`, `WarnMock`.
- Produces: the home composition of prototype 4304–4393, in order: hero (`h1[data-decode][data-island="decode"]`, holo `ambient` + line button), showcase (`ProductFrame` vault, eager, `data-showcase`), reasons (ruled 3), where stories, features bento (FillDemo tile wide, passkeys, codes, desktop wide, phone, tracking), **agent** section (`id="agent"`, no badge), **tools** section (`id="tools"`: ImportDemo + caption, PhoneDemo + caption, badges incl. `web.tools.programs`), pricing split, final CTA. `withCode(t, key, ref)` helper renders `{ref}` as `<code>`.
- Copy (EN / DE), exact; new keys in `launch.ts`:

| key | EN | DE |
|---|---|---|
| `web.agent.eyebrow` | Agent ready | Bereit für Agenten |
| `web.agent.title` | AI agents can use your API keys without seeing them. | KI-Agenten nutzen deine API-Keys, ohne sie zu sehen. |
| `web.agent.intro` | The agent gets a reference like {ref}. {name} puts the real key into the request, and only when it goes to api.openai.com. | Der Agent bekommt nur einen Verweis wie {ref}. Den echten Key setzt {name} in die Anfrage ein, und nur, wenn sie an api.openai.com geht. |
| `web.agent.p1` / `p2` / `p3` | from `i18n.json` `x.agent.p1`–`p3` | from `i18n.json` |
| `web.tools.eyebrow`, `web.tools.title` | from `i18n.json` | from `i18n.json` |
| `web.tools.intro` | Logins fill in Chrome, Edge and Firefox, in Windows programs, inside iPhone and Android apps, and from the command line. | Logins füllen sich in Chrome, Edge und Firefox, in Windows-Programmen, in iPhone- und Android-Apps und auf der Kommandozeile. |
| `web.tools.ext` / `desktop` / `mobile` / `cli` | from `i18n.json` | from `i18n.json` |
| `web.tools.programs` | Windows programs | Windows-Programme |
| `web.tools.import` | Import from your browser, 1Password, Bitwarden, LastPass and more | Import aus deinem Browser, 1Password, Bitwarden, LastPass und mehr |
| `web.switch.bring.text` (changed value) | Chrome, Edge, Firefox, Safari, 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass, Enpass, or any spreadsheet export. | Chrome, Edge, Firefox, Safari, 1Password, Bitwarden, LastPass, Dashlane, Keeper, NordPass, Proton Pass, RoboForm, KeePass, Enpass oder jeder Tabellen-Export. |

The agent reference value is the fixed literal `mosaic://work/openai/api-key` (format `mosaic://<vault>/<item>/<field>`, agent-access spec).

- [ ] **Step 1: Write the failing tests** (`pages.test.ts`)

```ts
it.each(["en", "de"] as const)("home %s tells the prototype story in order", (locale) => {
  const html = pages.find((p) => p.page === "home" && p.locale === locale)!.html;
  const order = ['class="hero hero--home"', "data-showcase", 'id="reasons-title"', 'id="where-title"', 'id="features-title"', 'id="agent-title"', 'id="tools-title"', 'id="pricing-title"', 'id="get-title"'];
  const at = order.map((s) => html.indexOf(s)); expect(at.every((v, i) => v >= 0 && (i === 0 || v > at[i - 1]!))).toBe(true);
  expect(html).toContain("<code>mosaic://work/openai/api-key</code>");
  expect(html.match(/fetchpriority="high"/g)).toHaveLength(1);
  expect(html.match(/class="button button--lg holo"/g)?.length).toBeLessThanOrEqual(2);  // hero + final CTA, never in one viewport
});
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** pages + catalog tests, lint, typecheck, build → PASS; compare with the reference at 1440 and 390 px.
- [ ] **Step 5: Commit** `feat(web): home page with agent access and tools sections`

---

### Task 14: Features, Security, Download and Next per the prototype

**Files:**
- Modify: `views/features.tsx`, `views/security.tsx`, `views/download.tsx`, `catalog/{en,de}/web/launch.ts`, `pages.test.ts`
- Create: `apps/web/src/styles/{security.css,download.css}` (proto 2567–2756, 2864–3116; terminal 2432–2532 stays in `demos.css`)

**Interfaces:**
- Consumes: Tasks 11–12 components and demos (`CliDemo`, `SyncDemo`, `CipherMock`, `FillMock`, `WarnMock`).
- Produces:
  - Features = prototype 4395–4443 plus one ruled row in the devices section with two new `Point`s: programs and Android linking, and one new tile in "more": own cloud folders.
  - Security = prototype 4445–4479 (how-steps list replaces the sticky stage; third step visual `SyncDemo`), plus the challenge block from Task 10 before the page CTA.
  - Download = prototype 4481–4505: hero holo `jump` to `#platforms` (`href="#platforms"`, section `id="platforms" tabIndex={-1}`), platform groups (existing `site.files`/`site.stores` and the `downloads` island keep deciding links and "Coming soon"), signed + next cards.
  - Next = prototype 4507–4524 (`ol.steps[data-island="demos"][data-demo="nextSteps"]`, the SVG line styled by class, no `stroke="var(…)"` attribute).
- Copy (EN / DE), exact:

| key | EN | DE |
|---|---|---|
| `web.features.programs.title` | Logins in Windows programs, too. | Logins auch in Windows-Programmen. |
| `web.features.programs.text` | When a Windows program asks you to sign in, {name} offers the matching login right at the field. One pick fills it. | Wenn ein Windows-Programm nach deinem Login fragt, bietet {name} den passenden direkt am Feld an. Eine Auswahl füllt ihn aus. |
| `web.features.androidLink.title` | One tap links an app. | Ein Tipp verknüpft eine App. |
| `web.features.androidLink.text` | On Android, {name} suggests the right login for an app it hasn’t seen yet. One tap and a fingerprint or face check link it and fill. | Auf Android schlägt {name} für eine neue App den passenden Login vor. Ein Tipp und dein Fingerabdruck oder Gesicht verknüpfen ihn und füllen aus. |
| `web.features.folders.title` | Your own cloud folder, on your phone too. | Dein eigener Cloud-Ordner, auch auf dem Handy. |
| `web.features.folders.text` | Sync through Dropbox, OneDrive or Nextcloud on your computer and on Android. Your files, your account, the same lock. | Synchronisiere über Dropbox, OneDrive oder Nextcloud, am Computer und auf Android. Deine Dateien, dein Konto, dasselbe Schloss. |
| `web.security.how` | How it works | So funktioniert es |

- [ ] **Step 1: Write the failing tests** (`pages.test.ts`)

```ts
it("download jumps to the platforms without JS", () => {
  const html = pages.find((p) => p.page === "download" && p.locale === "en")!.html;
  expect(html).toMatch(/<a [^>]*href="#platforms"[^>]*data-jump=""/); expect(html).toMatch(/id="platforms" tabindex="-1"/);
});
it("security explains in steps and points to the challenge", () => {
  const html = pages.find((p) => p.page === "security" && p.locale === "de")!.html;
  expect(html.match(/class="how-step"/g)).toHaveLength(3); expect(html).toContain('href="/de/sicherheit/challenge/"');
  expect(html).not.toContain('data-island="steps"');
});
it("next steps animate from a finished list", () => expect(pages.find((p) => p.page === "next" && p.locale === "en")!.html).toMatch(/data-demo="nextSteps"/));
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement.** **Step 4: Run** tests, lint, typecheck, build → PASS; side-by-side check with the reference.
- [ ] **Step 5: Commit** `feat(web): features, security, download and next steps per the prototype`

---

### Task 15: Every other page in the new visual system

**Files:**
- Modify: `views/compare.tsx`, `views/plans.tsx` (pricing, teams), `views/switch.tsx`, `views/check.tsx`, `views/faq.tsx`, `views/download.tsx` (Verify), `views/small.tsx` (thanks, confirmed, legal, 404), `views/help.tsx`, `styles/pages.css`, `styles/check.css`
- Delete: CSS rules no page uses (check with the built HTML class list)

**Interfaces:**
- Consumes: `PageHero`, `PageCta`, `Tile`, `Point`, `Checks`, `ruled`/`bento`/`split`/`timeline` grids, `card`, `see__card`, frames (Task 11).
- Produces, page by page (content and copy unchanged unless named):
  - Compare hub and vendor pages: `PageHero` (crumbs on vendors), the matrix as a `card` with 1 px rules, sticky first column on ≥ 900 px, horizontally scrollable inside its own wrapper below (never the page); "ahead" points as `Point`s; `PageCta` with switch secondary.
  - Pricing: `PageHero`, `PlanCards full`, FAQ subset as `Questions`; Teams: `PageHero` with `MembersMock` visual, storage options as `ruled--3`, contact `card`, capture card unchanged.
  - Switch: `PageHero`, sources as `bento` tiles, steps as `timeline`, checklist `card` (print island and `@media print` stay), capture card.
  - Self-check: `PageHero` and the existing check UI inside a `card`; radio inputs keep 24 px targets and visible focus.
  - FAQ: `PageHero`, groups as `split` with `Questions`; Verify: `PageHero` with crumbs, command blocks in `.terminal` style (static, no demo).
  - Thanks, Confirmed, Legal, 404, Report, Challenge: `hero--plain` with the field still behind; legal text measure ≤ 64 ch.
- Budgets after this task: CSS ≤ 30 000 B, every page HTML ≤ 40 000 B.

- [ ] **Step 1: Write the failing test** (`pages.test.ts`)

```ts
it.each(pages.map((p) => [p.file, p] as const))("%s uses the shared hero", (_f, p) => {
  expect(p.html).toMatch(/<section class="hero hero--(home|page)/);
  expect(p.html).not.toMatch(/class="[^"]*\b(stage|p[1-9]-|foil)\b/);   // retired v2 patterns
});
```

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**; remove unused CSS.
- [ ] **Step 4: Run** pages test, lint, typecheck, build (all budgets) → PASS. Click through every page in EN and DE at 1440 and 390 px in `vite preview`.
- [ ] **Step 5: Commit** `feat(web): compare, pricing, teams, switch, check, faq and small pages restyled`

---

### Task 16: Launch-gate claims registry

**Files:**
- Create: `apps/web/src/pages/claims.ts`, `apps/web/src/pages/claims.test.ts`
- Modify: `apps/web/scripts/prerender.ts` (`buildStart`: with `MOSAIC_DEPLOY=production`, throw when `unverifiedClaims()` is not empty, listing ids)

**Interfaces:**
- Produces: `interface Claim { id: string; keys: readonly WebKey[]; check: string; verified: boolean }`; `CLAIMS: readonly Claim[]`; `unverifiedClaims(): Claim[]`; `CLAIM_TERMS: RegExp` = `/agent|MCP|API-Key|API key|LastPass|Dashlane|Keeper|NordPass|Proton Pass|RoboForm|Enpass|Safari|Windows program|Windows-Programm|Android|iPhone|Face ID|Dropbox|OneDrive|Nextcloud|iCloud|passkey|Passkey|SSH|command line|Kommandozeile|challenge|Challenge|test server|Testserver/`.
- The registry (all `verified: false`; the owner flips each one after checking the release):

| id | keys (prefixes) | check |
|---|---|---|
| G1 agent | `web.agent.`, `web.agent.t.` | `mosaic agent mcp` is released; references `mosaic://<vault>/<item>/<field>`; requests to hosts outside the key's list are refused; responses are masked; the client named in `web.agent.t.session` is a tested client |
| G2 import | `web.tools.import`, `web.tools.import.`, `web.switch.bring.text`, `web.pricing.free.item4` | every source named has a parser and a guide in the released desktop app (onboarding-import plan); the cleanup step offers to delete the export |
| G3 programs | `web.tools.intro`, `web.tools.programs`, `web.features.programs.` | Windows program fill phase 2 ships enabled in the Windows release |
| G4 browsers | `web.tools.ext`, `web.faq.*` lines naming browsers, download browser tiles | Chrome, Edge and Firefox extensions are published or installable as the download page says |
| G5 mobile | `web.features.phone.`, `web.tools.mobile`, `web.tools.phone.`, `web.cmp.mobile.mosaic`, `web.cmp.passkeys.mosaic`, `web.hero.note`, `web.pricing.free.item1` | iPhone and Android apps are in their stores (`site.stores`), fill logins and passkeys in apps; Face ID unlock exists on iPhone |
| G6 android link | `web.features.androidLink.` | Android one-tap linking ships with a biometric confirmation |
| G7 folders | `web.features.folders.`, `web.only.cloud.text`, `web.pricing.free.item5`, `web.teamsPage.storage.company`, landing folder sentences | each named provider works on each platform the sentence names (desktop: any synced folder incl. iCloud Drive; Android: Dropbox, OneDrive, Nextcloud) |
| G8 cli | CLI demo literals, `web.features.terminal.`, `web.tools.cli` | `mosaic totp`, `mosaic run --env`, `mosaic ssh list` exist with that syntax in the released CLI (`docs/CLI.md`) |
| G9 passkeys/ssh | `web.features.passkeys.`, `web.features.ssh.`, `web.cmp.ssh.mosaic` | passkey sync across devices and the SSH agent's per-use prompt are in the release |
| G10 challenge | `web.challenge.`, `web.security.challenge.`, `web.report.` | the repository is public; `SECURITY.md`, `docs/BUG_BOUNTY.md`, `docs/CHALLENGE.md`, `docs/HALL_OF_FAME.md` are on `main`; private vulnerability reporting is on; response times match ADR-0087; `site.challengeOpen` matches the test server; safe-harbor wording matches `BUG_BOUNTY.md` |

- [ ] **Step 1: Write the failing tests**

```ts
it("names only keys that exist", () => { for (const c of CLAIMS) for (const k of c.keys) expect(Object.keys(catalogs.en).some((x) => x === k || x.startsWith(k)), `${c.id}: ${k}`).toBe(true); });
it.each(locales)("%s: every claim-like web string is gated", (L) => {
  const gated = (k: string) => CLAIMS.some((c) => c.keys.some((p) => k === p || k.startsWith(p)));
  const loose = Object.entries(catalogs[L]).filter(([k, v]) => k.startsWith("web.") && !/^web\.(cmp|compare|vs1password|vsBitwarden|where)\./.test(k) && CLAIM_TERMS.test(v) && !gated(k)).map(([k]) => k);
  expect(loose).toEqual([]);
});
it("blocks production while a claim is open", () => expect(unverifiedClaims().map((c) => c.id)).toContain("G1"));
```

(The comparison and "where" keys are sourced third-party facts under the existing `n/v` guard; the Mosaic cells `web.cmp.*.mosaic` are listed in G5/G9 explicitly.)

- [ ] **Step 2: Run** → FAIL. **Step 3: Implement**; widen a claim's key list rather than the exclusion regex when the second test lists a key.
- [ ] **Step 4: Run** → PASS; `MOSAIC_DEPLOY=production pnpm --filter @mosaic/web build` fails naming G1–G10 (and the existing legal check).
- [ ] **Step 5: Commit** `feat(web): launch-gate registry for claims about features in flight`

---

### Task 17: Playwright: shaders, pages, fallbacks, transitions, runtime budgets; CI job

**Files:**
- Create: `apps/web/playwright.config.ts`, `apps/web/e2e/{fixtures.ts,shaders.spec.ts,pages.spec.ts,fallbacks.spec.ts,transitions.spec.ts,perf.spec.ts}`
- Modify: `apps/web/package.json` (devDependencies `@playwright/test`, `@axe-core/playwright`, `jsdom` at the exact versions `apps/desktop/package.json` pins: no new package for the workspace; script `"test:e2e": "playwright test"`), `apps/web/vitest.config.ts` (exclude `e2e/**`), `eslint.config.js` only if the web e2e folder needs the existing e2e override, `.gitignore` (`apps/web/test-results/`, `apps/web/playwright-report/`), `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `FIELD_VERT`, `FIELD_FRAG` (Task 7), `VERT`, `FRAG` (`src/fx/shader.ts`), `ROUTES` (`src/pages/routes.ts`), `dist/.vite/manifest.json`.
- Produces:
  - `playwright.config.ts`: `testDir: "e2e"`, `workers: 1`, `webServer: { command: "pnpm exec vite preview --port 5181 --strictPort", url: "http://localhost:5181/", reuseExistingServer: false }` (needs a prior `pnpm --filter @mosaic/web build`), one Chromium project with `launchOptions.args: ["--use-angle=swiftshader", "--enable-unsafe-swiftshader", "--ignore-gpu-blocklist"]`.
  - `fixtures.ts`: `allPaths(): string[]` (every route in both languages + `/404.html`); `forceField(page)` (init script that strips `failIfMajorPerformanceCaveat` from `getContext("webgl2", …)` attributes, so SwiftShader counts as capable); `noWebGl2(page)`; `saveData(page)` (defines `navigator.connection = { saveData: true, effectiveType: "4g" }`); `collectViolations(page)` (listens to `securitypolicyviolation` via init script and to console errors); `chunkFile(src: string): string` (from the manifest).
- Specs and their assertions:
  - `shaders.spec.ts`: WebGL2 must exist (fail, never skip); compile and link `FIELD_VERT/FIELD_FRAG` and `VERT/FRAG`; `COMPILE_STATUS` and `LINK_STATUS` true and the info logs empty.
  - `pages.spec.ts` (every path): status 200 (404 page: 404); no CSP or Trusted Types violation, no console error; every request's origin is `http://localhost:5181`; axe with tags `wcag2a, wcag2aa, wcag21a, wcag21aa, wcag22aa` has no violations; at 320×640 `document.documentElement.scrollWidth <= clientWidth` (Review Focus 4); with JS disabled every `[data-reveal]` has computed opacity `"1"` (Review Focus 1).
  - `fallbacks.spec.ts`: `emulateMedia({ reducedMotion: "reduce" })` → no request for the field chunk, no `canvas.field`, every `[data-demo]` already in its final frame, `.field-still` visible; `saveData` → no field chunk, no holo chunk; `noWebGl2` → no `canvas.field`, `html[data-fx="none"]`; motion toggle click → `html[data-motion="off"]`, `html[data-fx-state="still"]` within 1 s on a forced field, and the choice survives a reload; forced field + `WEBGL_lose_context().loseContext()` → `.field-still` visible, no page error (Review Focus 5).
  - `transitions.spec.ts` (forced field, desktop): pointer click on the header "Features" link → `.veil` visible during, URL `/features/` within 1 500 ms; `Enter` on a focused nav link → navigation without `.veil.is-on`; Ctrl+click → a new page opens, current URL unchanged; after a wipe, `page.goBack()` → `.veil` hidden (Review Focus 2); download page: click the hero holo → `scrollY` grows, a `wheel` stops the glide; hovering a nav link triggers a request with header `sec-purpose: prefetch` (if this fails because of the CSP, remove the Speculation-Rules header in `security.ts` and note it; never loosen the CSP).
  - `perf.spec.ts`: for `/`, `/features/`, `/download/` (EN), mobile and desktop profiles from the Global Constraints via CDP (`Emulation.setCPUThrottlingRate`, `Network.emulateNetworkConditions`), median of 3 cold runs: LCP, CLS to 6 s, TBT from `longtask` entries, all within budget; every lazy chunk request (`chunkFile("src/field/bootField.ts")`, holo, demos, micro) starts after the final LCP `startTime`; forced field on mobile: TBT over 5 s after `html[data-fx-state]` appears ≤ 200 ms.
- CI job `web` in `.github/workflows/ci.yml`, named "Website (budgets, shaders, CSP, axe, perf)", `ubuntu-latest`, steps mirroring the extension job: checkout, pnpm setup, `pnpm install --frozen-lockfile`, `pnpm --filter @mosaic/web lint`, `typecheck`, `test`, `build`, `pnpm --filter @mosaic/web exec playwright install --with-deps chromium`, `pnpm --filter @mosaic/web exec playwright test`; uses only actions already on the allowlist.

- [ ] **Step 1: Write the specs** above.
- [ ] **Step 2: Run** `pnpm --filter @mosaic/web build && pnpm --filter @mosaic/web exec playwright test` → expected FAIL first on whatever is not yet right (if everything passes on the first run, break one budget on purpose locally to see it fail, then restore).
- [ ] **Step 3: Fix** the causes in the owning modules (never by loosening a budget or a test). Typical: LCP image priority, a CLS from the decode or the field fade, a long task from shader linking (use the parallel-compile path).
- [ ] **Step 4: Run** the suite twice in a row → PASS both times (no flakes).
- [ ] **Step 5: Commit** `test(web): Playwright shader compile, CSP, axe, fallbacks, transitions and runtime budgets; CI job` (body: no new packages, versions follow apps/desktop). Note for the owner: add "Website (budgets, shaders, CSP, axe, perf)" to the required checks of `main`.

---

### Task 18: Final gate, review, docs (opus)

**Files:**
- Modify: `docs/DECISIONS.md` (one ADR, next free number after checking `docs/DECISIONS.md` and the numbers reserved in `docs/superpowers/plans/*.md`), `docs/PROGRESS.md`, `apps/web/README.md` if present (commands, budgets, claims gate)

- [ ] **Step 1: ADR** "Website runtime: static pages, tiered effects, view transitions": the seven decisions of the Global Constraints with alternatives (Lighthouse CI, GSAP self-hosted, prerender speculation, Plex subsetting, hashes instead of external scripts) and consequences (claims gate blocks production; required check to add).
- [ ] **Step 2: Full gate** in the web worktree, once: `FULL_RUN_OK=1` prefix, the CLAUDE.md command list, plus `pnpm --filter @mosaic/web build` and `pnpm --filter @mosaic/web exec playwright test`. Expected: all pass.
- [ ] **Step 3: Review** with the `ux-reviewer` agent once (light/dark is dark-only here; DE and EN; WCAG 2.2 AA; motion toggle; demos' `aria-label`s). Fix findings with a sonnet agent; an opus agent verifies the fixes.
- [ ] **Step 4: PROGRESS** entry: what shipped, the budgets with the measured values from the last build and Playwright run, the open claims G1–G10, the owner items (required check, legal texts, domain, `site.challengeOpen`).
- [ ] **Step 5: Commit** `docs(web): website runtime ADR and progress`, push `feat/web`, open the PR (body ends with the Claude Code line), then `worktree.sh prune` after the merge.
