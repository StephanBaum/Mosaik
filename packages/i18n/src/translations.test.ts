/// <reference types="node" />
/**
 * The translation gate (ADR-0168). A scaffolded catalog file starts with the marker line below
 * and holds English; a translator replaces every value and deletes the marker.
 *
 * `TRANSLATIONS_PENDING` lists the locales whose translation has not landed yet. For those the
 * check is inverted (`it.fails`), so the branch stays green while they are English copies. Remove
 * a locale from the list when its files are translated; the check then fails while any of its
 * files still holds the marker, naming them. Nothing may be released or merged to main while the
 * list is not empty.
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { locales, type Locale } from "./index.ts";

const TRANSLATIONS_PENDING: readonly Locale[] = [];

const MARKER = "// UNTRANSLATED: copy of English, replace every value";
const catalogDir = join(import.meta.dirname, "catalog");

/** The catalog files of `locale` that still hold the marker, as `catalog/<locale>/<file>`. */
function untranslated(locale: Locale): string[] {
  const dir = join(catalogDir, locale);
  return readdirSync(dir)
    .filter((file) => file.endsWith(".ts"))
    .filter((file) => readFileSync(join(dir, file), "utf8").includes(MARKER))
    .map((file) => `catalog/${locale}/${file}`);
}

describe("translation gate", () => {
  const translated = locales.filter((locale) => locale !== "en");

  describe.each(translated)("%s", (locale) => {
    const check = TRANSLATIONS_PENDING.includes(locale) ? it.fails : it;
    check("no catalog file still holds the UNTRANSLATED marker", () => {
      expect(untranslated(locale)).toEqual([]);
    });
  });

  it("lists only real locales, never English or German, as pending", () => {
    for (const locale of TRANSLATIONS_PENDING) {
      expect(locales).toContain(locale);
      expect(["en", "de"]).not.toContain(locale);
    }
  });

  it("finds the marker only as a whole first line", () => {
    for (const locale of translated) {
      for (const file of untranslated(locale)) {
        const first = readFileSync(join(catalogDir, file.slice("catalog/".length)), "utf8").split("\n")[0];
        expect(first?.trimEnd(), file).toBe(MARKER);
      }
    }
  });
});
