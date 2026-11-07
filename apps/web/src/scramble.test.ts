/** Scramble-decode timing and the build-time art helpers (deterministic output). */
import { describe, expect, it } from "vitest";
import { bandTone, ditherPath } from "./pages/Art.tsx";
import { charState, decodeEnd, GLYPHS, type ScrambleTiming } from "./scramble.ts";

const timing: ScrambleTiming = { delay: 100, stagger: 20, cycle: 300, tick: 50, blocks: true };

describe("charState", () => {
  it("shows a pixel block before the character starts", () => {
    const state = charState(3, 50, timing);
    expect(state.kind).toBe("block");
    if (state.kind === "block") {
      expect(state.tone).toBeGreaterThanOrEqual(1);
      expect(state.tone).toBeLessThanOrEqual(5);
    }
  });

  it("stays plain before it starts on hover", () => {
    expect(charState(3, 0, { ...timing, blocks: false })).toEqual({ kind: "plain" });
  });

  it("cycles random glyphs, then settles in stagger order", () => {
    const glyph = charState(0, 150, timing);
    expect(glyph.kind).toBe("glyph");
    if (glyph.kind === "glyph") {
      expect(GLYPHS).toContain(glyph.glyph);
    }
    expect(charState(0, 100 + 300, timing)).toEqual({ kind: "plain" });
    expect(charState(5, 100 + 300, timing).kind).toBe("glyph");
  });

  it("is deterministic", () => {
    expect(charState(7, 333, timing)).toEqual(charState(7, 333, timing));
  });

  it("ends when the last character settles", () => {
    expect(decodeEnd(10, timing)).toBe(100 + 9 * 20 + 300);
    expect(charState(9, decodeEnd(10, timing), timing)).toEqual({ kind: "plain" });
  });
});

describe("art helpers", () => {
  it("orders the band tones from purple to pink across the columns", () => {
    const tones = Array.from({ length: 24 }, (_, c) => bandTone(c, 1));
    expect(tones.every((t) => t >= 0 && t <= 5)).toBe(true);
    const loud = tones.filter((t) => t > 0);
    expect(loud[0] ?? 0).toBeLessThan(loud[loud.length - 1] ?? 0);
  });

  it("draws a stable halftone blob of circles", () => {
    const path = ditherPath(3, 240, 8);
    expect(path).toBe(ditherPath(3, 240, 8));
    expect(path).not.toBe(ditherPath(11, 240, 8));
    expect(path.match(/M/g)?.length ?? 0).toBeGreaterThan(40);
    expect(path).toMatch(/^M[\d.]+ [\d.]+a/);
  });
});
