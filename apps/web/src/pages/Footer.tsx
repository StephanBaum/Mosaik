/**
 * Site footer (website v2 §1.3): about line, four link columns (Product, Get started, Free
 * resources, Compare) and a legal line with Imprint, Privacy and the language switch.
 */
import { Brand, LanguageSwitch } from "./Header.tsx";
import type { Ctx, WebKey } from "./i18n.ts";
import type { RouteId } from "./routes.ts";

type Link = readonly [label: WebKey, page: RouteId, hash?: string];
type Column = readonly [title: WebKey, links: readonly Link[]];

const COLUMNS: readonly Column[] = [
  [
    "web.footer.product",
    [
      ["web.nav.features", "features"],
      ["web.nav.security", "security"],
      ["web.nav.pricing", "pricing"],
      ["web.nav.teams", "teams"],
      ["web.footer.faq", "faq"],
    ],
  ],
  [
    "web.footer.start",
    [
      ["web.footer.download", "download"],
      ["web.footer.switch", "switch"],
      ["web.footer.verify", "verify"],
      ["web.platform.extensions", "download", "browser"],
      ["web.platform.cli", "download", "cli"],
    ],
  ],
  [
    "web.footer.tools",
    [
      ["web.footer.selfCheck", "check"],
      ["web.footer.switchKit", "switch"],
      ["web.footer.checklist", "switch", "checklist"],
    ],
  ],
  [
    "web.footer.compare",
    [
      ["web.footer.overview", "compare"],
      ["web.footer.vs1password", "vs1password"],
      ["web.footer.vsBitwarden", "vsBitwarden"],
    ],
  ],
];

export function Footer({ ctx, current }: { ctx: Ctx; current: RouteId | null }) {
  const { t } = ctx;
  return (
    <footer className="site-footer" id="site-footer" aria-label={t("web.footer.label")}>
      <div className="site-footer__inner">
        <div className="site-footer__about">
          <Brand ctx={ctx} />
          <p>{t("web.footer.about")}</p>
        </div>
        {COLUMNS.map(([title, links]) => (
          <nav key={title} className="site-footer__col" aria-label={t(title)}>
            <h2 className="site-footer__title">{t(title)}</h2>
            <ul>
              {links.map(([key, page, hash]) => (
                <li key={key}>
                  <a href={ctx.href(page, hash)}>{t(key)}</a>
                </li>
              ))}
            </ul>
          </nav>
        ))}
      </div>
      <div className="site-footer__base">
        <ul className="site-footer__legal">
          <li>
            <a href={ctx.href("imprint")}>{t("web.footer.imprint")}</a>
          </li>
          <li>
            <a href={ctx.href("privacy")}>{t("web.footer.privacy")}</a>
          </li>
        </ul>
        <LanguageSwitch ctx={ctx} current={current} long />
      </div>
    </footer>
  );
}
