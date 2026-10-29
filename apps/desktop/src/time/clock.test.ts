import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createClock } from "./clock.ts";

describe("createClock", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(10_250);
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("ticks on the boundary and keeps a stable snapshot between ticks", () => {
    const clock = createClock();
    const cb = vi.fn();
    const stop = clock.subscribe(cb);
    expect(clock.getSnapshot()).toBe(10_250);
    vi.advanceTimersByTime(749);
    expect(cb).not.toHaveBeenCalled();
    expect(clock.getSnapshot()).toBe(10_250);
    vi.advanceTimersByTime(1);
    expect(cb).toHaveBeenCalledTimes(1);
    expect(clock.getSnapshot()).toBe(11_000);
    vi.advanceTimersByTime(1000);
    expect(cb).toHaveBeenCalledTimes(2);
    stop();
  });

  it("stops its timers after the last unsubscribe", () => {
    const clock = createClock();
    const stop = clock.subscribe(() => undefined);
    vi.advanceTimersByTime(2000);
    stop();
    expect(vi.getTimerCount()).toBe(0);
  });
});
