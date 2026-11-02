/**
 * Helpers for static foil surfaces (ADR-0014; the accents of a later work package). Pure
 * maths only here; painting goes through `drawFoil` on the renderer's context.
 */
import { screenPeriod } from "./draw.ts";
import { STRIPE_MULT } from "./shader.ts";

/** Direction of the stripes in the shader: `normalize(vec2(1.0, 0.7))`. */
const STRIPE_DIRECTION_X = 1 / Math.sqrt(1 + 0.49);

/**
 * The stripe frequency factor that makes `width` (device px) an integer number of stripe
 * periods along x, so a surface tiles horizontally (changes the frequency by under 0.5 %).
 */
export function tileStripeK(width: number, dpr: number): number {
  const xPeriod =
    (2 * Math.PI) / (((STRIPE_MULT * 2 * Math.PI) / screenPeriod(dpr)) * STRIPE_DIRECTION_X);
  const periods = Math.max(1, Math.round(width / xPeriod));
  return (periods * xPeriod) / width;
}
