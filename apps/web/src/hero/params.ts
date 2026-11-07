/**
 * Tunable values of the decrypt-mosaic hero art, tuned live behind `?tune=1` in dev. Lengths
 * are CSS px unless noted; the renderer scales them by the device pixel ratio.
 */
export interface ArtParams {
  block: {
    /** Settled block size of the encrypted mosaic. */
    base: number;
    /** Share of blocks split once more at random (the mosaic is a quadtree, not a grid). */
    split: number;
    /** How hard a block snaps to the reduced palette (0 = the image, 1 = flat palette). */
    snap: number;
    /** Per-block colour jitter before the palette snap: dithers the boundaries. */
    dither: number;
    /** 1 px bevel on each block edge. */
    bevel: number;
  };
  gradient: {
    /** Flow speed of the mesh gradient. */
    speed: number;
    /** Domain warp of the colour field (organic forms). */
    warp: number;
    /** Chroma lift in OKLab after blending (keeps it loud, never muddy). */
    chroma: number;
    /** Rotates the five loud palette colours around the field. */
    hueShift: number;
  };
  grain: {
    amount: number;
    /** Grain frames per second (film-like: low). */
    fps: number;
  };
  mask: {
    /** Radius of the cursor key. */
    radius: number;
    /** Follow rate of the key (1/s): lower is more inertia. */
    inertia: number;
    /** Soft edge as a share of the radius. */
    softness: number;
    /** Lens magnification at the edge. */
    lens: number;
    /** Width (px) and strength of the difference ring at the mask edge. */
    ring: number;
    ringAmount: number;
    /** Size of the ambient key that breathes when no pointer leads (and on touch), relative to the key. */
    breathe: number;
  };
  sort: {
    /** Pixel-sort streak length. */
    length: number;
    /** Luminance a run must start above (higher = fewer streaks). */
    threshold: number;
    /** Share of column groups that carry streaks. */
    density: number;
    strength: number;
  };
  glitch: {
    /** Share of blocks with channel split and difference slices at any moment. */
    amount: number;
    /** Glitch re-rolls per second. */
    rate: number;
  };
  decode: {
    /** Seconds for 64 → 32 → 16 → 8 → clear. */
    duration: number;
    /** Seconds the clear image holds before it collapses into the key. */
    hold: number;
    collapse: number;
  };
}

export function defaultParams(): ArtParams {
  return {
    block: { base: 30, split: 0.22, snap: 0.82, dither: 0.07, bevel: 0 },
    gradient: { speed: 0.6, warp: 0.9, chroma: 1.3, hueShift: 0 },
    grain: { amount: 0.075, fps: 14 },
    mask: { radius: 190, inertia: 6, softness: 0.22, lens: 0.14, ring: 1.4, ringAmount: 0.85, breathe: 0.8 },
    sort: { length: 300, threshold: 0.4, density: 0.42, strength: 0.9 },
    glitch: { amount: 0.35, rate: 7 },
    decode: { duration: 1.6, hold: 0.25, collapse: 1.1 },
  };
}
