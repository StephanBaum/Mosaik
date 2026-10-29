import { describe, expect, it } from "vitest";
import { colors, contrastRatio, primaryAction, type ColorScheme } from "./index.ts";

type Pair = [fg: keyof ColorScheme, bg: keyof ColorScheme, min: number];

// WCAG 2.2 AA: 1.4.3 text ≥ 4.5:1, 1.4.11 UI components and focus ≥ 3:1.
const requirements: Pair[] = [
  ...(["text", "textMuted", "danger", "warning", "success"] as const).flatMap(
    (fg) => (["bg", "bgSubtle", "surface"] as const).map((bg): Pair => [fg, bg, 4.5]),
  ),
  // `accent` is a fill only (never text on bg): its label needs 4.5:1, and the
  // fill marks the primary control's boundary, so it needs 3:1 like any UI
  // component (1.4.11). A fill that cannot reach 3:1 needs an outline token
  // that does, tested here instead.
  ["accentText", "accent", 4.5],
  ...(["textSecondary"] as const).flatMap((fg) =>
    (["bg", "bgSubtle", "surface", "bgSelected"] as const).map((bg): Pair => [fg, bg, 4.5]),
  ),
  ...(["bgSelected", "dangerSubtle", "warningSubtle", "successSubtle"] as const).map(
    (bg): Pair => ["text", bg, 4.5],
  ),
  ["danger", "dangerSubtle", 3],
  ["warning", "warningSubtle", 3],
  ["success", "successSubtle", 3],
  ...(["focusRing", "borderStrong", "accent"] as const).flatMap((fg) =>
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

describe("primaryAction", () => {
  it("keeps its label readable on the solid fallback", () => {
    expect(contrastRatio(primaryAction.label, primaryAction.fallback)).toBeGreaterThanOrEqual(4.5);
  });
});
