import { describe, expect, it } from "vitest";
import {
  catalogs,
  formatMessage,
  icuArguments,
  icuPluralSelectors,
  isMachineTranslated,
  localeNames,
  localeOf,
  locales,
  negotiateLocale,
  pluralCategories,
  type MessageKey,
} from "./index.ts";

const keys = Object.keys(catalogs.en) as MessageKey[];
const sorted = (names: Set<string>): string[] => [...names].sort();

// One test per locale that names every bad key (a test per key and locale is ~70,000 tests).
describe("catalog parity", () => {
  it.each(locales)("%s has exactly the English keys", (locale) => {
    expect(Object.keys(catalogs[locale]).sort()).toEqual([...keys].sort());
  });

  it.each(locales)("%s: every message is non-empty and uses English's ICU arguments", (locale) => {
    const problems = keys.flatMap((key) => {
      const message = catalogs[locale][key];
      const want = sorted(icuArguments(catalogs.en[key]));
      const got = sorted(icuArguments(message));
      if (message.trim() === "") return [`${key}: empty`];
      return got.join() === want.join() ? [] : [`${key}: {${got.join()}} instead of {${want.join()}}`];
    });
    expect(problems).toEqual([]);
  });

  it.each(locales)("%s: plurals use only its CLDR categories (or =N) and have other", (locale) => {
    const allowed = new Set(pluralCategories[locale]);
    const problems = keys.flatMap((key) =>
      icuPluralSelectors(catalogs[locale][key]).flatMap((selectors) => {
        const bad = selectors.filter((s) => !allowed.has(s) && !/^=\d+$/.test(s));
        const missing = selectors.includes("other") ? [] : ["other is missing"];
        return [...bad.map((s) => `"${s}" is not a ${locale} category`), ...missing].map(
          (why) => `${key}: ${why}`,
        );
      }),
    );
    expect(problems).toEqual([]);
  });

  it("the plural categories are the ones Intl.PluralRules knows", () => {
    for (const locale of locales) {
      const intl = new Intl.PluralRules(locale).resolvedOptions().pluralCategories;
      expect([...intl].sort(), locale).toEqual([...pluralCategories[locale]].sort());
    }
  });
});

describe("icuPluralSelectors", () => {
  it("lists the selectors of every plural, also nested in select", () => {
    expect(icuPluralSelectors("{n, plural, =0 {none} one {#} other {# {x}}}")).toEqual([
      ["=0", "one", "other"],
    ]);
    expect(
      icuPluralSelectors("{g, select, a {{n, plural, other {#}}} other {{m, plural, few {#} other {#}}}}"),
    ).toEqual([["other"], ["few", "other"]]);
    expect(icuPluralSelectors("{n, plural, offset:1 one {a} other {b}}")).toEqual([["one", "other"]]);
    expect(icuPluralSelectors("{g, select, a {x} other {y}}")).toEqual([]);
  });
});

describe("German terminology", () => {
  it("uses Emergency Kit and Secret Key, not translated variants", () => {
    for (const [key, value] of Object.entries(catalogs.de)) {
      expect(value, key).not.toMatch(/Notfall-Kit|Geheime[nrs]? Schlüssel/);
    }
  });
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
    expect(negotiateLocale(["fr-FR", "EN-gb"])).toBe("fr");
    expect(negotiateLocale(["ru-RU", "EN-gb"])).toBe("en");
    expect(negotiateLocale(["ja"])).toBe("ja");
    expect(negotiateLocale(["ko-KR"])).toBe("ko");
    expect(negotiateLocale(["sv", "ru"])).toBe("en");
    expect(negotiateLocale([])).toBe("en");
  });

  it("serves Brazilian Portuguese to every Portuguese tag", () => {
    expect(negotiateLocale(["pt-BR"])).toBe("pt");
    expect(negotiateLocale(["pt-PT"])).toBe("pt");
    expect(negotiateLocale(["pt"])).toBe("pt");
  });

  it("serves Simplified Chinese only to Simplified tags", () => {
    for (const tag of ["zh", "zh-CN", "zh-SG", "zh-Hans", "zh-Hans-CN", "zh-Hans-HK", "zh_CN.UTF-8"]) {
      expect(negotiateLocale([tag]), tag).toBe("zh");
    }
    for (const tag of ["zh-TW", "zh-HK", "zh-MO", "zh-Hant", "zh-Hant-TW", "zh-Hant-CN", "zh_TW.UTF-8"]) {
      expect(negotiateLocale([tag]), tag).toBe("en");
    }
    // Traditional is unsupported, so the next preference counts.
    expect(negotiateLocale(["zh-TW", "ja"])).toBe("ja");
  });

  it("reads POSIX locale names and ignores unknown ones", () => {
    expect(localeOf("pl_PL.UTF-8")).toBe("pl");
    expect(localeOf("tr_TR@euro")).toBe("tr");
    expect(localeOf("C")).toBeNull();
    expect(localeOf("")).toBeNull();
  });
});

describe("language names", () => {
  it("names every locale in its own language", () => {
    expect(Object.keys(localeNames).sort()).toEqual([...locales].sort());
    expect(localeNames.es).toBe("Español");
    expect(localeNames.zh).toBe("简体中文");
  });

  it("marks every language but English and German as translated with AI help", () => {
    expect(locales.filter((locale) => !isMachineTranslated(locale))).toEqual(["en", "de"]);
  });
});

describe("formatMessage", () => {
  it("formats plurals and arguments in both locales", () => {
    expect(formatMessage("en", "vault.itemCount", { count: 0 })).toBe("No items");
    expect(formatMessage("en", "vault.itemCount", { count: 1 })).toBe("1 item");
    expect(formatMessage("de", "vault.itemCount", { count: 2 })).toBe("2 Einträge");
    expect(formatMessage("de", "onboarding.step", { current: 1, total: 5 })).toBe(
      "Schritt 1 von 5",
    );
  });

  it.each(locales)("%s: every message is valid ICU and formats with sample values", (locale) => {
    const problems = keys.flatMap((key) => {
      const values = Object.fromEntries(
        [...icuArguments(catalogs.en[key])].map((name) => [name, 2]),
      );
      try {
        return formatMessage(locale, key, values).trim() === "" ? [`${key}: empty`] : [];
      } catch (error) {
        return [`${key}: ${String(error)}`];
      }
    });
    expect(problems).toEqual([]);
  });
});
