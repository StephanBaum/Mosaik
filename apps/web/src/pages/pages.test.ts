/**
 * Website rules over every rendered page in both languages: i18n coverage, the claims rules,
 * no inline scripts or styles, no secret inputs and no generator, links that resolve, and the
 * comparison guard. The built output gets the same link check when `dist/` exists.
 */
import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { catalogs, locales } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { checkLinks, outPages } from "../../scripts/prerender.ts";
import { evaluate } from "../islands/selfcheck.ts";
import { levelFor } from "./checkData.ts";
import { translator, type WebKey } from "./i18n.ts";
import { renderPage, type PageAssets } from "./render.tsx";
import { fileOf, lookup, ROUTES } from "./routes.ts";
import { site } from "./site.ts";

const assets: PageAssets = { styles: ["/a.css"], scripts: ["/a.js"], fonts: [], csp: true };
const webKeys = (locale: (typeof locales)[number]): WebKey[] => Object.keys(catalogs[locale]).filter((key): key is WebKey => key.startsWith("web."));

const escaped = (text: string): string =>
  text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/'/g, "&#x27;").replace(/"/g, "&quot;");

const pages = outPages().map((p) => ({ ...p, html: renderPage({ locale: p.locale, page: p.page, assets }) }));
const publicDir = fileURLToPath(new URL("../../public/", import.meta.url));
const publicFiles = new Set(readdirSync(publicDir, { recursive: true }).map((f) => `/${String(f).replace(/\\/g, "/")}`));

/** Keys that are not page text, or only render in a state that is switched off. */
const OFF_PAGE = new RegExp(
  [
    "^web\\.site\\.domain$",
    "^web\\.notFound\\.meta\\.", // the bilingual 404 page has an English head
    "^web\\.(features\\.terminal\\.text|pricing\\.free\\.item7)$",
    "^web\\.check\\.(low|mid|high)\\.", // rendered, but split across hidden blocks
    // Rendered with the lead sentence set apart in its own element.
    "^web\\.(compare\\.ahead\\.|teamsPage\\.storage\\.(company|hosted|own)$|next\\.s[2-5]$)",
    ...(site.captureEnabled ? [] : ["^web\\.capture\\.(title|text|label|button|privacy|error)$", "^web\\.teamsPage\\.contact\\.capture$"]),
  ].join("|"),
);

describe("web catalog", () => {
  it("has the same keys in English and German", () => {
    expect(webKeys("de").sort()).toEqual(webKeys("en").sort());
  });

  it.each(locales)("%s never claims an audit for {name} and uses no forbidden words", (locale) => {
    for (const key of webKeys(locale)) {
      const text = catalogs[locale][key];
      // Others' audits may be named on the comparison pages; {name} is never called audited.
      if (!/^web\.(compare|cmp|vs1password|vsBitwarden)\./.test(key)) {
        expect(text, key).not.toMatch(/audit|geprüft von/i);
      }
      expect(text, key).not.toMatch(/\{name\}[^.]*\b(is|ist) (independently |unabhängig )?(audited|geprüft)\b/i);
      expect(text, key).not.toMatch(/bank-grade|unhackable|unknackbar/i);
      expect(text, key).not.toMatch(/cookie|banner|pre-audit/i);
      expect(text, key).not.toMatch(/\{NAME\}|\{DOMAIN\}|\{PRICE\}|\bn\/v\b/);
    }
  });

  it.each(locales)("%s hero copy stays plain", (locale) => {
    for (const key of webKeys(locale).filter((k) => k.startsWith("web.hero.") || k.startsWith("web.home.meta."))) {
      expect(catalogs[locale][key], key).not.toMatch(/roll(s|ed)? ?back|freez|fork|trust no server|vertrau keinem server/i);
    }
  });
});

