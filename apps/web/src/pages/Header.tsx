/**
 * Site header (website v2 §1.3): logo, five page links, the language switch to the counterpart
 * page, and the solid Download pill (never holo). Fixed and dark-blurred once the page scrolls.
 * Below 900 px the links move into a full-screen `<dialog>`; the Download pill stays visible.
 * Without JS the menu button is a link to the footer links.
 */
import type { Locale } from "@mosaic/i18n";
import { Icon } from "./Icon.tsx";
import type { Ctx, WebKey } from "./i18n.ts";
import { LogoMark } from "./Logo.tsx";
import { pathOf, type RouteId } from "./routes.ts";

export const NAV: readonly (readonly [WebKey, RouteId])[] = [
  ["web.nav.features", "features"],
  ["web.nav.security", "security"],
  ["web.nav.compare", "compare"],
  ["web.nav.pricing", "pricing"],
  ["web.nav.teams", "teams"],
];

/** The section of the site a page belongs to, for `aria-current` in the header. */
function sectionOf(id: RouteId | null): RouteId | null {
  if (id === "vs1password" || id === "vsBitwarden") {
    return "compare";
  }
  return id;
}

function NavLinks({ ctx, current, className }: { ctx: Ctx; current: RouteId | null; className: string }) {
  const active = sectionOf(current);
  return (
    <ul className={className}>
      {NAV.map(([key, id]) => (
        <li key={id}>
          <a href={ctx.href(id)} aria-current={active === id ? "page" : undefined}>
            {ctx.t(key)}
          </a>
        </li>
      ))}
    </ul>
  );
}

/** EN / DE, each linking to the same page in that language (the home page on the 404 page). */
export function LanguageSwitch({ ctx, current, long = false }: { ctx: Ctx; current: RouteId | null; long?: boolean }) {
  const languages = [
    ["en", "web.lang.en", "web.lang.enName"],
    ["de", "web.lang.de", "web.lang.deName"],
  ] as const satisfies readonly (readonly [Locale, WebKey, WebKey])[];
  return (
    <ul className={long ? "lang lang--long" : "lang"} aria-label={ctx.t("web.lang.label")}>
      {languages.map(([code, short, name]) => (
        <li key={code}>
          <a
            href={pathOf(current ?? "home", code)}
            hrefLang={code}
            lang={code}
            aria-label={long ? undefined : ctx.t(name)}
            aria-current={code === ctx.locale && current !== null ? "page" : undefined}
          >
            {ctx.t(long ? name : short)}
          </a>
        </li>
      ))}
    </ul>
  );
}

export function Brand({ ctx }: { ctx: Ctx }) {
  return (
    <a className="brand" href={ctx.href("home")} aria-label={ctx.t("web.nav.home")}>
      <LogoMark />
      <span className="brand__name" aria-hidden="true">
        {ctx.t("web.brand.name")}
      </span>
    </a>
  );
}

export function Header({ ctx, current }: { ctx: Ctx; current: RouteId | null }) {
  const { t } = ctx;
  return (
    <header className="site-header">
      <div className="site-header__inner">
        <Brand ctx={ctx} />
        <nav className="site-nav" aria-label={t("web.nav.label")}>
          <NavLinks ctx={ctx} current={current} className="site-nav__links" />
        </nav>
        <div className="site-header__end">
          <LanguageSwitch ctx={ctx} current={current} />
          <a className="button button--sm button--solid site-header__cta" href={ctx.href("download")}>
            {t("web.nav.cta")}
          </a>
          <button className="menu-toggle" type="button" aria-haspopup="dialog" aria-controls="menu" aria-label={t("web.nav.open")}>
            <Icon name="menu" />
          </button>
          <a className="menu-toggle menu-toggle--nojs" href="#site-footer" aria-label={t("web.nav.open")}>
            <Icon name="menu" />
          </a>
        </div>
      </div>
      <dialog className="menu" id="menu" aria-label={t("web.nav.label")}>
        <div className="menu__bar">
          <span className="brand" aria-hidden="true">
            <LogoMark />
            <span className="brand__name">{t("web.brand.name")}</span>
          </span>
          <button className="menu__close" type="button" aria-label={t("web.nav.close")}>
            <Icon name="close" />
          </button>
        </div>
        <nav aria-label={t("web.nav.label")}>
          <NavLinks ctx={ctx} current={current} className="menu__links" />
        </nav>
        <div className="menu__foot">
          <a className="button button--lg button--solid" href={ctx.href("download")}>
            {t("web.nav.cta")}
          </a>
          <LanguageSwitch ctx={ctx} current={current} />
        </div>
      </dialog>
    </header>
  );
}
