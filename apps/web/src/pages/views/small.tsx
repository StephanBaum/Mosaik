/** Small pages: thanks, confirmed, imprint, privacy and the bilingual 404 page. */
import { context, type Ctx, type WebKey } from "../i18n.ts";
import { ArrowLink, IconTile } from "../ui.tsx";

/** Thanks and Confirmed (website v2 §5.6): the double opt-in steps. */
export function Thanks({ ctx, confirmed = false }: { ctx: Ctx; confirmed?: boolean }) {
  const { t } = ctx;
  return (
    <section className="hero hero--small" aria-labelledby="hero-title">
      <div className="wrap small__grid">
        <IconTile name="mail" className="icon-tile--xl" />
        <div className="small__copy">
          <h1 id="hero-title" className="h1">
            {t(confirmed ? "web.confirmed.title" : "web.thanks.title")}
          </h1>
          <p className="lead">{t(confirmed ? "web.confirmed.text" : "web.thanks.text")}</p>
          <ArrowLink href={ctx.href("download")} label={t("web.thanks.link")} />
        </div>
      </div>
    </section>
  );
}

/**
 * Imprint and Privacy: plain text pages, no art (website v2 §5.11). The owner supplies the
 * texts; until then the pages show the headings and the empty fields, never a made-up address,
 * and the production build refuses to ship (site.legalReady).
 */
export function Legal({ ctx, kind }: { ctx: Ctx; kind: "imprint" | "privacy" }) {
  const { t } = ctx;
  const fields = ["web.legal.responsible", "web.legal.address", "web.legal.contact"] as const satisfies readonly WebKey[];
  return (
    <section className="hero hero--legal" aria-labelledby="hero-title">
      <div className="wrap legal">
        <h1 id="hero-title" className="h1">
          {t(kind === "imprint" ? "web.imprint.title" : "web.privacy.title")}
        </h1>
        <dl className="legal__fields">
          {fields.map((k) => (
            <div key={k}>
              <dt>{t(k)}</dt>
              <dd>–</dd>
            </div>
          ))}
        </dl>
      </div>
    </section>
  );
}

/** The bilingual 404 page (website v2 §5.11). */
export function NotFound() {
  const en = context("en");
  const de = context("de");
  return (
    <section className="hero hero--lost" aria-labelledby="hero-title">
      <div className="wrap">
        <p className="lost__code" aria-hidden="true">404</p>
        <div className="lost">
          {[en, de].map((ctx) => (
            <div key={ctx.locale} className="lost__lang" lang={ctx.locale}>
              <h1 id={ctx.locale === "en" ? "hero-title" : undefined} className="h1">
                {ctx.t("web.notFound.title")}
              </h1>
              <p className="lead">{ctx.t("web.notFound.body")}</p>
              <ul className="lost__links">
                <li>
                  <a href={ctx.href("home")}>{ctx.t("web.notFound.home")}</a>
                </li>
                <li>
                  <a href={ctx.href("check")}>{ctx.t("web.notFound.check")}</a>
                </li>
                <li>
                  <a href={ctx.href("download")}>{ctx.t("web.notFound.download")}</a>
                </li>
              </ul>
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
