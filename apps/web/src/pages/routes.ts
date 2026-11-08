/**
 * The single route table of the website (website v2 §1.4). The prerenderer writes every route
 * twice (EN, DE) to `<path>/index.html`, the dev server answers from it, and the head, the
 * language switch, the sitemap, `_headers` and `_redirects` are generated from it.
 */
import type { Locale } from "@mosaic/i18n";

export type RouteId =
  | "home"
  | "features"
  | "security"
  | "compare"
  | "vs1password"
  | "vsBitwarden"
  | "pricing"
  | "teams"
  | "download"
  | "next"
  | "verify"
  | "switch"
  | "check"
  | "faq"
  | "thanks"
  | "confirmed"
  | "imprint"
  | "privacy";

/** Client islands a page may use; main.ts imports only the ones a page marks with `data-island`. */
export type IslandId = "holo" | "downloads" | "foil" | "selfcheck" | "steps" | "print";

/** Section and page tints (website v2 §3 P7), mapped to colours in base.css. */
export type Tint = "cobalt" | "tangerine" | "teal" | "violet" | "yellow" | "coral" | "mint" | "sky";

/** The catalog prefix of a page's `meta.title` and `meta.description`. */
export type MetaKey =
  | "home"
  | "features"
  | "security"
  | "compare"
  | "vs1password"
  | "vsBitwarden"
  | "pricingPage"
  | "teamsPage"
  | "downloadPage"
  | "next"
  | "verify"
  | "switchPage"
  | "check"
  | "faqPage"
  | "thanks"
  | "confirmed"
  | "imprint"
  | "privacy";

export interface Route {
  id: RouteId;
  en: string;
  de: string;
  /** Listed in the sitemap and indexable. */
  index: boolean;
  meta: MetaKey;
  tint: Tint;
  /** Second-level pages get breadcrumbs (website v2 §1.3). */
  parent?: RouteId;
  /** Renders the capture form once capture is enabled (adds `form-action 'self'`). */
  capture?: boolean;
}

export const ROUTES: readonly Route[] = [
  { id: "home", en: "/", de: "/de/", index: true, meta: "home", tint: "cobalt" },
  { id: "features", en: "/features/", de: "/de/funktionen/", index: true, meta: "features", tint: "violet" },
  { id: "security", en: "/security/", de: "/de/sicherheit/", index: true, meta: "security", tint: "teal" },
  { id: "compare", en: "/compare/", de: "/de/vergleich/", index: true, meta: "compare", tint: "yellow", capture: true },
  {
    id: "vs1password",
    en: "/compare/1password/",
    de: "/de/vergleich/1password/",
    index: true,
    meta: "vs1password",
    tint: "yellow",
    parent: "compare",
    capture: true,
  },
  {
    id: "vsBitwarden",
    en: "/compare/bitwarden/",
    de: "/de/vergleich/bitwarden/",
    index: true,
    meta: "vsBitwarden",
    tint: "yellow",
    parent: "compare",
    capture: true,
  },
  { id: "pricing", en: "/pricing/", de: "/de/preise/", index: true, meta: "pricingPage", tint: "tangerine" },
  { id: "teams", en: "/teams/", de: "/de/teams/", index: true, meta: "teamsPage", tint: "mint", capture: true },
  { id: "download", en: "/download/", de: "/de/download/", index: true, meta: "downloadPage", tint: "violet" },
  { id: "next", en: "/download/next/", de: "/de/download/weiter/", index: false, meta: "next", tint: "cobalt", parent: "download" },
  {
    id: "verify",
    en: "/download/verify/",
    de: "/de/download/pruefen/",
    index: true,
    meta: "verify",
    tint: "violet",
    parent: "download",
  },
  { id: "switch", en: "/switch/", de: "/de/umzug/", index: true, meta: "switchPage", tint: "sky", capture: true },
  { id: "check", en: "/check/", de: "/de/check/", index: true, meta: "check", tint: "violet", capture: true },
  { id: "faq", en: "/faq/", de: "/de/fragen/", index: true, meta: "faqPage", tint: "teal" },
  { id: "thanks", en: "/thanks/", de: "/de/danke/", index: false, meta: "thanks", tint: "coral" },
  { id: "confirmed", en: "/confirmed/", de: "/de/bestaetigt/", index: false, meta: "confirmed", tint: "coral" },
  { id: "imprint", en: "/imprint/", de: "/de/impressum/", index: true, meta: "imprint", tint: "cobalt" },
  { id: "privacy", en: "/privacy/", de: "/de/datenschutz/", index: true, meta: "privacy", tint: "cobalt" },
];

/** The one bilingual 404 page. */
export const NOT_FOUND_PATH = "/404.html";

export function route(id: RouteId): Route {
  const found = ROUTES.find((r) => r.id === id);
  if (found === undefined) {
    throw new Error(`unknown route ${id}`);
  }
  return found;
}

/** The public path of a page in a locale, always with a trailing slash. */
export function pathOf(id: RouteId, locale: Locale): string {
  return route(id)[locale];
}

/** The output file of a path: `/de/preise/` → `de/preise/index.html`. */
export function fileOf(path: string): string {
  return `${path.replace(/^\//, "")}index.html`;
}

/** The route and locale a request path belongs to (slash-less paths and `index.html` included). */
export function lookup(path: string): { route: Route; locale: Locale } | undefined {
  const clean = path.split(/[?#]/)[0] ?? "/";
  const normal = clean.replace(/index\.html$/, "").replace(/([^/])$/, "$1/");
  for (const r of ROUTES) {
    if (r.en === normal) {
      return { route: r, locale: "en" };
    }
    if (r.de === normal) {
      return { route: r, locale: "de" };
    }
  }
  return undefined;
}
