import { describe, expect, it } from "vitest";
import { catalogs, locales, type Locale } from "./index.ts";

const keysOf = (locale: Locale, prefix: string): string[] =>
  Object.keys(catalogs[locale])
    .filter((k) => k.startsWith(prefix))
    .sort();

describe("mobile texts exist in every language", () => {
  it.each(locales)("every mobile.* key is in %s with a non-empty text", (locale) => {
    const en = keysOf("en", "mobile.");
    expect(en.length).toBeGreaterThan(0);
    expect(keysOf(locale, "mobile.")).toEqual(en);
    for (const key of en) {
      const text = (catalogs[locale] as Record<string, string>)[key];
      expect(text?.trim(), `${locale} ${key}`).toBeTruthy();
    }
  });

  it.each(locales)("every mobile.error.<code> in %s overrides an error.<code> that exists", (locale) => {
    const en = keysOf("en", "mobile.error.");
    expect(en.length).toBeGreaterThan(0);
    expect(keysOf(locale, "mobile.error.")).toEqual(en);
    for (const key of en) {
      const base = `error.${key.slice("mobile.error.".length)}`;
      expect(Object.keys(catalogs[locale]), `${locale} ${base}`).toContain(base);
    }
  });
});
