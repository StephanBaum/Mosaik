/**
 * Site header: logo, section links, the language switch and a small download button. It sits
 * transparent over the hero and turns dark and blurred once the page scrolls (a CSS
 * scroll-driven animation). Below 900 px the links move into a full-screen `<dialog>` menu;
 * without JS the menu button is a link to the footer links.
 */
import type { Locale } from "@mosaic/i18n";
import { Icon } from "./Icon.tsx";
import { homePath, type Translate } from "./i18n.ts";
import { LogoMark } from "./Logo.tsx";

export const NAV = [
  ["web.nav.howItWorks", "#how-it-works"],
  ["web.nav.features", "#features"],
  ["web.nav.pricing", "#pricing"],
  ["web.nav.teams", "#teams"],
  ["web.nav.faq", "#faq"],
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

export function LanguageSwitch({ t, locale, long = false }: { t: Translate; locale: Locale; long?: boolean }) {
  const languages = [
    ["en", "web.lang.en", "web.lang.enName"],
    ["de", "web.lang.de", "web.lang.deName"],
  ] as const;
  return (
    <ul className={long ? "lang lang--long" : "lang"} aria-label={t("web.lang.label")}>
      {languages.map(([code, short, name]) => (
        <li key={code}>
          <a
            href={homePath(code)}
            hrefLang={code}
            lang={code}
            aria-label={long ? undefined : t(name)}
            aria-current={code === locale ? "page" : undefined}
          >
            {t(long ? name : short)}
          </a>
        </li>
      ))}
    </ul>
  );
}

function Brand({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <a className="brand" href={homePath(locale)} aria-label={t("web.nav.home")}>
      <LogoMark />
      <span className="brand__name" aria-hidden="true">
        {t("web.brand.name")}
      </span>
    </a>
  );
}

export function Header({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <header className="site-header">
      <div className="site-header__inner">
        <Brand t={t} locale={locale} />
        <nav className="site-nav" aria-label={t("web.nav.label")}>
          <NavLinks t={t} className="site-nav__links" />
        </nav>
        <div className="site-header__end">
          <LanguageSwitch t={t} locale={locale} />
          <a className="button button--sm button--solid site-header__cta" href="#download">
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
          <NavLinks t={t} className="menu__links" />
        </nav>
        <div className="menu__foot">
          <a className="button button--lg button--solid" href="#download">
            {t("web.nav.cta")}
          </a>
          <LanguageSwitch t={t} locale={locale} />
        </div>
      </dialog>
    </header>
  );
}
