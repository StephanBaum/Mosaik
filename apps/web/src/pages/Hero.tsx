/**
 * Hero: the painting full-bleed (an acrylic field with a barn on a windy evening), the copy
 * on its darker left side. The `<img>` is the painting itself and the largest paint; the baked
 * band over it is the static pixel sort (no JS, no WebGL2, reduced motion); the canvas paints
 * the live pixel sort once the island starts (src/hero). Every string comes from the `web`
 * catalog.
 */
import { HoloButton, LineButton } from "./Section.tsx";
import type { Translate, WebKey } from "./i18n.ts";

const WIDTHS = [900, 1600, 2560] as const;
/** Width over height of the painting. */
const ASPECT = 1600 / 894;
/** Portrait and narrow views crop the sides: the painting is then as wide as 100vh times its aspect. */
const SIZES = `(max-aspect-ratio: 1600/894) ${String(Math.ceil(ASPECT * 100))}vh, 100vw`;

const srcSet = (name: string, ext: string, widths: readonly number[]): string =>
  widths.map((w) => `/img/${name}-${String(w)}.${ext} ${String(w)}w`).join(", ");

/** Every catalog key the hero renders (the i18n coverage test checks them). */
export const HERO_KEYS = [
  "web.hero.eyebrow",
  "web.hero.title1",
  "web.hero.title2",
  "web.hero.title3",
  "web.hero.sub1",
  "web.hero.sub2",
  "web.hero.sub3",
  "web.hero.ctaPrimary",
  "web.hero.ctaSecondary",
  "web.hero.note",
  "web.hero.imageAlt",
] as const satisfies readonly WebKey[];

function Media({ t }: { t: Translate }) {
  return (
    <div className="hero__media">
      <picture>
        <source type="image/avif" srcSet={srcSet("hero-barn", "avif", WIDTHS)} sizes={SIZES} />
        <source type="image/webp" srcSet={srcSet("hero-barn", "webp", WIDTHS)} sizes={SIZES} />
        <img
          className="hero__img"
          src="/img/hero-barn-1600.webp"
          alt={t("web.hero.imageAlt")}
          width={1600}
          height={894}
          fetchPriority="high"
          decoding="async"
        />
      </picture>
      <picture className="hero__still" aria-hidden="true">
        <source type="image/webp" srcSet={srcSet("hero-barn-sort", "webp", [1600, 2560])} sizes={SIZES} />
        <img src="/img/hero-barn-sort-1600.webp" alt="" width={1600} height={892} decoding="async" />
      </picture>
      <canvas className="hero__canvas" aria-hidden="true" />
      <div className="hero__shade" aria-hidden="true" />
    </div>
  );
}

export function Hero({ t }: { t: Translate }) {
  return (
    <section className="hero" aria-labelledby="hero-title">
      <Media t={t} />
      <div className="hero__copy">
        <div className="hero__text">
          <p className="hero__eyebrow">{t("web.hero.eyebrow")}</p>
          <h1 id="hero-title" className="hero__title">
            <span className="hero__beat">{t("web.hero.title1")}</span> <span className="hero__beat">{t("web.hero.title2")}</span>{" "}
            <span className="hero__beat">{t("web.hero.title3")}</span>
          </h1>
          <p className="hero__sub">
            <span>{t("web.hero.sub1")}</span> <span>{t("web.hero.sub2")}</span> <span>{t("web.hero.sub3")}</span>
          </p>
          <div className="actions">
            <HoloButton href="#download" label={t("web.hero.ctaPrimary")} />
            <LineButton href="#how-it-works" label={t("web.hero.ctaSecondary")} large />
          </div>
          <p className="hero__note">{t("web.hero.note")}</p>
        </div>
      </div>
    </section>
  );
}
