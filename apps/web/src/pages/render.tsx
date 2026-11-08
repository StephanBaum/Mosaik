/**
 * Renders the static pages with react-dom/server at build time (and in the dev server). No
 * React reaches the browser: the client gets plain HTML, one stylesheet and the vanilla-TS
 * islands a page marks with `data-island` (design concept §5, website v2 §1.4).
 */
import type { Locale } from "@mosaic/i18n";
import type { ReactNode } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Footer } from "./Footer.tsx";
import { Header } from "./Header.tsx";
import { context, type Ctx, type WebKey } from "./i18n.ts";
import { pathOf, route, type Route, type RouteId } from "./routes.ts";
import { cspMetaFor } from "./security.ts";
import { VIEWS, type View } from "./views/index.ts";
import { NotFound } from "./views/small.tsx";

export type PageName = RouteId | "notFound";

export interface PageAssets {
  styles: readonly string[];
  scripts: readonly string[];
  /** Fonts to preload (DM Sans latin only). */
  fonts: readonly string[];
  /** Adds the CSP meta tag (production pages; the dev server injects inline styles). */
  csp: boolean;
}

export interface PageRequest {
  locale: Locale;
  page: PageName;
  assets: PageAssets;
}

/** The ink colour, for the browser chrome (base.css `--ink`). */
const THEME_COLOR = "#0C0B14";

/** JSON for a `<script type="application/ld+json">` data block, safe inside HTML. */
function ldJson(data: unknown): string {
  return JSON.stringify(data).replace(/</g, "\\u003c");
}

function breadcrumbs(ctx: Ctx, r: Route, origin: string): unknown {
  if (r.parent === undefined) {
    return null;
  }
  const items: [RouteId, WebKey][] = [
    [r.parent, `web.${route(r.parent).meta}.meta.title` as WebKey],
    [r.id, `web.${r.meta}.meta.title` as WebKey],
  ];
  return {
    "@context": "https://schema.org",
    "@type": "BreadcrumbList",
    itemListElement: items.map(([id, key], index) => ({
      "@type": "ListItem",
      position: index + 1,
      name: ctx.t(key),
      item: origin + ctx.href(id),
    })),
  };
}

function Head({ ctx, r, view, assets }: { ctx: Ctx; r: Route | null; view: View | null; assets: PageAssets }) {
  const { t, locale } = ctx;
  const origin = `https://${t("web.site.domain")}`;
  const title = r === null ? t("web.notFound.meta.title") : t(`web.${r.meta}.meta.title` as WebKey);
  const description = r === null ? t("web.notFound.meta.description") : t(`web.${r.meta}.meta.description` as WebKey);
  const data = r === null ? [] : [breadcrumbs(ctx, r, origin), view?.jsonLd?.(ctx) ?? null].filter((d) => d !== null);
  return (
    <head>
      <meta charSet="utf-8" />
      {assets.csp ? <meta httpEquiv="Content-Security-Policy" content={cspMetaFor(r)} /> : null}
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <title>{title}</title>
      <meta name="description" content={description} />
      <meta name="referrer" content="no-referrer" />
      <meta name="color-scheme" content="dark" />
      <meta name="theme-color" content={THEME_COLOR} />
      {r !== null && r.index ? (
        <>
          <link rel="canonical" href={origin + pathOf(r.id, locale)} />
          <link rel="alternate" hrefLang="en" href={origin + r.en} />
          <link rel="alternate" hrefLang="de" href={origin + r.de} />
          <link rel="alternate" hrefLang="x-default" href={origin + r.en} />
        </>
      ) : (
        <meta name="robots" content="noindex" />
      )}
      <meta property="og:type" content="website" />
      <meta property="og:title" content={title} />
      <meta property="og:description" content={description} />
      <meta property="og:image" content={`${origin}/img/app-vault-${locale}-1280.webp`} />
      <meta property="og:locale" content={locale === "de" ? "de_DE" : "en_US"} />
      <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
      {assets.fonts.map((href) => (
        <link key={href} rel="preload" href={href} as="font" type="font/woff2" crossOrigin="anonymous" />
      ))}
      {assets.styles.map((href) => (
        <link key={href} rel="stylesheet" href={href} />
      ))}
      {assets.scripts.map((src) => (
        <script key={src} type="module" src={src} />
      ))}
      {data.map((d, i) => (
        <script key={i} type="application/ld+json" dangerouslySetInnerHTML={{ __html: ldJson(d) }} />
      ))}
    </head>
  );
}

function Document({ request }: { request: PageRequest }) {
  const { locale, page, assets } = request;
  const ctx = context(locale);
  const r = page === "notFound" ? null : route(page);
  const view = r === null ? null : VIEWS[r.id];
  let main: ReactNode;
  if (view === null) {
    main = <NotFound />;
  } else {
    const Body = view.Page;
    main = <Body ctx={ctx} />;
  }
  return (
    <html lang={locale} dir="ltr" data-theme="dark">
      <Head ctx={ctx} r={r} view={view} assets={assets} />
      <body data-tint={r?.tint ?? "teal"} data-page={page}>
        <a className="skip-link" href="#main">
          {ctx.t("web.skipLink")}
        </a>
        <Header ctx={ctx} current={r?.id ?? null} />
        <main id="main" tabIndex={-1}>
          {main}
        </main>
        <Footer ctx={ctx} current={r?.id ?? null} />
      </body>
    </html>
  );
}

export function renderPage(request: PageRequest): string {
  return `<!doctype html>${renderToStaticMarkup(<Document request={request} />)}`;
}
