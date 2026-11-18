import { describe, expect, it } from "vitest";
import { canvasSize } from "./size.ts";

describe("canvasSize", () => {
  it("caps 4K at 2x under the pixel ceiling", () => {
    const s = canvasSize(3840, 2160, 2, 1.5);
    expect(s.w * s.h).toBeLessThanOrEqual(2_400_000);
    expect(s.scale).toBeLessThan(1.5);
  });
  it("keeps a small canvas at the DPR cap", () => {
    expect(canvasSize(390, 844, 3, 1)).toEqual({ w: 390, h: 844, scale: 1 });
  });
  it("never goes below 2 pixels", () => {
    expect(canvasSize(0, 0, 1, 1)).toMatchObject({ w: 2, h: 2 });
  });
});
