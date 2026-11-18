import { describe, expect, it } from "vitest";
import { backOut, elasticOut, linear, power1InOut, power1Out, power2In, power2InOut, power2Out, power3In, power3InOut, power3Out } from "./ease.ts";

describe("ease", () => {
  it.each([linear, power1Out, power1InOut, power2In, power2Out, power2InOut, power3In, power3Out, power3InOut, backOut(2), elasticOut(1, 0.42)])(
    "curve %# maps 0 to 0 and 1 to 1",
    (e) => {
      expect(e(0)).toBeCloseTo(0, 9);
      expect(e(1)).toBeCloseTo(1, 9);
    },
  );

  it("matches the GSAP curves", () => {
    expect(power2In(0.5)).toBeCloseTo(0.125);
    expect(power3Out(0.5)).toBeCloseTo(0.9375);
    expect(power2InOut(0.25)).toBeCloseTo(0.0625);
    expect(backOut(2)(0.5)).toBeCloseTo(1.125);
    const peak = Math.max(...Array.from({ length: 50 }, (_, i) => elasticOut(1, 0.42)(i / 50)));
    expect(peak).toBeGreaterThan(1.05);
  });
});
