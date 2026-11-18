// Provenance: verbatim copy of apps/desktop/src/holo/effects.ts (design concept §5, owner decision 7).
// Never edit this copy; change the desktop source and copy it again. src/fx/parity.test.ts pins it.

/**
 * Press ripple and focus waves of the holo button (docs/design/holo/effect-studies.html, the
 * values the owner approved). Lengths are in button heights, times in seconds. Pure data and
 * maths; the shader draws the waves.
 */
export interface WaveTuning {
  speed: number;
  wavelength: number;
  runout: number;
  duration: number;
  outline: number;
  refraction: number;
  shading: number;
}

export const RIPPLE: WaveTuning = {
  speed: 4.7,
  wavelength: 1.3,
  runout: 0.7,
  duration: 1.4,
  outline: 0.05,
  refraction: 0.032,
  shading: 0.25,
};

export const FOCUS: WaveTuning & { grow: number; gap: number } = {
  ...RIPPLE,
  duration: 1.8,
  grow: 0.03,
  gap: 0.5,
};

const RIPPLE_FALLOFF = 1.6;
const FOCUS_FALLOFF = 1.4;

/** Age that means "no effect running". */
export const EFFECT_IDLE = 99;

/** CSS px the foil canvas extends past the button on every side so the outline can deform. */
export const FOIL_MARGIN = 8;

function weight(age: number, duration: number, falloff: number): number {
  return age < duration ? Math.pow(1 - age / duration, falloff) : 0;
}

export function rippleWeight(age: number): number {
  return weight(age, RIPPLE.duration, RIPPLE_FALLOFF);
}

export function focusWeight(age: number): number {
  return weight(age, FOCUS.duration, FOCUS_FALLOFF);
}

export function rippleRunning(age: number): boolean {
  return age < RIPPLE.duration;
}

export function focusRunning(age: number): boolean {
  return age < FOCUS.duration;
}
