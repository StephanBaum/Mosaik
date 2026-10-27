/**
 * UX_GUIDELINES §2 and §9: only tokens in app styles. Colors must come from the token
 * stylesheet, and sizes from the spacing, type, radius and border tokens.
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const srcDir = join(import.meta.dirname, "..");

function files(dir: string, extensions: readonly string[]): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      return files(path, extensions);
    }
    return extensions.some((ext) => entry.name.endsWith(ext)) ? [path] : [];
  });
}

const COLOR = /#[0-9a-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|lab|lch)\(/i;

describe("app styles use tokens only", () => {
  it.each(files(srcDir, [".css"]))("%s", (path) => {
    const css = readFileSync(path, "utf8")
      .replace(/\/\*[\s\S]*?\*\//g, "")
      // Breakpoints cannot use custom properties (900 px: UX_GUIDELINES §3).
      .replace(/@media[^{]*\{/g, "@media {");
    expect(css).not.toMatch(COLOR);
    // 1px only for the visually-hidden clip pattern.
    const sizes = css.match(/\b\d*\.?\d+px\b/g) ?? [];
    expect(sizes.filter((size) => size !== "1px")).toEqual([]);
  });

  it.each(files(srcDir, [".tsx"]))("%s has no inline colors", (path) => {
    expect(readFileSync(path, "utf8")).not.toMatch(COLOR);
  });
});
