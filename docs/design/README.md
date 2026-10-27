# Design

These are reference mockups for every planned Mosaic screen, covering the
desktop app, the browser extension and mobile. The live, commentable version
is the "Mosaic — App Screens" design canvas
(https://claude.ai/artifact/Lh2sUSdw6tt459GHVjAJZQ). The files here are
snapshots of it.

- `screens/*.dc.html` hold one screen each, as the canvas's source markup.
  Files starting with `D` are desktop screens (`Main.dc.html` is the
  vault), `E` are extension popups, and `M` are mobile screens.
- `screens/canvas.json` holds the canvas layout and the screen titles.

The snapshots are not standalone pages: every screen loads the canvas's
`support.js` and most reference its uploaded images as `/_blob/<hash>`.
Neither is in this repository, so the files render only inside the design
canvas; opened directly they show unstyled or broken markup. They are kept
for review diffs and as the source to refresh the canvas from.

The files are design references, not app code. The snapshots load DM
Sans and IBM Plex Mono from Google Fonts and the canvas's own assets, so
opening them contacts those servers. The apps must never do this. They
bundle the font files locally (ADR-0014). The apps take colors, type,
spacing and radii from `packages/design-tokens` and all strings from
`packages/i18n` (see `docs/UX_GUIDELINES.md`). The example data in the
mockups is illustrative, including the Secret Key
`M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2`.

To refresh a snapshot, copy the canvas's current files over `screens/`.

## Holo-foil primary action (ADR-0014)

`holo/` holds the following:

- `button-studies.html`: the shader exploration page ("Mosaic — Button
  Studies", https://claude.ai/artifact/SbnCPAM7ootkSnupMgZnCH). The chosen
  style is the v5 base with "angled rainbow stripes".
- `holo-button.js` and `holo-button.md`: a dependency-free WebGL2 runtime
  for that style, with its API. M3 ports it to TypeScript in the desktop
  app.
- `holo-stripes-tile*.png`: baked stills for static mockups only.
