import { describe, expect, it, vi } from "vitest";
import { linear } from "./ease.ts";
import { createTicker } from "./ticker.ts";
import { manualTicker } from "./testTicker.ts";
import { tween } from "./tween.ts";

describe("tween", () => {
  it("resolves false on cancel", async () => {
    const h = tween({ from: 0, to: 1, duration: 1, onUpdate: () => undefined }, manualTicker());
    h.cancel();
    await expect(h.done).resolves.toBe(false);
  });

  it("honours the delay and resolves true at the end", async () => {
    const t = manualTicker();
    const seen: number[] = [];
    const h = tween({ from: 0, to: 10, duration: 1, delay: 0.5, ease: linear, onUpdate: (v) => seen.push(v) }, t);
    t.step(0.4);
    expect(seen).toEqual([]);
    t.step(0.6);
    expect(seen[0]).toBeCloseTo(5);
    t.step(1);
    await expect(h.done).resolves.toBe(true);
    expect(seen[seen.length - 1]).toBe(10);
  });
});

describe("createTicker", () => {
  it("runs only while subscribed and clamps dt", () => {
    const cbs: FrameRequestCallback[] = [];
    const raf = vi.fn((cb: FrameRequestCallback) => cbs.push(cb));
    const caf = vi.fn();
    const ticker = createTicker(raf, caf);
    const seen: number[] = [];
    const off = ticker.add((_n, dt) => seen.push(dt));
    expect(raf).toHaveBeenCalledTimes(1);
    cbs.shift()?.(1000);
    cbs.shift()?.(3000);
    expect(seen).toEqual([0, 0.05]);
    off();
    expect(caf).toHaveBeenCalledTimes(1);
  });
});
