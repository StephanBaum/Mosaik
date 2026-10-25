import { describe, expect, it } from "vitest";
import { colors, contrastRatio, type ColorScheme } from "./index.ts";

type Pair = [fg: keyof ColorScheme, bg: keyof ColorScheme, min: number];

// WCAG 2.2 AA: 1.4.3 text ≥ 4.5:1, 1.4.11 UI components and focus ≥ 3:1.
const requirements: Pair[] = [
  ...(["text", "textMuted", "danger", "warning", "success", "accent"] as const).flatMap(
    (fg) => (["bg", "bgSubtle", "surface"] as const).map((bg): Pair => [fg, bg, 4.5]),
  ),
  ["accentText", "accent", 4.5],
  ...(["focusRing", "borderStrong"] as const).flatMap((fg) =>
    (["bg", "bgSubtle", "surface"] as const).map((bg): Pair => [fg, bg, 3]),
  ),
];

describe("contrastRatio", () => {
  it("matches the WCAG reference extremes", () => {
    expect(contrastRatio("#000000", "#FFFFFF")).toBeCloseTo(21, 5);
    expect(contrastRatio("#777777", "#777777")).toBe(1);
  });

  it("is symmetric", () => {
    expect(contrastRatio("#2F5BD3", "#FFFFFF")).toBe(contrastRatio("#FFFFFF", "#2F5BD3"));
  });

  it("rejects malformed colors", () => {
    expect(() => contrastRatio("blue", "#FFFFFF")).toThrow(/#RRGGBB/);
  });
});

describe.each(Object.entries(colors))("%s theme", (_name, scheme) => {
  it.each(requirements)("%s on %s has contrast ≥ %d:1", (fg, bg, min) => {
    expect(contrastRatio(scheme[fg], scheme[bg])).toBeGreaterThanOrEqual(min);
  });
});
