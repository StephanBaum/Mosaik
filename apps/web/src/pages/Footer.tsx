/** Site footer (design concept §6 chapter 10, minimal for now): links, language, the pre-audit line. */
import type { Locale } from "@mosaic/i18n";
import { LanguageSwitch, NavLinks } from "./Header.tsx";
import type { Translate } from "./i18n.ts";

export function Footer({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <footer className="site-footer" id="site-footer" aria-label={t("web.footer.label")}>
      <div className="site-footer__inner">
        <nav aria-label={t("web.nav.label")}>
          <NavLinks t={t} className="site-footer__links" />
        </nav>
        <LanguageSwitch t={t} locale={locale} />
        <p className="label site-footer__note">
          <span>{t("web.footer.preAudit")}</span> <span>{t("web.footer.noCookies")}</span>
        </p>
      </div>
      <p className="site-footer__wordmark" aria-hidden="true">
        {t("web.brand.name")}
      </p>
    </footer>
  );
}
