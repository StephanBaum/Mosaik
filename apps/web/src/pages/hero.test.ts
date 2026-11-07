/**
 * i18n coverage and page rules: every `web` string exists in both locales and appears on the
 * rendered page, the sections come in the copy's order, every in-page link has its target, and
 * the output keeps the privacy and claims rules (no inline scripts or styles, no forbidden
 * words, no meta lines about cookies or what is not done yet).
 */
import { catalogs, locales } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { HERO_KEYS } from "./Hero.tsx";
import { translator, type WebKey } from "./i18n.ts";
import { renderPage, type PageAssets } from "./render.tsx";

const assets: PageAssets = { styles: ["/a.css"], scripts: ["/a.js"], modules: ["/b.js"], fonts: [], csp: true };
const webKeys = (locale: (typeof locales)[number]): WebKey[] =>
  Object.keys(catalogs[locale]).filter((key): key is WebKey => key.startsWith("web."));

/** Text as react-dom/server escapes it inside elements and attributes. */
const escaped = (text: string): string =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/'/g, "&#x27;").replace(/"/g, "&quot;");

/** Keys that are not page text: the 404 page, the placeholder domain, and messages split around `{cmd}`. */
const OFF_PAGE = /^web\.(notFound\.|site\.domain$)|^web\.(features\.terminal\.text|pricing\.free\.item7)$/;

const SECTIONS = [
  "problem",
  "where-it-lives",
  "how-it-works",
  "features",
  "only-mosaic",
  "switch",
  "pricing",
  "teams",
  "faq",
  "download",
];

describe("web catalog", () => {
  it("has the same keys in English and German", () => {
    expect(webKeys("de").sort()).toEqual(webKeys("en").sort());
  });

  it.each(locales)("%s never claims an audit or uses forbidden words", (locale) => {
    for (const key of webKeys(locale)) {
      expect(catalogs[locale][key], key).not.toMatch(/audit|bank-grade|unhackable|geprüft von|unknackbar/i);
      // The page sells: no meta lines about cookies, banners or what is not done yet.
      expect(catalogs[locale][key], key).not.toMatch(/cookie|banner|not yet|noch kein|pre-audit/i);
      expect(catalogs[locale][key], key).not.toMatch(/\{NAME\}|\{DOMAIN\}|\{PRICE\}/);
    }
  });

  it.each(locales)("%s hero copy stays plain: no protocol jargon", (locale) => {
    for (const key of webKeys(locale).filter((k) => k.startsWith("web.hero.") || k.startsWith("web.meta."))) {
      expect(catalogs[locale][key], key).not.toMatch(/roll(s|ed)? ?back|freez|fork|equivocat|spaltet|eingefroren/i);
    }
  });

  it.each(locales)("%s does not lead with the rejected hero lines", (locale) => {
    for (const key of webKeys(locale).filter((k) => k.startsWith("web.hero.") || k.startsWith("web.meta."))) {
      expect(catalogs[locale][key], key).not.toMatch(/trust no server|vertrau keinem server/i);
    }
  });
});

describe.each(locales)("page (%s)", (locale) => {
  const t = translator(locale);
  const html = renderPage({ locale, page: "home", assets });

  it("renders every web string of the catalog", () => {
    // Titles are split into one span per sentence; join them back for the comparison.
    const text = html.replace(/<\/span>( ?)<span class="sentence">/g, "$1");
    for (const key of webKeys(locale).filter((k) => !OFF_PAGE.test(k))) {
      expect(text, key).toContain(escaped(t(key)));
    }
    for (const key of HERO_KEYS) {
      expect(catalogs[locale][key].trim(), key).not.toBe("");
    }
  });

  it("sets the command as code inside its sentences", () => {
    expect(html.match(/<code>mosaic<\/code>/g)).toHaveLength(2);
  });

  it("fills in the product name and leaves no ICU syntax", () => {
    expect(html).toContain("Mosaic");
    expect(html).not.toMatch(/\{(name|domain|cmd)\}/);
  });

  it("keeps the sections in the copy's order", () => {
    const positions = SECTIONS.map((id) => html.indexOf(`<section id="${id}"`));
    expect(positions.every((p) => p > 0)).toBe(true);
    expect([...positions].sort((a, b) => a - b)).toEqual(positions);
  });

  it("links only to targets on the page", () => {
    const ids = new Set([...html.matchAll(/\sid="([^"]+)"/g)].map((m) => m[1]));
    for (const [, target] of html.matchAll(/href="#([^"]*)"/g)) {
      expect(ids.has(target), `#${target ?? ""}`).toBe(true);
    }
  });

  it("ships the painting as the hero's high-priority image, AVIF first", () => {
    expect(html).toMatch(/<source type="image\/avif" srcSet="\/img\/hero-barn-900\.avif 900w[^"]*"[^>]*><source type="image\/webp"/);
    expect(html).toMatch(/<img class="hero__img"[^>]*fetchPriority="high"|<img class="hero__img"[^>]*fetchpriority="high"/);
    expect(html).toContain(`alt="${escaped(t("web.hero.imageAlt"))}"`);
    expect(html).toContain('<canvas class="hero__canvas" aria-hidden="true">');
  });

  it("puts the holo button on the hero and the download section", () => {
    expect(html.match(/class="button button--lg holo"/g)).toHaveLength(2);
    expect(html.match(/<canvas class="holo__foil" aria-hidden="true">/g)).toHaveLength(2);
  });

  it("opens the first question of the FAQ only", () => {
    expect(html.match(/<details class="qa"/g)).toHaveLength(10);
    expect(html.match(/<details class="qa" open=""/g)).toHaveLength(1);
  });

  it("is dark only, sets the language and keeps scripts and styles external", () => {
    expect(html).toContain(`<html lang="${locale}" dir="ltr" data-theme="dark"`);
    expect(html).toContain('<meta name="color-scheme" content="dark"/>');
    expect(html).not.toMatch(/\sstyle=/);
    expect(html).not.toMatch(/<script(?![^>]*\bsrc=)/);
    expect(html).not.toMatch(/<style/);
  });
});
