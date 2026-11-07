/**
 * Renders the static pages with react-dom/server at build time (and in the dev server). No
 * React reaches the browser: the client gets plain HTML, one stylesheet and the vanilla-TS
 * islands (design concept §5).
 */
import { colors } from "@mosaic/design-tokens";
import type { Locale } from "@mosaic/i18n";
import { renderToStaticMarkup } from "react-dom/server";
import { Footer } from "./Footer.tsx";
import { Header } from "./Header.tsx";
import { Hero } from "./Hero.tsx";
import { homePath, translator, type Translate } from "./i18n.ts";
import { NotFound } from "./NotFound.tsx";
import { CSP_META } from "./security.ts";

export type PageName = "home" | "notFound";

export interface PageAssets {
  styles: readonly string[];
  scripts: readonly string[];
  /** Islands to fetch in parallel with the entry (the hero island starts right after first paint). */
  modules: readonly string[];
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

function Head({ t, request }: { t: Translate; request: PageRequest }) {
  const { locale, page, assets } = request;
  const origin = `https://${t("web.site.domain")}`;
  return (
    <head>
      <meta charSet="utf-8" />
      {assets.csp ? <meta httpEquiv="Content-Security-Policy" content={CSP_META} /> : null}
      <meta name="viewport" content="width=device-width, initial-scale=1" />
      <title>{t("web.meta.title")}</title>
      <meta name="description" content={t("web.meta.description")} />
      <meta name="referrer" content="no-referrer" />
      <meta name="color-scheme" content="dark light" />
      <meta name="theme-color" media="(prefers-color-scheme: dark)" content={colors.dark.bg} />
      <meta name="theme-color" media="(prefers-color-scheme: light)" content={colors.light.bg} />
      {page === "home" ? (
        <>
          <link rel="canonical" href={origin + homePath(locale)} />
          <link rel="alternate" hrefLang="en" href={`${origin}/`} />
          <link rel="alternate" hrefLang="de" href={`${origin}/de/`} />
          <link rel="alternate" hrefLang="x-default" href={`${origin}/`} />
        </>
      ) : (
        <meta name="robots" content="noindex" />
      )}
      <link rel="icon" href="/favicon.svg" type="image/svg+xml" />
      {assets.fonts.map((href) => (
        <link key={href} rel="preload" href={href} as="font" type="font/woff2" crossOrigin="anonymous" />
      ))}
      {assets.styles.map((href) => (
        <link key={href} rel="stylesheet" href={href} />
      ))}
      {assets.modules.map((href) => (
        <link key={href} rel="modulepreload" href={href} />
      ))}
      {assets.scripts.map((src) => (
        <script key={src} type="module" src={src} />
      ))}
    </head>
  );
}

function Page({ request }: { request: PageRequest }) {
  const { locale, page } = request;
  const t = translator(locale);
  return (
    <html lang={locale} dir="ltr">
      <Head t={t} request={request} />
      <body>
        <a className="skip-link" href="#main">
          {t("web.skipLink")}
        </a>
        <Header t={t} locale={locale} />
        <main id="main" tabIndex={-1}>
          {page === "home" ? <Hero t={t} /> : <NotFound t={t} locale={locale} />}
        </main>
        <Footer t={t} locale={locale} />
      </body>
    </html>
  );
}

export function renderPage(request: PageRequest): string {
  return `<!doctype html>${renderToStaticMarkup(<Page request={request} />)}`;
}
