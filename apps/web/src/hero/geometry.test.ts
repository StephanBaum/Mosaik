import { describe, expect, it } from "vitest";
import { canvasScale, coverMap, follow, parseObjectPosition, rectUv, scrollProgress } from "./geometry.ts";

describe("coverMap", () => {
  const image = { width: 2000, height: 1000 };

  it("is the identity when the view has the image's aspect", () => {
    expect(coverMap({ width: 1000, height: 500 }, image, 0.5, 0.5)).toEqual([1, 1, -0, -0]);
  });

  it("crops the sides of a tall view around the object position", () => {
    // 500 x 1000 view: the image is scaled to 2000 x 1000 and shows a quarter of its width.
    const [su, sv, ou, ov] = coverMap({ width: 500, height: 1000 }, image, 0.5, 0.5);
    expect(su).toBeCloseTo(0.25);
    expect(sv).toBeCloseTo(1);
    expect(ou).toBeCloseTo(0.375);
    expect(ov).toBeCloseTo(0);
    // Pushed right: the right edge of the view is the right edge of the image.
    const right = coverMap({ width: 500, height: 1000 }, image, 1, 0.5);
    expect(right[0] + right[2]).toBeCloseTo(1);
  });

  it("falls back to the identity for empty sizes", () => {
    expect(coverMap({ width: 0, height: 10 }, image, 0.5, 0.5)).toEqual([1, 1, 0, 0]);
  });
});

describe("parseObjectPosition", () => {
  it("reads percentages and keywords", () => {
    expect(parseObjectPosition("62% 40%")).toEqual([0.62, 0.4]);
    expect(parseObjectPosition("right top")).toEqual([1, 0]);
    expect(parseObjectPosition("center")).toEqual([0.5, 0.5]);
    expect(parseObjectPosition("12px 50%")).toEqual([0.5, 0.5]);
  });
});

describe("canvasScale", () => {
  it("follows the device pixel ratio inside the budget", () => {
    expect(canvasScale({ width: 1000, height: 500 }, 2, 1, 4e6)).toBe(2);
    expect(canvasScale({ width: 1000, height: 500 }, 1, 0.5, 4e6)).toBe(0.5);
  });

  it("caps the area at the budget", () => {
    const scale = canvasScale({ width: 2000, height: 1000 }, 2, 1, 1e6);
    expect(2000 * scale * 1000 * scale).toBeCloseTo(1e6);
  });
});

describe("follow", () => {
  it("approaches the target independent of the frame split", () => {
    const once = follow(0, 1, 5, 0.2);
    const split = follow(follow(0, 1, 5, 0.1), 1, 5, 0.1);
    expect(split).toBeCloseTo(once);
    expect(once).toBeGreaterThan(0.6);
    expect(once).toBeLessThan(1);
  });

  it("stays put with no time or no rate", () => {
    expect(follow(0.3, 1, 5, 0)).toBeCloseTo(0.3);
    expect(follow(0.3, 1, 0, 1)).toBeCloseTo(0.3);
  });
});

describe("scrollProgress and rectUv", () => {
  it("clamps the scroll share", () => {
    expect(scrollProgress(0, 800)).toBe(0);
    expect(scrollProgress(-400, 800)).toBe(0.5);
    expect(scrollProgress(-2000, 800)).toBe(1);
    expect(scrollProgress(100, 800)).toBe(0);
  });

  it("maps a child box into its parent", () => {
    expect(rectUv({ left: 10, top: 20, right: 60, bottom: 70 }, { left: 0, top: 0, width: 100, height: 100 })).toEqual([
      0.1, 0.2, 0.6, 0.7,
    ]);
  });
});
