/**
 * The popup's strings: every error code the extension can show has a text in both languages
 * (and nothing else is in the `ext.error.` namespace), and no popup source carries a raw colour
 * or pixel size (UX_GUIDELINES §2 and §6).
 */
import { catalogs, locales, type MessageKey } from "@mosaic/i18n";
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { LOCAL_ERROR_CODES, WIRE_ERROR_CODES } from "../shared/errors.ts";

const CODES = [...WIRE_ERROR_CODES, ...LOCAL_ERROR_CODES];

describe("error texts", () => {
  it.each(locales)("%s has a text for every error code, and no other ext.error key", (locale) => {
    const keys = Object.keys(catalogs[locale]).filter((key) => key.startsWith("ext.error."));
    expect(keys.sort()).toEqual(CODES.map((code) => `ext.error.${code}`).sort());
  });

  it.each(locales)("%s texts are sentences, not code names", (locale) => {
    for (const code of CODES) {
      const text = catalogs[locale][`ext.error.${code}` as MessageKey];
      expect(text.length, code).toBeGreaterThan(10);
      expect(text, code).not.toMatch(/[a-z][A-Z]/);
    }
  });
});

const dir = join(import.meta.dirname);

function files(path: string, ending: string): string[] {
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    const full = join(path, entry.name);
    if (entry.isDirectory()) {
      return files(full, ending);
    }
    return entry.name.endsWith(ending) && !entry.name.endsWith(`.test${ending}`) ? [full] : [];
  });
}

const COLOR = /#[0-9a-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|lab|lch)\(/i;

describe("popup styles use tokens only", () => {
  it.each(files(dir, ".css"))("%s", (path) => {
    const css = readFileSync(path, "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
    expect(css).not.toMatch(COLOR);
    // 1px only for the visually-hidden clip pattern and the hairline.
    const sizes = css.match(/\b\d*\.?\d+px\b/g) ?? [];
    expect(sizes.filter((size) => size !== "1px")).toEqual([]);
  });

  it.each([...files(dir, ".tsx")])("%s has no inline colors or styles", (path) => {
    const source = readFileSync(path, "utf8");
    expect(source).not.toMatch(COLOR);
    expect(source).not.toMatch(/style=\{/);
  });
});
