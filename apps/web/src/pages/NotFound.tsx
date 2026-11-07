/** The 404 page (design concept §6). The path scramble and the small mosaic follow in a later step. */
import type { Locale } from "@mosaic/i18n";
import { homePath, type Translate } from "./i18n.ts";

export function NotFound({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <section className="not-found" aria-labelledby="not-found-title">
      <p className="label">404</p>
      <h1 id="not-found-title" className="display not-found__title">
        {t("web.notFound.title")}
      </h1>
      <p className="lead">{t("web.notFound.body")}</p>
      <p>
        <a className="text-link" href={homePath(locale)}>
          {t("web.notFound.back")}
        </a>
      </p>
    </section>
  );
}
