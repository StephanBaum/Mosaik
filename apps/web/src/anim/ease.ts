/** GSAP-compatible easing curves (the names and formulas of the prototype's `power2.out` and friends). */
export type Ease = (t: number) => number;

export const linear: Ease = (t) => t;

const powerIn =
  (n: number): Ease =>
  (t) =>
    t ** n;
const powerOut =
  (n: number): Ease =>
  (t) =>
    1 - (1 - t) ** n;
const powerInOut =
  (n: number): Ease =>
  (t) =>
    t < 0.5 ? 0.5 * (2 * t) ** n : 1 - 0.5 * (2 * (1 - t)) ** n;

export const power1Out: Ease = powerOut(2);
export const power1InOut: Ease = powerInOut(2);
export const power2In: Ease = powerIn(3);
export const power2Out: Ease = powerOut(3);
export const power2InOut: Ease = powerInOut(3);
export const power3In: Ease = powerIn(4);
export const power3Out: Ease = powerOut(4);
export const power3InOut: Ease = powerInOut(4);

/** `back.out(overshoot)`: 1 + (s+1)(t-1)^3 + s(t-1)^2. */
export function backOut(overshoot = 1.70158): Ease {
  const s = overshoot;
  return (t) => 1 + (s + 1) * (t - 1) ** 3 + s * (t - 1) ** 2;
}

/** `elastic.out(amplitude, period)`. */
export function elasticOut(amplitude = 1, period = 0.3): Ease {
  const a = Math.max(amplitude, 1);
  const phase = (period / (2 * Math.PI)) * Math.asin(1 / a);
  return (t) => (t === 0 || t === 1 ? t : a * 2 ** (-10 * t) * Math.sin(((t - phase) * 2 * Math.PI) / period) + 1);
}
