import { describe, expect, it, vi } from "vitest";
import { linear } from "./ease.ts";
import { timeline } from "./timeline.ts";
import { manualTicker } from "./testTicker.ts";

interface FakeEl {
  style: { opacity: string; transform: string; strokeDashoffset: string; setProperty: ReturnType<typeof vi.fn> };
}
const fakeEl = (): Element & FakeEl =>
  ({ style: { opacity: "", transform: "", strokeDashoffset: "", setProperty: vi.fn() } }) as unknown as Element & FakeEl;

describe("timeline", () => {
  it("places children like GSAP", () => {
    const a = { v: 0 };
    const b = { v: 0 };
    expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, "<").duration).toBe(1);
    expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, "+=0.5").duration).toBe(2.5);
    expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, "-=0.5").duration).toBe(1.5);
    expect(timeline().to(a, { v: 1 }, { duration: 1 }).to(b, { v: 1 }, { duration: 1 }, 3).duration).toBe(4);
  });

  it("eases, staggers and restores on seek back", () => {
    const xs = [{ v: 0 }, { v: 0 }, { v: 0 }];
    const tl = timeline().to(xs, { v: 1 }, { duration: 1, ease: linear, stagger: 0.2 });
    expect(tl.duration).toBeCloseTo(1.4);
    tl.seek(0.2);
    expect(xs.map((x) => x.v)).toEqual([expect.closeTo(0.2), 0, 0]);
    tl.seek(0);
    expect(xs[0]?.v).toBe(0);
  });

  it("starts a later tween from where the earlier one left the value", () => {
    const o = { v: 0 };
    const tl = timeline().to(o, { v: 10 }, { duration: 1, ease: linear }).to(o, { v: 0 }, { duration: 1, ease: linear });
    tl.progress(1);
    expect(o.v).toBe(0);
    tl.seek(1);
    expect(o.v).toBeCloseTo(10);
  });

  it("lets the earliest tween own a rewound value, as when a loop restarts", () => {
    const o = { v: 0 };
    const tl = timeline().to(o, { v: 10 }, { duration: 1, ease: linear }).to(o, { v: 0 }, { duration: 1, ease: linear });
    tl.progress(1);
    tl.seek(0.5);
    expect(o.v).toBeCloseTo(5);
    tl.seek(0);
    expect(o.v).toBe(0);
  });

  it("draws the moment of each crossed call before running it, so a jump ends like playing", () => {
    const o = { v: 0 };
    const seen: number[] = [];
    const tl = timeline()
      .to(o, { v: 1 }, { duration: 1, ease: linear })
      .call(() => seen.push(o.v), 0.5);
    tl.progress(1);
    expect(seen).toEqual([expect.closeTo(0.5)]);
    expect(o.v).toBe(1);
  });

  it("fires calls forward only, and all of them on progress(1)", () => {
    const fn = vi.fn();
    const tl = timeline().to({ v: 0 }, { v: 1 }, { duration: 1 }).call(fn, 0.5);
    tl.progress(1);
    tl.seek(0.2);
    tl.seek(0.4);
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it("repeats with a delay and completes once", () => {
    const done = vi.fn();
    const tl = timeline({ repeat: 2, repeatDelay: 1, onComplete: done }).to({ v: 0 }, { v: 1 }, { duration: 1 });
    tl.seek(5);
    expect(tl.loops).toBe(3);
    expect(done).toHaveBeenCalledTimes(1);
    tl.seek(6);
    expect(done).toHaveBeenCalledTimes(1);
  });

  it("yoyos a repeated child", () => {
    const o = { v: 0 };
    const tl = timeline().to(o, { v: 100 }, { duration: 1, ease: linear, repeat: 1, yoyo: true });
    tl.seek(1.5);
    expect(o.v).toBeCloseTo(50);
  });

  it("writes one transform string", () => {
    const el = fakeEl();
    timeline().to(el, { x: 10, scale: 2 }, { duration: 1 }).progress(1);
    expect(el.style.transform).toBe("translate(0%, 0%) translate(10px, 0px) rotateX(0deg) rotateY(0deg) scale(2, 2)");
  });

  it("writes opacity, dash offset and custom properties through style", () => {
    const el = fakeEl();
    timeline().to(el, { opacity: 0.5, strokeDashoffset: 4, "--p": 1 }, { duration: 1, ease: linear }).progress(1);
    expect(el.style.opacity).toBe("0.5");
    expect(el.style.strokeDashoffset).toBe("4");
    expect(el.style.setProperty).toHaveBeenCalledWith("--p", "1");
  });

  it("set applies numbers and strings", () => {
    const el = fakeEl();
    timeline().set(el, { opacity: "0", "--k": "none" }).progress(1);
    expect(el.style.opacity).toBe("0");
    expect(el.style.setProperty).toHaveBeenCalledWith("--k", "none");
  });

  it("rejects unknown props", () => {
    expect(() => {
      timeline().to(fakeEl(), { width: 3 }).progress(1);
    }).toThrow(/unsupported prop width/);
  });

  it("plays on a ticker and stops when done", () => {
    const t = manualTicker();
    const o = { v: 0 };
    const onComplete = vi.fn();
    const tl = timeline({ ticker: t, onComplete }).to(o, { v: 1 }, { duration: 1, ease: linear });
    tl.play();
    t.step(0.5);
    expect(o.v).toBeCloseTo(0.5);
    t.step(0.6);
    expect(o.v).toBe(1);
    expect(onComplete).toHaveBeenCalledTimes(1);
    t.step(1);
    expect(onComplete).toHaveBeenCalledTimes(1);
  });
});
