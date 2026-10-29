/** ADR-0014: the holo button's flat fallback must not depend on the theme's surface colors. */
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

describe("app.css .holo", () => {
  const css = readFileSync(join(import.meta.dirname, "app.css"), "utf8");
  const block = /^\.holo \{([^}]*)\}/m.exec(css)?.[1] ?? "";

  it("has a rule", () => {
    expect(block).toContain("linear-gradient");
  });

  it("uses no theme colors, only --primary-action-* variables", () => {
    expect(block).not.toContain("var(--color-");
    const used = [...block.matchAll(/var\((--[\w-]+)\)/g)].map((match) => match[1] ?? "");
    expect(used.length).toBeGreaterThan(0);
    for (const name of used) {
      expect(name).toMatch(/^--primary-action-/);
    }
  });
});
