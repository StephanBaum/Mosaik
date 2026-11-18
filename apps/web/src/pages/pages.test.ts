/**
 * Website rules over every rendered page in both languages: i18n coverage, the claims rules,
 * no inline scripts or styles, no secret inputs and no generator, links that resolve, and the
 * comparison guard. The built output gets the same link check when `dist/` exists.
 */
import { existsSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { catalogs } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { checkLinks, outPages } from "../../scripts/prerender.ts";
import { evaluate } from "../islands/selfcheck.ts";
import { levelFor } from "./checkData.ts";
import { translator, type WebKey } from "./i18n.ts";
import { renderPage, type PageAssets } from "./render.tsx";
import { fileOf, lookup, ROUTES, SITE_LOCALES } from "./routes.ts";
import { LINKS, site } from "./site.ts";

const assets: PageAssets = { styles: ["/a.css"], scripts: ["/a.js"], fonts: [], csp: true };
const webKeys = (locale: (typeof SITE_LOCALES)[number]): WebKey[] => Object.keys(catalogs[locale]).filter((key): key is WebKey => key.startsWith("web."));

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
    // Only the line of the current challenge state renders.
    site.challengeOpen ? "^web\\.challenge\\.soon$" : "^web\\.challenge\\.open$",
    ...(site.captureEnabled ? [] : ["^web\\.capture\\.(title|text|label|button|privacy|error)$", "^web\\.teamsPage\\.contact\\.capture$"]),
  ].join("|"),
);

