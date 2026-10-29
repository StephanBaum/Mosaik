import { describe, expect, it } from "vitest";
import { cssVariables } from "./css.ts";
import { colors, fontSize, radius, space } from "./index.ts";

describe("cssVariables", () => {
  const css = cssVariables();

  it("declares every spacing, radius and type token", () => {
    space.forEach((value, index) => {
      expect(css).toContain(`--space-${String(index)}: ${String(value)}px;`);
    });
    for (const [name, value] of Object.entries(radius)) {
      expect(css).toContain(`--radius-${name}: ${String(value)}px;`);
    }
    for (const [name, { size, lineHeight }] of Object.entries(fontSize)) {
      expect(css).toContain(`--font-size-${name}: ${String(size)}px;`);
      expect(css).toContain(`--line-height-${name}: ${String(lineHeight)}px;`);
    }
  });

  it("declares every color of both themes, with system as the default", () => {
    expect(css).toContain(`--color-bg-subtle: ${colors.light.bgSubtle};`);
    expect(css).toContain(`--color-focus-ring: ${colors.dark.focusRing};`);
    expect(css).toContain(':root[data-theme="dark"]');
    expect(css).toContain('@media (prefers-color-scheme: dark) {\n  :root:not([data-theme="light"])');
    const darkBlocks = css.split(`--color-bg: ${colors.dark.bg};`).length - 1;
    expect(darkBlocks).toBe(2);
  });

  it("declares the M3 UI tokens", () => {
    for (const name of [
      "--color-text-secondary",
      "--color-bg-selected",
      "--color-scrim",
      "--shadow-popover",
      "--vault-color-5",
      "--print-text",
      "--layer-toast: 40;",
      "--primary-action-label-shadow",
    ]) {
      expect(css).toContain(name);
    }
  });

  it("turns motion off for reduced motion", () => {
    expect(css).toMatch(/prefers-reduced-motion: reduce\) \{\s*:root \{\s*--motion-micro: 0ms;/);
  });

  it("never references a remote resource", () => {
    expect(css).not.toMatch(/url\(|@import|https?:/);
  });
});
