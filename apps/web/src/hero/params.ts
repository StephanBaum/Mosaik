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
  keep: {
    /** Centre of the clean ellipse around the barn, in painting uv. */
    x: number;
    y: number;
    /** Its radii, in painting uv. */
    rx: number;
    ry: number;
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
    sort: { threshold: 0.21, length: 560, strength: 1 },
    band: { position: 0.17, width: 0.09, sky: 1, edge: 0.7 },
    wind: { drift: 30 },
    cursor: { radius: 210, inertia: 5 },
    scroll: { gain: 1.4 },
    keep: { x: 0.715, y: 0.44, rx: 0.1, ry: 0.09 },
    copy: { calm: 0.92, gradient: 0.85 },
  };
}
