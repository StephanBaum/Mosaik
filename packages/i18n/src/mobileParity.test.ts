import { describe, expect, it } from "vitest";
import { catalogs } from "./index.ts";

const keysOf = (locale: "en" | "de", prefix: string): string[] =>
  Object.keys(catalogs[locale])
    .filter((k) => k.startsWith(prefix))
    .sort();

describe("mobile texts exist in both languages", () => {
  it("every mobile.* key is in EN and DE with a non-empty text", () => {
    const en = keysOf("en", "mobile.");
    expect(en.length).toBeGreaterThan(0);
    expect(keysOf("de", "mobile.")).toEqual(en);
    for (const key of en) {
      for (const locale of ["en", "de"] as const) {
        const text = (catalogs[locale] as Record<string, string>)[key];
        expect(text?.trim(), `${locale} ${key}`).toBeTruthy();
      }
    }
  });

  it("every mobile.error.<code> is in both languages and overrides an error.<code> that exists", () => {
    const en = keysOf("en", "mobile.error.");
    expect(en.length).toBeGreaterThan(0);
    expect(keysOf("de", "mobile.error.")).toEqual(en);
    for (const key of en) {
      const base = `error.${key.slice("mobile.error.".length)}`;
      for (const locale of ["en", "de"] as const) {
        expect(Object.keys(catalogs[locale]), `${locale} ${base}`).toContain(base);
      }
    }
  });
});
