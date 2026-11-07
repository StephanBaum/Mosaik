/**
 * Site footer, from the copy: logo and about line, the Product, Download and More columns and
 * the language switch. Every link points into this page; pages that do not exist yet are not
 * linked.
 */
import type { Locale } from "@mosaic/i18n";
import { LanguageSwitch } from "./Header.tsx";
import { homePath, type Translate, type WebKey } from "./i18n.ts";
import { LogoMark } from "./Logo.tsx";

type Column = readonly [title: WebKey, links: readonly (readonly [WebKey, string])[]];

const COLUMNS: readonly Column[] = [
  [
    "web.footer.product",
    [
      ["web.nav.features", "#features"],
      ["web.nav.howItWorks", "#how-it-works"],
      ["web.nav.pricing", "#pricing"],
      ["web.footer.teams", "#teams"],
      ["web.nav.faq", "#faq"],
    ],
  ],
  [
    "web.footer.download",
    [
      ["web.platform.windows", "#download"],
      ["web.platform.macos", "#download"],
      ["web.platform.linux", "#download"],
      ["web.platform.iphone", "#download"],
      ["web.platform.android", "#download"],
      ["web.platform.extensions", "#download"],
      ["web.platform.cli", "#download"],
    ],
  ],
  [
    "web.footer.more",
    [
      ["web.footer.compare", "#only-mosaic"],
      ["web.footer.verify", "#verify"],
    ],
  ],
];

export function Footer({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <footer className="site-footer" id="site-footer" aria-label={t("web.footer.label")}>
      <div className="site-footer__inner">
        <div className="site-footer__about">
          <a className="brand" href={homePath(locale)} aria-label={t("web.nav.home")}>
            <LogoMark />
            <span className="brand__name" aria-hidden="true">
              {t("web.brand.name")}
            </span>
          </a>
          <p>{t("web.footer.about")}</p>
        </div>
        {COLUMNS.map(([title, links]) => (
          <nav key={title} className="site-footer__col" aria-label={t(title)}>
            <h2 className="site-footer__title">{t(title)}</h2>
            <ul>
              {links.map(([key, href]) => (
                <li key={key}>
                  <a href={href}>{t(key)}</a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
      </div>
      <div className="site-footer__base">
        <LanguageSwitch t={t} locale={locale} long />
      </div>
    </footer>
  );
}
