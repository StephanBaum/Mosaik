/**
 * Constants and timing functions of the reveal, ported from the study (reveal-studies.html
 * section 5). Every frame is a pure function of the run time `u` and the stage geometry.
 * GLASS holds the owner's tuned values (ADR-0032): do not change them here.
 */
import type { Lens } from "./lens.ts";

export const GLASS = {
  duration: 1.9,
  push: 3.8,
  bevel: 0.57,
  bevelMax: 110,
  height: 50,
  magnify: 0,
  refraction: 6.8,
  dispersion: 0.235,
  film: 0.15,
  shape: 0.1,
  wobble: 4,
  lead: 130,
} as const;

/** Reduced motion: a crossfade of this length inside a run of REDUCED_TOTAL_S. */
export const REDUCED_FADE_S = 0.25;
export const REDUCED_TOTAL_S = 0.3;
export const MAX_DPR = 2;
/** The longest time step one frame may advance the run by. */
export const MAX_STEP_S = 0.05;
export const BAND_PIXEL_BUDGET = 2.1e6;
export const SEALED_PIXEL_BUDGET = 8.3e6;
export const FIRST_U_STEP = 0.004;
/** Three frames in a row slower than this drop the glass band for the plain lens. */
export const SLOW_FRAME_MS = 50;
export const SLOW_FRAMES = 3;
export const MUTATION_REBUILD_MS = 150;
/** How long a sealed vault waits for the locked session before it is put back. */
export const SEAL_TIMEOUT_MS = 4000;

export function clamp(x: number, a = 0, b = 1): number {
  return Math.min(b, Math.max(a, x));
}

export function easeOutCubic(x: number): number {
  return 1 - Math.pow(1 - clamp(x), 3);
}

export function smooth(a: number, b: number, x: number): number {
  const t = clamp((x - a) / (b - a));
  return t * t * (3 - 2 * t);
}

/** Radius at which even the band's inner edge has left the stage. */
export function rFull(maxD: number): number {
  return (maxD + GLASS.bevelMax + GLASS.wobble + 24) / Math.max(0.5, 1 - GLASS.shape);
}

export function growT(): number {
  return 0.9 * GLASS.duration;
}

export function unlockR(u: number, maxD: number): number {
  return rFull(maxD) * (1 - Math.pow(1 - clamp(u / growT()), GLASS.push));
}

export function lockDur(): number {
  return 0.75 * GLASS.duration;
}

export function lockR(u: number, maxD: number): number {
  return (rFull(maxD) + 60) * (1 - easeOutCubic(u / lockDur())) - 60;
}

export function bevelWidth(R: number): number {
  return Math.min(Math.max(R * GLASS.bevel, 10), GLASS.bevelMax);
}

export function lensAt(dir: "unlock" | "lock", u: number, maxD: number): Lens {
  if (dir === "unlock") {
    const R = unlockR(u, maxD);
    return {
      R,
      t: u,
      shape: GLASS.shape,
      wob: GLASS.wobble * Math.sin(Math.PI * clamp(u / growT())) * clamp(R / 60),
      H: GLASS.height * easeOutCubic(u / ((0.3 * GLASS.duration) / 1.6)),
      pres: smooth(0, 24, R),
    };
  }
  const R = lockR(u, maxD);
  return {
    R,
    t: u,
    shape: GLASS.shape,
    wob: GLASS.wobble * clamp(R / 60),
    H: GLASS.height * smooth(0, 90, R),
    pres: smooth(0, 24, R),
  };
}

/** The first run time in [0, end] where `pred` holds, searched in FIRST_U_STEP steps (study L763). */
export function firstU(pred: (u: number) => boolean, end: number): number {
  for (let u = 0; u <= end; u += FIRST_U_STEP) {
    if (pred(u)) {
      return u;
    }
  }
  return end;
}
