/**
 * Bakes stills of the pixel sort, dev server only (`?bake=1`; main.ts imports this module under
 * `import.meta.env.DEV`). The shipped stills in public/img were made with it:
 * `hero-barn-sort-*` is the band overlay of the static fallback (transparent where the painting
 * is clean), `detail-*` and `sky-*` are crops of the composite. A developer tool, not page code.
 */
import { defaultParams } from "./params.ts";
import { createSortRenderer } from "./renderer.ts";

export interface BakeOptions {
  src: string;
  /** Multiplies the sort strength field (1 = the live hero at rest). */
  gain: number;
  /** The copy's box as image uv, kept calm. */
  text: readonly [number, number, number, number];
  /** Width the lengths are tuned for (the live hero at 1440 CSS px). */
  reference: number;
  time: number;
}

export async function bake(options: BakeOptions): Promise<{ overlay: string; composite: string }> {
  const img = new Image();
  img.src = options.src;
  await img.decode();
  const canvas = document.createElement("canvas");
  const renderer = await createSortRenderer(canvas, img, { preserveDrawingBuffer: true });
  if (renderer === null) {
    throw new Error("bake: no WebGL2");
  }
  renderer.resize(img.naturalWidth, img.naturalHeight);
  const p = defaultParams();
  const k = img.naturalWidth / options.reference;
  renderer.draw({
    cover: [1, 1, 0, 0],
    time: options.time,
    sort: [p.sort.threshold, p.sort.length * k, 0, p.sort.strength],
    band: [p.band.position, p.band.width, p.band.sky, p.band.edge],
    text: options.text,
    calm: [p.copy.calm, 0.14],
    cursor: [0, 0, 1, 0],
    gain: options.gain,
  });
  const overlay = canvas.toDataURL("image/png");
  const flat = document.createElement("canvas");
  flat.width = img.naturalWidth;
  flat.height = img.naturalHeight;
  const ctx = flat.getContext("2d");
  if (ctx === null) {
    throw new Error("bake: no 2D context");
  }
  ctx.drawImage(img, 0, 0);
  ctx.drawImage(canvas, 0, 0);
  return { overlay, composite: flat.toDataURL("image/png") };
}
