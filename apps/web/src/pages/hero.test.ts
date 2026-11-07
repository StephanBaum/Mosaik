/**
 * i18n coverage of the hero and the page shell: every hero string comes from the `web`
 * catalog in both locales, the rendered pages contain them, and the output keeps the
 * privacy and claims rules (no inline scripts or styles, no forbidden words).
 */
import { catalogs, locales } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { HERO_KEYS, measure, titleParts } from "./Hero.tsx";
import { translator, type WebKey } from "./i18n.ts";
import { renderPage, type PageAssets } from "./render.tsx";

const assets: PageAssets = { styles: ["/a.css"], scripts: ["/a.js"], modules: ["/b.js"], fonts: [], csp: true };
const webKeys = (locale: (typeof locales)[number]): WebKey[] =>
  Object.keys(catalogs[locale]).filter((key): key is WebKey => key.startsWith("web."));

/** Text as react-dom/server escapes it inside elements. */
const escaped = (text: string): string =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/'/g, "&#x27;").replace(/"/g, "&quot;");

describe("web catalog", () => {
  it("has the same keys in English and German", () => {
    expect(webKeys("de").sort()).toEqual(webKeys("en").sort());
  });

  it.each(locales)("%s never claims an audit or uses forbidden words", (locale) => {
    for (const key of webKeys(locale)) {
      expect(catalogs[locale][key], key).not.toMatch(/audited|bank-grade|unhackable|geprüft von|unknackbar/i);
      expect(catalogs[locale][key], key).not.toMatch(/\{NAME\}|\{DOMAIN\}/);
    }
  });

  it.each(locales)("%s hero copy stays plain: no protocol jargon", (locale) => {
    for (const key of webKeys(locale).filter((k) => k.startsWith("web.hero.") || k.startsWith("web.meta."))) {
      expect(catalogs[locale][key], key).not.toMatch(/roll(s|ed)? ?back|freez|fork|equivocat|spaltet|eingefroren/i);
    }
  });
});

describe.each(locales)("hero (%s)", (locale) => {
  const t = translator(locale);
  const html = renderPage({ locale, page: "home", assets });

  it("renders every hero key from the catalog", () => {
    for (const key of HERO_KEYS) {
      expect(catalogs[locale][key].trim(), key).not.toBe("");
    }
    const { first, before, emphasis, after } = titleParts(t);
    for (const text of [first, before, emphasis, after, t("web.hero.label"), t("web.hero.subhead")]) {
      expect(html).toContain(escaped(text));
    }
    for (const key of ["web.hero.ctaPrimary", "web.hero.ctaSecondary", "web.hero.trust.audit"] as const) {
      expect(html).toContain(escaped(t(key)));
    }
  });

  it("fills in the product name and leaves no ICU syntax", () => {
    expect(html).toContain("Mosaic");
    expect(html).not.toMatch(/\{(name|domain|emphasis)\}/);
  });

  it("marks the emphasis word inside the second line", () => {
    expect(html).toMatch(/<span class="line line--2"><span class="line__inner">[^<]*<em class="hero__emphasis">/);
  });

  it("sets the language and keeps scripts and styles external", () => {
    expect(html).toContain(`<html lang="${locale}"`);
    expect(html).not.toMatch(/\sstyle=/);
    expect(html).not.toMatch(/<script(?![^>]*\bsrc=)/);
    expect(html).not.toMatch(/<style/);
  });
});

describe("headline measure", () => {
  it("sizes the headline by its longest line", () => {
    expect(measure(["Trust no server.", "Not even ours."])).toBe("short");
    expect(measure(["Vertrau keinem Server.", "Nicht mal unserem."])).toBe("medium");
    expect(measure(["Every secret. Every device.", "No blind trust."])).toBe("long");
  });
});
