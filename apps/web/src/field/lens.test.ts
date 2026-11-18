import { describe, expect, it } from "vitest";
import { createLens, lensRadius, stepLens } from "./lens.ts";

describe("lens", () => {
  it("clamps the lens radius", () => {
    expect(lensRadius(320, 568)).toBe(180);
    expect(lensRadius(3840, 2160)).toBe(360);
  });
  it("orbits inside the viewport after the pointer rests", () => {
    const l = createLens(1440, 900);
    l.lastPointer = 0;
    for (let i = 0; i < 3600; i++) {
      stepLens(l, 1 / 60, 3000 + i * 16.7, { w: 1440, h: 900 });
      expect(l.x).toBeGreaterThan(-200);
      expect(l.x).toBeLessThan(1640);
    }
    expect(l.wasIdle).toBe(true);
  });
  it("follows the pointer while it moves", () => {
    const l = createLens(1000, 800);
    l.tx = 100;
    l.ty = 100;
    l.lastPointer = 10_000;
    for (let i = 0; i < 120; i++) {
      stepLens(l, 1 / 60, 10_100, { w: 1000, h: 800 });
    }
    expect(Math.abs(l.x - 100)).toBeLessThan(1);
    expect(l.wasIdle).toBe(false);
  });
});
