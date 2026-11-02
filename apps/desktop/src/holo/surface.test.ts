import { describe, expect, it } from "vitest";
import { screenPeriod } from "./draw.ts";
import { STRIPE_MULT } from "./shader.ts";
import { tileStripeK } from "./surface.ts";

describe("tileStripeK", () => {
  it.each([320, 700, 1234])("makes %d px an integer number of stripe periods", (width) => {
    const k = tileStripeK(width, 2);
    const xPeriod = (2 * Math.PI) / (((STRIPE_MULT * 2 * Math.PI) / screenPeriod(2)) * (1 / Math.sqrt(1.49)));
    const periods = (width / xPeriod) * k;
    expect(Math.abs(periods - Math.round(periods))).toBeLessThan(1e-9);
  });

  it("changes the frequency only slightly for wide surfaces", () => {
    expect(Math.abs(tileStripeK(1234, 2) - 1)).toBeLessThan(0.05);
  });
});
