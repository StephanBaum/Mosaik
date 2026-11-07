/**
 * Site header (design concept §6): 64 px, sticky, transparent over the hero and solid with a
 * hairline once the page scrolls (a CSS scroll-driven animation). Below 900 px the links move
 * into a full-screen `<dialog>` menu; without JS the menu button is a link to the footer links.
 */
import type { Locale } from "@mosaic/i18n";
import { homePath, type Translate } from "./i18n.ts";
import { LogoMark } from "./Logo.tsx";

export const NAV = [
  ["web.nav.howItWorks", "#how-it-works"],
  ["web.nav.security", "#security"],
  ["web.nav.pricing", "#pricing"],
  ["web.nav.download", "#download"],
] as const;

export function NavLinks({ t, className }: { t: Translate; className: string }) {
  return (
    <ul className={className}>
      {NAV.map(([key, href]) => (
        <li key={key}>
          <a href={href}>{t(key)}</a>
        </li>
      ))}
    </ul>
  );
}

export function LanguageSwitch({ t, locale }: { t: Translate; locale: Locale }) {
  const languages = [
    ["en", "web.lang.en", "web.lang.enName"],
    ["de", "web.lang.de", "web.lang.deName"],
  ] as const;
  return (
    <ul className="lang" aria-label={t("web.lang.label")}>
      {languages.map(([code, short, name]) => (
        <li key={code}>
          <a
            href={homePath(code)}
            hrefLang={code}
            lang={code}
            aria-label={t(name)}
            aria-current={code === locale ? "page" : undefined}
          >
            {t(short)}
          </a>
        </li>
      ))}
    </ul>
  );
}

export function Header({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <header className="site-header">
      <div className="site-header__inner">
        <a className="brand" href={homePath(locale)} aria-label={t("web.nav.home")}>
          <LogoMark />
          <span className="brand__name" aria-hidden="true">
            {t("web.brand.name")}
          </span>
        </a>
        <nav className="site-nav" aria-label={t("web.nav.label")}>
          <NavLinks t={t} className="site-nav__links" />
        </nav>
        <div className="site-header__end">
          <LanguageSwitch t={t} locale={locale} />
          <a className="button button--outline button--sm site-header__cta" href="#download">
            {t("web.header.cta")}
          </a>
          <button className="menu-toggle" type="button" aria-haspopup="dialog" aria-controls="menu">
            {t("web.nav.menu")}
          </button>
          <a className="menu-toggle menu-toggle--nojs" href="#site-footer">
            {t("web.nav.menu")}
          </a>
        </div>
      </div>
      <dialog className="menu" id="menu" aria-label={t("web.nav.label")}>
        <div className="menu__bar">
          <span className="brand" aria-hidden="true">
            <LogoMark />
            <span className="brand__name">{t("web.brand.name")}</span>
          </span>
          <button className="menu__close" type="button">
            {t("web.nav.close")}
          </button>
        </div>
        <nav aria-label={t("web.nav.label")}>
          <NavLinks t={t} className="menu__links" />
        </nav>
        <LanguageSwitch t={t} locale={locale} />
      </dialog>
    </header>
  );
}