describe("pages", () => {
  it("covers every route in both languages plus the 404 page", () => {
    expect(pages).toHaveLength(ROUTES.length * 2 + 1);
  });

  it.each(locales)("%s renders every web string somewhere on the site", (locale) => {
    const t = translator(locale);
    const all = pages.filter((p) => p.locale === locale || p.page === "notFound").map((p) => p.html.replace(/<\/span>( ?)<span class="sentence">/g, "$1")).join("\n");
    // Messages with their own arguments ({n}, {vendor}, {os}) are checked by the pages that fill them.
    const own = (k: WebKey): boolean => /\{(?!name\}|domain\}|cmd\})\w+/.test(catalogs[locale][k]);
    const missing = webKeys(locale).filter((k) => !OFF_PAGE.test(k) && !own(k) && !all.includes(escaped(t(k))));
    expect(missing).toEqual([]);
  });

  it.each(pages.map((p) => [p.file, p] as const))("%s keeps scripts and styles external and fills every placeholder", (_file, page) => {
    expect(page.html).toContain(`<html lang="${page.locale}" dir="ltr" data-theme="dark"`);
    expect(page.html).not.toMatch(/\sstyle=/);
    expect(page.html).not.toMatch(/<style/);
    // Only data blocks may be inline (JSON-LD); the browser never runs them.
    expect(page.html).not.toMatch(/<script(?![^>]*\bsrc=)(?![^>]*type="application\/(ld\+)?json")/);
    expect(page.html).not.toMatch(/\{(name|domain|cmd|os|vendor)\}/);
  });

  it.each(pages.map((p) => [p.file, p] as const))("%s has no secret inputs, no generator and no illustration", (_file, page) => {
    expect(page.html).not.toMatch(/type="password"/i);
    expect(page.html).not.toMatch(/password-generator|passwort-generator/i);
    expect(page.html).not.toMatch(/<input(?![^>]*type="(radio|email|hidden|text)")[^>]*>/);
    // Only the capture card may hold a text-like input, and only once capture is enabled.
    if (!site.captureEnabled) {
      expect(page.html).not.toMatch(/<input(?![^>]*type="radio")[^>]*>/);
    }
    expect(page.html).not.toMatch(/\/img\/art-/);
  });

  it("has no generator route", () => {
    expect(lookup("/password-generator/")).toBeUndefined();
    expect(lookup("/de/passwort-generator/")).toBeUndefined();
  });

  it.each(pages.map((p) => [p.file, p] as const))("%s links only to pages, files and ids that exist", (_file, page) => {
    const routePaths = new Set(ROUTES.flatMap((r) => [r.en, r.de]));
    const idsOf = (html: string): Set<string> => new Set([...html.matchAll(/\sid="([^"]+)"/g)].map((m) => m[1] ?? ""));
    for (const [, raw = ""] of page.html.matchAll(/\s(?:href|src|srcSet)="([^"]+)"/g)) {
      for (const candidate of raw.split(/,\s*/)) {
        const href = candidate.trim().split(/\s+/)[0] ?? "";
        if (!href.startsWith("/") && !href.startsWith("#")) {
          continue;
        }
        if (["/a.css", "/a.js"].includes(href)) {
          continue;
        }
        const [path = "", hash] = href.split("#");
        const target = path === "" ? page : pages.find((p) => p.file === fileOf(path));
        expect(path === "" || routePaths.has(path) || publicFiles.has(path), `${page.file}: ${href}`).toBe(true);
        if (hash !== undefined && hash !== "" && target !== undefined) {
          expect(idsOf(target.html).has(hash), `${page.file}: ${href}`).toBe(true);
        }
      }
    }
  });

  it("shows no unverified comparison cell as text", () => {
    for (const page of pages) {
      expect(page.html, page.file).not.toMatch(/\bn\/v\b/);
    }
  });

  it("opens the first question of each FAQ group only", () => {
    const faq = pages.find((p) => p.page === "faq" && p.locale === "en")?.html ?? "";
    expect(faq.match(/<details class="qa"/g)).toHaveLength(10);
    expect(faq.match(/<details class="qa" open=""/g)).toHaveLength(3);
    expect(faq).toContain('"@type":"FAQPage"');
  });
});

describe("self-check", () => {
  it("scores into three levels", () => {
    expect(levelFor(0)).toBe("low");
    expect(levelFor(3)).toBe("low");
    expect(levelFor(4)).toBe("mid");
    expect(levelFor(8)).toBe("mid");
    expect(levelFor(9)).toBe("high");
    expect(levelFor(15)).toBe("high");
  });

  it("picks at most three next steps in priority order, and a calm one when nothing applies", () => {
    const r = evaluate([
      { points: 1, tip: "review" },
      { points: 3, tip: "reuse" },
      { points: 3, tip: "twofa" },
      { points: 2, tip: "store" },
      { points: 1, tip: "many" },
    ]);
    expect(r.level).toBe("high");
    expect(r.tips).toEqual(["twofa", "reuse", "store"]);
    expect(evaluate([{ points: 0 }, { points: 1 }])).toEqual({ level: "low", tips: ["keep"] });
  });
});

const dist = fileURLToPath(new URL("../../dist/", import.meta.url));

describe.skipIf(!existsSync(`${dist}index.html`))("built output", () => {
  it("has no broken internal links", () => {
    expect(() => {
      checkLinks(dist);
    }).not.toThrow();
  });
});
