import { describe, expect, it } from "vitest";
import { catalogs, icuArguments, locales, negotiateLocale, type MessageKey } from "./index.ts";

const keys = Object.keys(catalogs.en) as MessageKey[];

describe("catalog parity", () => {
  it.each(locales)("%s has exactly the English keys", (locale) => {
    expect(Object.keys(catalogs[locale]).sort()).toEqual([...keys].sort());
  });

  it.each(locales.flatMap((locale) => keys.map((key) => [locale, key] as const)))(
    "%s %s is non-empty and uses the same ICU arguments as English",
    (locale, key) => {
      const message = catalogs[locale][key];
      expect(message.trim()).not.toBe("");
      expect(icuArguments(message)).toEqual(icuArguments(catalogs.en[key]));
    },
  );
});

describe("icuArguments", () => {
  it("finds simple and plural arguments but not branch selectors", () => {
    expect(icuArguments("Hi {name}")).toEqual(new Set(["name"]));
    expect(icuArguments("{n, plural, one {# item} other {# items for {who}}}")).toEqual(
      new Set(["n", "who"]),
    );
  });

  it("handles select, offsets and simple styles", () => {
    expect(
      icuArguments("{g, select, female {{who} liked it} other {they liked it on {d, date, short}}}"),
    ).toEqual(new Set(["g", "who", "d"]));
    expect(icuArguments("{n, plural, offset:1 =0 {nobody} other {# others}}")).toEqual(
      new Set(["n"]),
    );
  });

  it("rejects unbalanced braces", () => {
    expect(() => icuArguments("{oops")).toThrow(/Unbalanced/);
    expect(() => icuArguments("oops}")).toThrow(/Unbalanced/);
    expect(() => icuArguments("{n, plural, one {x}")).toThrow(/Unbalanced/);
    expect(() => icuArguments("{n, plural, one {x")).toThrow(/Unbalanced/);
  });
});

describe("negotiateLocale", () => {
  it("matches on the base language and falls back to English", () => {
    expect(negotiateLocale(["de-AT", "en"])).toBe("de");
    expect(negotiateLocale(["fr-FR", "EN-gb"])).toBe("en");
    expect(negotiateLocale(["ja"])).toBe("en");
    expect(negotiateLocale([])).toBe("en");
  });
});
