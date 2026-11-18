/**
 * The app languages (ADR-0168): the list, their own names, their CLDR plural categories, and the
 * negotiation from a BCP 47 preference list. No dependencies, so the export scripts can import it.
 */

/** Every app language. `pt` is Brazilian Portuguese; `zh` is Simplified Chinese. */
export const locales = ["en", "de", "es", "fr", "it", "pt", "nl", "pl", "tr", "ja", "ko", "zh"] as const;
export type Locale = (typeof locales)[number];

/** Each language under its own name, the same in every UI language (not translated). */
export const localeNames: Readonly<Record<Locale, string>> = {
  en: "English",
  de: "Deutsch",
  es: "Español",
  fr: "Français",
  it: "Italiano",
  pt: "Português (Brasil)",
  nl: "Nederlands",
  pl: "Polski",
  tr: "Türkçe",
  ja: "日本語",
  ko: "한국어",
  zh: "简体中文",
};

/** Languages written and reviewed by people; every other one shows `settings.language.machineNote`. */
export const reviewedLocales: readonly Locale[] = ["en", "de"];

/** Whether `locale` was translated with AI help (the UI says so under the language choice). */
export function isMachineTranslated(locale: Locale): boolean {
  return !reviewedLocales.includes(locale);
}

/**
 * The CLDR cardinal plural categories of each language, in canonical order. A plural message may
 * use only these (plus `=N`), and must have `other`. They equal `Intl.PluralRules` (CLDR 44+).
 */
export const pluralCategories: Readonly<Record<Locale, readonly string[]>> = {
  en: ["one", "other"],
  de: ["one", "other"],
  es: ["one", "many", "other"],
  fr: ["one", "many", "other"],
  it: ["one", "many", "other"],
  pt: ["one", "many", "other"],
  nl: ["one", "other"],
  pl: ["one", "few", "many", "other"],
  tr: ["one", "other"],
  ja: ["other"],
  ko: ["other"],
  zh: ["other"],
};

// Chinese written in Traditional characters: served in English, never in Simplified.
const traditionalChineseRegions = new Set(["tw", "hk", "mo"]);

/** The supported locale of one BCP 47 or POSIX tag (`zh_CN.UTF-8`), or `null`. */
export function localeOf(tag: string): Locale | null {
  const parts = tag.trim().toLowerCase().split(/[.@]/)[0]?.split(/[-_]/) ?? [];
  const [base = "", ...rest] = parts;
  if (base === "zh") {
    const script = rest.find((part) => part.length === 4);
    if (script !== undefined) {
      return script === "hans" ? "zh" : null;
    }
    return rest.some((part) => traditionalChineseRegions.has(part)) ? null : "zh";
  }
  return locales.find((locale) => locale === base) ?? null;
}

/** Pick the best supported locale for a BCP 47 preference list; English is the fallback. */
export function negotiateLocale(preferred: readonly string[]): Locale {
  for (const tag of preferred) {
    const match = localeOf(tag);
    if (match !== null) {
      return match;
    }
  }
  return "en";
}
