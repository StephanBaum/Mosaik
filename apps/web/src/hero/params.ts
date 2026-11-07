/**
 * Tunable values of the living mosaic. The ripple starts from the owner's `RIPPLE` (the copied
 * holo/effects.ts); everything else is the hero's own look, tuned live behind `?tune=1` in dev
 * (design concept §3 M1, build step 2, checkpoint A). Lengths are in tile pitches unless noted.
 */
import { DEFAULT_SEED } from "../fx/shader.ts";
import { RIPPLE } from "../fx/effects.ts";

export interface MosaicParams {
  foil: {
    /** Iridescence floor of every tile (the desktop stills use 0.5). */
    vibrance: number;
    /** Saturation lift after the foil (1 = none). */
    lift: number;
    /** Base thin-film seed, its gradient across the mosaic and its per-tile jitter. */
    hueBase: number;
    hueSpread: number;
    hueJitter: number;
    /** Stripe frequency factor (1 = the app's stripes). */
    stripeScale: number;
    /** Foil clock rate (1 = the app's; the sheen crosses about every 12.5 s). */
    timeScale: number;
    /** Seconds of sheen delay per tile along the sheen direction. */
    sheenLag: number;
    /** Corner radius as a share of the tile (12 px on a 72 px tile). */
    radius: number;
  };
  glyph: { share: number; scale: number; etch: number };
  cursor: {
    /** Gaussian reach of the cursor light, in pitches. */
    sigma: number;
    hoverGain: number;
    /**
     * Largest light offset (tile units) the cursor gives a tile. Beyond about the idle
     * wander's range the foil's view tilt cycles each stripe twice and greys out.
     */
    tilt: number;
    /** Brightness the cursor light adds to the tiles under it (multiplicative, keeps the hue). */
    glow: number;
    /** Easing rates (1/s) while the pointer moves and after it leaves, as in holo/motion.ts. */
    followRate: number;
    settleRate: number;
    presenceRate: number;
  };
  ripple: {
    speed: number;
    wavelength: number;
    runout: number;
    duration: number;
    outline: number;
    refraction: number;
    shading: number;
  };
  intro: { stagger: number; duration: number };
  /** The reduced-motion and Save-Data still: foil clock and a fixed light position. */
  still: { clock: number; lightX: number; lightY: number; presence: number };
}

export function defaultParams(): MosaicParams {
  return {
    foil: {
      vibrance: 0.85,
      lift: 1.8,
      hueBase: DEFAULT_SEED,
      hueSpread: 1.6,
      hueJitter: 3,
      stripeScale: 2.4,
      timeScale: 1,
      sheenLag: 0.32,
      radius: 12 / 72,
    },
    glyph: { share: 0.2, scale: 0.44, etch: 0.74 },
    cursor: { sigma: 2.5, hoverGain: 1.8, glow: 0.28, tilt: 0.15, followRate: 24, settleRate: 7, presenceRate: 9 },
    ripple: {
      speed: RIPPLE.speed,
      wavelength: RIPPLE.wavelength,
      runout: RIPPLE.runout,
      duration: RIPPLE.duration,
      outline: RIPPLE.outline,
      refraction: RIPPLE.refraction,
      shading: RIPPLE.shading,
    },
    intro: { stagger: 0.018, duration: 0.4 },
    still: { clock: 7.4, lightX: 1.6, lightY: 1.4, presence: 0.85 },
  };
}

/** Falloff of the press ripple's weight (RIPPLE_FALLOFF in holo/effects.ts). */
export const RIPPLE_FALLOFF = 1.6;

export function rippleWeight(age: number, duration: number): number {
  return age < duration ? Math.pow(1 - age / duration, RIPPLE_FALLOFF) : 0;
}
