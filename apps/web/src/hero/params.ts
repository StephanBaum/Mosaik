/**
 * Tunable values of the painted hero's pixel sorting, tuned live behind `?tune=1` in dev.
 * Lengths are CSS px; positions are shares of the hero (0 = top or left, 1 = bottom or right).
 */
export interface SortParams {
  sort: {
    /** Luminance a pixel must exceed to join a sorted run (lower = more streaks). */
    threshold: number;
    /** Longest run, in CSS px. */
    length: number;
    /** Opacity of the sorted runs over the painting. */
    strength: number;
  };
  band: {
    /** Vertical centre of the flowing band. */
    position: number;
    /** Half-height of the band. */
    width: number;
    /** Strength in the sky (above the painted horizon). */
    sky: number;
    /** Strength toward the right edge. */
    edge: number;
  };
  wind: {
    /** Drift of the runs, CSS px per second, left to right. */
    drift: number;
  };
  cursor: {
    /** Radius of the clean circle around the pointer. */
    radius: number;
    /** Follow rate (1/s): lower is more inertia. */
    inertia: number;
  };
  scroll: {
    /** Extra sorting when the hero has scrolled out (1 = twice as strong). */
    gain: number;
  };
  copy: {
    /** How much lighter the sorting is behind the headline (1 = none there). */
    calm: number;
    /** Strength of the dark gradient behind the copy. */
    gradient: number;
  };
}

export function defaultParams(): SortParams {
  return {
    sort: { threshold: 0.2, length: 360, strength: 0.95 },
    band: { position: 0.2, width: 0.14, sky: 0.85, edge: 0.75 },
    wind: { drift: 14 },
    cursor: { radius: 210, inertia: 5 },
    scroll: { gain: 1.4 },
    copy: { calm: 0.85, gradient: 0.85 },
  };
}
