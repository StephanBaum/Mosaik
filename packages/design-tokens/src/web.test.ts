import { describe, expect, it } from "vitest";
import { contrastRatio } from "./index.ts";
import { webAlphas, webColors, webCssVariables } from "./web.ts";

describe("web palette", () => {
  it("keeps text contrast on the dark ground", () => {
    expect(contrastRatio(webColors.paper, webColors.ink)).toBeGreaterThanOrEqual(15);
    expect(contrastRatio(webColors.paper2, webColors.ink)).toBeGreaterThanOrEqual(7);
    for (const bg of [webColors.ink, webColors.inkRaised]) {
      expect(contrastRatio(webColors.paper3, bg)).toBeGreaterThanOrEqual(4.5);
      expect(contrastRatio(webColors.tintText, bg)).toBeGreaterThanOrEqual(7);
      expect(contrastRatio(webColors.tint, bg)).toBeGreaterThanOrEqual(4.5);
    }
  });

  it("emits each variable once with prototype names", () => {
    const css = webCssVariables();
    const names = [...css.matchAll(/(--[a-z0-9-]+):/g)].map((m) => m[1]);
    expect(new Set(names).size).toBe(names.length);
    expect(css).toContain("--paper-2:#c9c6d6");
    expect(css).toContain("--c-cobalt:#2b4bff");
    expect(css).toContain(`--rule:${String(webAlphas["rule"])}`);
    expect(css).toContain("--layer-veil:50;");
  });
});
