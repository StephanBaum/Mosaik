# holo-button.js

Runtime for Mosaic's primary action button in the "angled rainbow stripes" holo foil style
(button studies page, v5 base, texture 10). One ES module, no dependencies, no build step.

## Usage

```html
<button type="button" data-holo>Unlock</button>
<button type="button" data-holo="dark" aria-label="Add item"><svg>...</svg></button>

<script type="module">
  import { mountHoloButtons } from "./holo-button.js";
  const holo = mountHoloButtons(document);   // upgrades every button[data-holo] under the root
  // later: holo.refresh();  holo.unmount();
</script>
```

The button keeps its own DOM label and icon, so accessibility works the same as for any button.
Icon-only buttons need an `aria-label`. The foil is painted underneath by one fixed canvas.

## API

`mountHoloButtons(root = document, options = {}) -> { refresh(), unmount() }`

- `root`: element or document to search for `button[data-holo]`. Buttons are collected once at mount
  time; call `unmount()` and mount again after adding or removing buttons.
- `options.zIndex` (default `1`): z-index of the fixed foil canvas. Buttons sit above it via
  `z-index: var(--holo-z, 2)`.
- `options.dprCap` (default `2`): device pixel ratio cap.
- `options.injectStyles` (default `true`): adds the small stylesheet below. Pass `false` to style the
  buttons yourself.
- `refresh()`: repaint everything, e.g. after a layout change the module cannot observe.
- `unmount()`: removes listeners, the canvas and the classes, and releases the WebGL context.

Button attributes:

| Attribute | Meaning |
|---|---|
| `data-holo` or `data-holo="light"` | button sits on a light card (default) |
| `data-holo="dark"` | button sits on a dark card: foil 6% darker, softer light rim |
| `data-holo-ambient` | sheen drifts continuously (use on one hero button at most) |
| `data-holo-seed="4.26"` | per-button variation of the sheen; default matches the study page |

`renderHoloSurface(width, height, opts) -> HTMLCanvasElement` renders the bare foil (no pill, rim or
label) at rest, at the page's DPR-2 stripe scale. `opts.dark`, `opts.tileable` (seamless horizontally,
stripe frequency adjusted by under 0.5%), `opts.halfAspect` and `opts.unitPx` (the reference button's
half-aspect and height in device px; defaults 1.36 and 176 = the 88 px hero pill at DPR 2).

`HOLO_SHADER` exports the vertex and fragment sources.

## Behaviour

- Rest: each button is painted once with the foil at rest and only repaints when its state changes,
  the page scrolls or the layout changes.
- Hover: the foil tilts toward the pointer and the sheen strengthens. Press: 6% darker and `scale(.975)`
  through the `holo-pressed` class. Keyboard focus (`:focus-visible`): a slight lift plus your own focus
  ring. Space and Enter press it as well.
- `prefers-reduced-motion: reduce`: still frames only; state changes repaint without animation.
- Slow-frame downgrade: if three frames in a row take more than 24 ms while something animates (ignored
  for 2.5 s after load or resize), it drops to 1x resolution and stops the ambient sheen.
- Fallback: without WebGL2, or after a lost context, buttons get the `holo-fallback` class: solid
  `#18181B` with `#FAFAFA` text on light cards, `#FAFAFA` with `#09090B` text on dark cards.
- Clipping: each button is clipped to its nearest ancestor with `overflow` other than `visible`.

## Visual constants

- Stripe period: 18.2 device px perpendicular to the stripes at DPR 2 or more, which is 9.1 CSS px
  (22.2 device px measured horizontally). At DPR 1 it is 13.6 CSS px.
- Label and icon: `#09090B`, with `text-shadow: 0 1px 0 rgba(255,255,255,.55)` (icons:
  `drop-shadow(0 1px 0 rgba(255,255,255,.55))`). Same on light and dark cards.
- Rim, 1 CSS px: on light cards `#D4D4D8` at full strength. On dark cards `#E6E6EB` at 75% over the
  foil, then the whole dark-card button is multiplied by 0.94 (effective rim about `#D8D8DD` blended 75%).
- Inner highlight: 1 CSS px white at 35% along the top inside edge.
- Shape: fully round pill (`border-radius: 999px`); a square button renders as a circle.

## Notes

- The canvas is `position: fixed` and draws above normal page content but below the buttons. An element
  that overlaps a holo button without its own z-index above the canvas will show the foil over it.
- Static buttons rest with the idle pointer position derived from the button's measured aspect. The
  study page used a default aspect on its very first paint of each button and the measured one from its
  next repaint on; the module matches the page after that repaint (within 1/255 per channel).
- The 512 x 128 tile repeats its soft sheen highlight every 512 px.
