/** The 404 page: a short line and the way back. */
import type { Locale } from "@mosaic/i18n";
import { homePath, type Translate } from "./i18n.ts";
import { LineButton } from "./Section.tsx";

export function NotFound({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <section className="not-found" aria-labelledby="not-found-title">
      <p className="eyebrow">404</p>
      <h1 id="not-found-title" className="not-found__title">
        {t("web.notFound.title")}
      </h1>
      <p className="body-lg">{t("web.notFound.body")}</p>
      <div>
        <LineButton href={homePath(locale)} label={t("web.notFound.back")} large />
      </div>
    </section>
  );
}
