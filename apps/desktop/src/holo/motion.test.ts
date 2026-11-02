import { describe, expect, it } from "vitest";
import { approach, idleTarget, newState, update } from "./motion.ts";

const CALM = { reduced: false, slow: false };

describe("approach", () => {
  it("returns the target under reduced motion", () => {
    expect(approach(0, 1, { rate: 9, dt: 0.016, reduced: true })).toBe(1);
  });

  it("converges monotonically without overshoot", () => {
    let value = 0;
    for (let step = 0; step < 50; step += 1) {
      const next = approach(value, 1, { rate: 9, dt: 0.016, reduced: false });
      expect(next).toBeGreaterThan(value);
      expect(next).toBeLessThanOrEqual(1);
      value = next;
    }
    expect(value).toBeGreaterThan(0.99);
  });
});

describe("idleTarget", () => {
  it("follows the prototype formula", () => {
    const point = idleTarget({ time: 2, seed: 3, bx: 1.5 });
    expect(point.x).toBeCloseTo(Math.sin(2 * 0.7 + 3) * 1.5 * 0.55, 10);
    expect(point.y).toBeCloseTo(0.3 + 0.12 * Math.sin(2 * 0.98 + 3 * 2), 10);
  });
});

describe("update", () => {
  it("is active while the pointer is inside", () => {
    const state = newState();
    state.inside = true;
    state.hoverT = 1;
    update(state, 0.016, 1000, CALM);
    expect(state.active).toBe(true);
    expect(state.time).toBeGreaterThan(2);
  });

  it("settles once idle without ambient after the leave window", () => {
    const state = newState();
    state.leaveAt = 0;
    update(state, 0.016, 500, CALM);
    expect(state.active).toBe(true);
    for (let step = 0; step < 400; step += 1) {
      update(state, 0.016, 2000 + step * 16, CALM);
    }
    expect(state.active).toBe(false);
  });

  it("keeps animating with ambient, unless slow or reduced", () => {
    const state = newState({ ambient: true });
    update(state, 0.016, 100_000, CALM);
    expect(state.active).toBe(true);
    update(state, 0.016, 100_016, { reduced: false, slow: true });
    expect(state.active).toBe(false);
    update(state, 0.016, 100_032, { reduced: true, slow: false });
    expect(state.active).toBe(false);
  });
});