describe("web catalog", () => {
  it("has the same keys in English and German", () => {
    expect(webKeys("de").sort()).toEqual(webKeys("en").sort());
  });

  it.each(SITE_LOCALES)("%s never claims an audit for {name} and uses no forbidden words", (locale) => {
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

  it.each(SITE_LOCALES)("%s hero copy stays plain", (locale) => {
    for (const key of webKeys(locale).filter((k) => k.startsWith("web.hero.") || k.startsWith("web.home.meta."))) {
      expect(catalogs[locale][key], key).not.toMatch(/roll(s|ed)? ?back|freez|fork|trust no server|vertrau keinem server/i);
    }
  });
});

describe("pages", () => {
  it("covers every route in both languages plus the 404 page", () => {
    expect(pages).toHaveLength(ROUTES.length * 2 + 1);
  });

  it.each(SITE_LOCALES)("%s renders every web string somewhere on the site", (locale) => {
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

  it.each(pages.map((p) => [p.file, p] as const))("%s renders without reveal classes and with one motion toggle", (_file, page) => {
    expect(page.html).not.toMatch(/reveal-pending|reveal-in/);
    expect(page.html.match(/data-motion-toggle/g)).toHaveLength(1);
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

describe("visual system", () => {
  it.each(pages.map((p) => [p.file, p] as const))("%s paints the field still and the transit hint once", (_f, p) => {
    expect(p.html.match(/class="field-still"/g)).toHaveLength(1);
    expect(p.html.match(/class="transit-hint"/g)).toHaveLength(1);
  });

  it.each(pages.map((p) => [p.file, p] as const))("%s marks nav links and opens external links safely", (_f, p) => {
    expect(p.html).toMatch(/<nav class="site-nav"[\s\S]*?data-nav=""/);
    for (const m of p.html.matchAll(/<a [^>]*href="https:\/\/[^"]+"[^>]*>/g)) {
      expect(m[0]).toContain('rel="noopener noreferrer"');
    }
  });

  it.each(["en", "de"] as const)("home %s tells the prototype story in order", (locale) => {
    const html = pages.find((p) => p.page === "home" && p.locale === locale)?.html ?? "";
    const order = [
      'class="hero hero--home"',
      "data-showcase",
      'id="reasons-title"',
      'id="where-title"',
      'id="features-title"',
      'id="agent-title"',
      'id="tools-title"',
      'id="pricing-title"',
      'id="get-title"',
    ];
    const at = order.map((s) => html.indexOf(s));
    expect(at.every((v, i) => v >= 0 && (i === 0 || v > (at[i - 1] ?? 0)))).toBe(true);
    expect(html).toContain("<code>mosaic://work/openai/api-key</code>");
    expect(html.match(/fetchpriority="high"/gi)).toHaveLength(1);
    expect(html.match(/class="button button--lg holo"/g)?.length).toBeLessThanOrEqual(2);
    expect(html).toMatch(/<h1 [^>]*data-decode=""[^>]*data-island="decode"/);
  });

  it("renders every demo in its final frame, marked for the demos island", () => {
    const html = pages.find((p) => p.page === "home" && p.locale === "en")?.html ?? "";
    for (const name of ["fill", "agent", "import", "phone"]) {
      expect(html).toMatch(new RegExp(`data-island="demos" data-demo="${name}" [^>]*role="img" aria-label=`));
    }
    expect(html).toContain('data-bad="1ogin.yourbank.com"');
  });
});

describe("shared hero", () => {
  it.each(pages.map((p) => [p.file, p] as const))("%s uses the shared hero and none of the retired patterns", (_f, p) => {
    expect(p.html).toMatch(/<section class="hero hero--(home|page)/);
    expect(p.html).not.toMatch(/class="[^"]*\b(stage|p[1-9]-|foil)\b/);
  });
});

describe("features, security, download and next", () => {
  const htmlOf = (page: string, locale: "en" | "de"): string => pages.find((p) => p.page === page && p.locale === locale)?.html ?? "";

  it("download jumps to the platforms without JS", () => {
    const html = htmlOf("download", "en");
    expect(html).toMatch(/<a [^>]*href="#platforms"[^>]*data-jump=""/);
    expect(html).toMatch(/id="platforms" tabindex="-1"/);
  });

  it("security explains in steps and points to the challenge", () => {
    const html = htmlOf("security", "de");
    expect(html.match(/class="how-step"/g)).toHaveLength(3);
    expect(html).toContain('href="/de/sicherheit/challenge/"');
    expect(html).not.toContain('data-island="steps"');
    expect(html).toMatch(/data-demo="sync"/);
  });

  it("next steps animate from a finished list", () => {
    expect(htmlOf("next", "en")).toMatch(/data-demo="nextSteps"/);
  });

  it("features replace the terminal with the demo and add programs, Android linking and folders", () => {
    const html = htmlOf("features", "en");
    expect(html).toMatch(/data-island="demos" data-demo="cli"/);
    expect(html).not.toContain("terminal__line\" aria-hidden=\"true\"><span class=\"terminal__prompt\">$</span> mosaic<");
    for (const key of ["web.features.programs.title", "web.features.androidLink.title", "web.features.folders.title"] as const) {
      expect(html).toContain(escaped(translator("en")(key)));
    }
  });
});

describe("report and challenge pages", () => {
  it("routes the report and challenge pages in both languages", () => {
    expect(lookup("/report/")?.route.id).toBe("report");
    expect(lookup("/de/melden/")?.route.id).toBe("report");
    expect(lookup("/security/challenge/")?.route.id).toBe("challenge");
    expect(lookup("/de/sicherheit/challenge/")?.route.id).toBe("challenge");
  });

  it("sends reports to GitHub, never to a form", () => {
    for (const locale of SITE_LOCALES) {
      const html = pages.find((p) => p.page === "report" && p.locale === locale)?.html ?? "";
      expect(html).toContain(`href="${LINKS.issues}"`);
      expect(html).toContain(`href="${LINKS.advisory}"`);
      expect(html).toContain('rel="noopener noreferrer"');
      expect(html).not.toMatch(/<form|<input|<textarea/);
    }
  });

  it("lists seven challenge targets and the rules links", () => {
    const html = pages.find((p) => p.page === "challenge" && p.locale === "de")?.html ?? "";
    for (const id of ["C1", "C2", "C3", "C4", "C5", "C6", "C7"]) {
      expect(html).toContain(`>${id}<`);
    }
    expect(html).toContain(`href="${LINKS.rules}"`);
    expect(html).toContain(`href="${LINKS.targets}"`);
    expect(html).not.toMatch(/<form|<input|<textarea/);
  });

  it("links the challenge from the security page and the footer, and takes no input", () => {
    const security = pages.find((p) => p.page === "security" && p.locale === "en")?.html ?? "";
    expect(security).toContain('href="/security/challenge/"');
    const home = pages.find((p) => p.page === "home" && p.locale === "de")?.html ?? "";
    expect(home).toContain('href="/de/melden/"');
    expect(home).toContain('href="/de/sicherheit/challenge/"');
  });

  it("keeps the CSP closed on both pages", () => {
    for (const id of ["report", "challenge"] as const) {
      const html = pages.find((p) => p.page === id && p.locale === "en")?.html ?? "";
      expect(html).toContain("form-action &#x27;none&#x27;");
      expect(html).toContain("connect-src &#x27;none&#x27;");
    }
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
