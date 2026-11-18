/** The app languages (ADR-0168): the setting's list, the locale it picks, and the kit's fallback. */
import { formatMessage, locales, type Locale } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { languages } from "../ipc/contract/index.ts";
import { isWinAnsi, kitLabels } from "../screens/onboarding/kit.ts";
import { resolveLocale } from "./I18nProvider.tsx";

describe("language setting", () => {
  it("offers system and exactly the locales of @mosaic/i18n", () => {
    expect(languages).toEqual(["system", ...locales]);
  });

  it("picks the chosen language, or the OS one for system", () => {
    expect(resolveLocale("ja", ["de-DE"])).toBe("ja");
    expect(resolveLocale("system", ["pt-PT", "en"])).toBe("pt");
    expect(resolveLocale("system", ["zh-TW", "zh-CN"])).toBe("zh");
    expect(resolveLocale("system", ["zh-Hant-HK"])).toBe("en");
  });
});

describe("Emergency Kit labels", () => {
  const labelsIn = (locale: Locale) => kitLabels((key, values) => formatMessage(locale, key, values));

  it("keeps a language the PDF fonts can print", () => {
    expect(labelsIn("en").title).toBe(formatMessage("en", "shell.emergencyKit.title"));
    expect(labelsIn("de").title).toBe(formatMessage("de", "shell.emergencyKit.title"));
    expect(Object.values(labelsIn("de")).every(isWinAnsi)).toBe(true);
  });

  it("falls back to English when a label needs characters outside WinAnsi", () => {
    const polish = kitLabels((key) => (key === "shell.emergencyKit.title" ? "Zestaw ratunkowy łą" : "x"));
    expect(polish.title).toBe(formatMessage("en", "shell.emergencyKit.title"));
    expect(polish.footer).toBe(formatMessage("en", "shell.emergencyKit.footer"));
    const japanese = kitLabels(() => "緊急キット");
    expect(japanese.intro).toBe(formatMessage("en", "shell.emergencyKit.intro"));
  });

  it("knows WinAnsi", () => {
    expect(isWinAnsi("Größe „Kit“ – 5 €")).toBe(true);
    expect(isWinAnsi("ł")).toBe(false);
    expect(isWinAnsi("ğ")).toBe(false);
    expect(isWinAnsi("한국어")).toBe(false);
  });
});
