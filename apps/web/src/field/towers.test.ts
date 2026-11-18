import { describe, expect, it } from "vitest";
import { label } from "./dataLayer.ts";
import { h21, towerH, type TowerParams } from "./towers.ts";

const at = (lx: number, ly: number): TowerParams => ({ cs: 12, t: 6.5, lx, ly, R: 260, click: null });

describe("towers", () => {
  it("hashes into [0, 1)", () => {
    for (let i = 0; i < 500; i++) {
      const v = h21(i * 1.7, i * 0.3);
      expect(v).toBeGreaterThanOrEqual(0);
      expect(v).toBeLessThan(1);
    }
  });
  it("raises towers near the lens only", () => {
    expect(towerH(1000, 1000, at(0, 0))).toBe(0);
    const near = Array.from({ length: 25 }, (_, k) => towerH(k % 5, Math.floor(k / 5), at(30, 30)));
    expect(Math.max(...near)).toBeGreaterThan(0);
  });
  it("raises towers on the click wave", () => {
    const p: TowerParams = { ...at(0, 0), click: { x: 5000, y: 5000, age: 0.4 } };
    const ring = 0.4 * 260 * 1.5;
    const heights = Array.from({ length: 40 }, (_, k) => towerH(Math.round((5000 + ring) / 12) + (k % 4), Math.round(5000 / 12) + Math.floor(k / 4), p));
    expect(Math.max(...heights)).toBeGreaterThan(0);
  });
  it("formats labels", () => {
    expect(label(1, 0, 0, 0.5)).toBe("Δ +6.00");
    expect(label(2, 0, 0, 0.5)).toBe("n=256");
    expect(label(0, 3, 4, 0.5)).toMatch(/^[0-9a-f]{1,4} {2}50\.0$/);
  });
});
