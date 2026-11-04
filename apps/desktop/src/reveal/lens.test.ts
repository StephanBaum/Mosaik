import { describe, expect, it } from "vitest";
import { edgeR, LENS_GLSL, LENS_N, lensBounds, lensClip, type Lens } from "./lens.ts";
import { GLASS, lensAt } from "./timing.ts";

const ORIGIN = [200, 150] as const;

function points(clip: string): [number, number][] {
  const body = clip.slice("polygon(".length, -1);
  return body.split(",").map((p) => {
    const [x, y] = p.split(" ").map((v) => Number.parseFloat(v));
    return [x ?? Number.NaN, y ?? Number.NaN];
  });
}

describe("lensClip", () => {
  it("has exactly LENS_N points", () => {
    const lens = lensAt("unlock", 0.5, 800);
    expect(points(lensClip(lens, ORIGIN))).toHaveLength(LENS_N);
    expect(LENS_N).toBe(120);
  });

  it("collapses to the origin at radius zero", () => {
    const lens = lensAt("unlock", 0, 800);
    for (const [x, y] of points(lensClip(lens, ORIGIN))) {
      expect(x).toBe(ORIGIN[0]);
      expect(y).toBe(ORIGIN[1]);
    }
  });

  it("is a circle without shape and wobble", () => {
    const lens: Lens = { R: 100, t: 1, shape: 0, wob: 0, H: 0, pres: 0 };
    for (const [x, y] of points(lensClip(lens, ORIGIN))) {
      expect(Math.hypot(x - ORIGIN[0], y - ORIGIN[1])).toBeCloseTo(100, 1);
    }
  });

  it("is irregular with the owner's shape, within its bounds", () => {
    const lens: Lens = { R: 300, t: 0.7, shape: GLASS.shape, wob: 0, H: 0, pres: 0 };
    const radii = Array.from({ length: LENS_N }, (_, i) => edgeR(-Math.PI + (i / LENS_N) * 2 * Math.PI, lens));
    expect(Math.max(...radii) - Math.min(...radii)).toBeGreaterThanOrEqual(0.05 * lens.R);
    const { inner, outer } = lensBounds(lens);
    for (const r of radii) {
      expect(r).toBeGreaterThanOrEqual(inner);
      expect(r).toBeLessThanOrEqual(outer);
    }
  });
});

describe("LENS_GLSL", () => {
  it("spells out the same terms as the JS outline", () => {
    for (const term of [
      "0.6*sin(a+2.1+1.3*uT)",
      "0.4*sin(2.0*a-0.4-1.1*uT)",
      "0.35*sin(3.0*a+5.0*uT)",
      "0.7*sin(a+3.1+0.15*uT)",
      "0.3*sin(2.0*a+1.7-0.1*uT)",
      "0.30*sin(2.0*a+1.3+0.4*uT)",
      "0.15*sin(3.0*a-0.7)",
    ]) {
      expect(LENS_GLSL).toContain(term);
    }
  });
});
