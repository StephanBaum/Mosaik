/** The hero art's choreography: decode steps, inertia, scroll re-encryption, palette maths. */
import { describe, expect, it } from "vitest";
import { hexToOklab } from "./palette.ts";
import { defaultParams } from "./params.ts";
import { DECODE_STEPS, follow, introEnd, introState, scrollLook, scrollOut } from "./timeline.ts";

const decode = defaultParams().decode;

describe("introState", () => {
  it("steps the blocks 64 → 32 → 16 → 8 with a sweep from top to bottom", () => {
    const steps = decode.duration * 0.8;
    const seen = [0.01, 0.26, 0.51, 0.76].map((share) => introState(share * steps, decode));
    expect(seen.map((s) => s.block)).toEqual([...DECODE_STEPS]);
    expect(seen.every((s) => s.reveal === 0)).toBe(true);
    const sweeps = seen.map((s) => s.sweep);
    expect([...sweeps].sort((a, b) => a - b)).toEqual(sweeps);
  });

  it("dissolves into the clear image by the end of the decode, holds, then collapses", () => {
    expect(introState(decode.duration - 1e-3, decode).reveal).toBeGreaterThan(0.99);
    expect(introState(decode.duration + decode.hold / 2, decode)).toMatchObject({ block: 0, reveal: 1 });
    const mid = introState(decode.duration + decode.hold + decode.collapse / 2, decode);
    expect(mid.reveal).toBeGreaterThan(0.2);
    expect(mid.reveal).toBeLessThan(0.8);
  });

  it("settles mostly encrypted and stops", () => {
    expect(introState(introEnd(decode) + 0.01, decode)).toEqual({ block: 0, reveal: 0, sweep: -1, running: false });
  });
});

describe("follow", () => {
  it("is frame-rate independent", () => {
    let fine = 0;
    for (let i = 0; i < 4; i++) {
      fine = follow(fine, 100, 6, 1 / 120);
    }
    const coarse = follow(0, 100, 6, 4 / 120);
    expect(fine).toBeCloseTo(coarse, 6);
  });
});

describe("scroll", () => {
  it("measures how far the hero has left the viewport", () => {
    expect(scrollOut(0, 900)).toBe(0);
    expect(scrollOut(-450, 900)).toBe(0.5);
    expect(scrollOut(-2000, 900)).toBe(1);
    expect(scrollOut(120, 900)).toBe(0);
    expect(scrollOut(0, 0)).toBe(0);
  });

  it("re-encrypts on the way out: blocks grow, the key closes, streaks pull down", () => {
    const top = scrollLook(0);
    const out = scrollLook(1);
    expect(top).toEqual({ blockScale: 1, keyScale: 1, sortBoost: 0, pull: 0 });
    expect(out.blockScale).toBeGreaterThan(2.5);
    expect(out.keyScale).toBe(0);
    expect(out.pull).toBeGreaterThan(0);
  });
});

describe("hexToOklab", () => {
  it("converts white, black and a loud colour", () => {
    const white = hexToOklab("#ffffff");
    expect(white?.[0]).toBeCloseTo(1, 3);
    expect(hexToOklab("#000")).toEqual([0, 0, 0]);
    const lime = hexToOklab(" #A8F542 ");
    expect(lime?.[0]).toBeGreaterThan(0.85);
    expect(Math.hypot(lime?.[1] ?? 0, lime?.[2] ?? 0)).toBeGreaterThan(0.15);
  });

  it("rejects anything but hex colours", () => {
    expect(hexToOklab("rgb(0 0 0)")).toBeNull();
    expect(hexToOklab("")).toBeNull();
  });
});
