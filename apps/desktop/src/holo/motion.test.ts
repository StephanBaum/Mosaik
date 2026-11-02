import { describe, expect, it } from "vitest";
import { focusRunning, focusWeight, rippleRunning, rippleWeight } from "./effects.ts";
import {
  approach,
  idleTarget,
  newState,
  startFocusWaves,
  startRipple,
  update,
} from "./motion.ts";

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

describe("press ripple and focus waves", () => {
  it("starts at the given point and ends after its duration", () => {
    const state = newState();
    startRipple(state, 0.8, -0.2, false);
    expect(state.ripAge).toBe(0);
    expect([state.ripX, state.ripY]).toEqual([0.8, -0.2]);
    update(state, 0.05, 0, CALM);
    expect(state.active).toBe(true);
    expect(state.fxRunning).toBe(true);
    for (let step = 0; step < 40; step += 1) {
      update(state, 0.05, 100_000 + step * 50, CALM);
    }
    expect(state.fxRunning).toBe(false);
    expect(rippleRunning(state.ripAge)).toBe(false);
  });

  it("flags a repaint when an effect finishes", () => {
    const state = newState();
    startRipple(state, 0, 0, false);
    update(state, 1.39, 100_000, CALM);
    state.dirty = false;
    update(state, 0.05, 100_050, CALM);
    expect(state.dirty).toBe(true);
  });

  it("runs the focus waves for 1.8 s", () => {
    const state = newState();
    startFocusWaves(state, false);
    update(state, 1.7, 100_000, CALM);
    expect(focusRunning(state.focusAge)).toBe(true);
    update(state, 0.2, 100_200, CALM);
    expect(focusRunning(state.focusAge)).toBe(false);
  });

  it("ignores both under reduced motion", () => {
    const state = newState();
    startRipple(state, 0, 0, true);
    startFocusWaves(state, true);
    update(state, 0.016, 0, { reduced: true, slow: false });
    expect(rippleRunning(state.ripAge)).toBe(false);
    expect(focusRunning(state.focusAge)).toBe(false);
    expect(state.fxRunning).toBe(false);
  });
});

describe("effect weights", () => {
  it("fall off to zero at the approved durations", () => {
    expect(rippleWeight(0)).toBe(1);
    expect(rippleWeight(0.7)).toBeCloseTo(0.5 ** 1.6, 10);
    expect(rippleWeight(1.4)).toBe(0);
    expect(focusWeight(1.8)).toBe(0);
  });
});
