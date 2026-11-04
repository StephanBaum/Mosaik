import { describe, expect, it } from "vitest";
import { GLASS, lensAt, lockDur, lockR, rFull, growT, unlockR } from "./timing.ts";

const MAX_D = 900;

describe("GLASS", () => {
  it("holds the owner's tuned values exactly", () => {
    expect(GLASS).toEqual({
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
    });
  });
});

describe("unlockR", () => {
  it("starts at zero, never shrinks, never overshoots and settles at rFull", () => {
    expect(unlockR(0, MAX_D)).toBe(0);
    let previous = 0;
    for (let u = 0; u <= 3; u += 0.01) {
      const R = unlockR(u, MAX_D);
      expect(R).toBeGreaterThanOrEqual(previous);
      expect(R).toBeLessThanOrEqual(rFull(MAX_D) + 1e-9);
      previous = R;
    }
    expect(unlockR(growT(), MAX_D)).toBeCloseTo(rFull(MAX_D), 9);
    expect(unlockR(growT() + 1, MAX_D)).toBe(rFull(MAX_D));
  });
});

describe("lockR", () => {
  it("starts at rFull, never grows and is closed (negative) by the end", () => {
    expect(lockR(0, MAX_D)).toBeCloseTo(rFull(MAX_D), 6);
    let previous = Infinity;
    for (let u = 0; u <= 3; u += 0.01) {
      const R = lockR(u, MAX_D);
      expect(R).toBeLessThanOrEqual(previous);
      previous = R;
    }
    expect(lockR(lockDur(), MAX_D)).toBeLessThanOrEqual(0);
    expect(lockR(lockDur(), MAX_D)).toBe(-60);
  });
});

describe("lensAt", () => {
  it("has no bead at the start of the unlock and none at the end of the lock", () => {
    expect(lensAt("unlock", 0, MAX_D).pres).toBe(0);
    expect(lensAt("lock", lockDur(), MAX_D).pres).toBe(0);
  });

  it("wobbles only while the lens has a size", () => {
    expect(lensAt("unlock", 0, MAX_D).wob).toBe(0);
    expect(lensAt("lock", lockDur(), MAX_D).wob).toBe(0);
  });
});
