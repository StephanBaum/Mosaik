/**
 * Hero (design concept §1 chapter 1, §3 M1, §6). The copy is the rewrite in
 * marketing/landing-copy.*.md; every string comes from the `web` catalog. The headline is two
 * short sentences, each its own masked line and its own beat in the load sequence; the
 * emphasis word settles from weight 400 to 720 as the mosaic lands.
 */
import { GlyphSprite, Icon } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";
import { Mosaic } from "./Mosaic.tsx";

const MARK = "";

/** Measure class of the headline from its longest line, so 3 to 8 words and German fit. */
export function measure(lines: readonly string[]): "short" | "medium" | "long" {
  const longest = Math.max(...lines.map((line) => line.length));
  return longest <= 17 ? "short" : longest <= 24 ? "medium" : "long";
}

/** The two headline lines; the second split around its emphasis word. */
export function titleParts(t: Translate): { first: string; before: string; emphasis: string; after: string } {
  const [before = "", after = ""] = t("web.hero.titleLine2", { emphasis: MARK }).split(MARK);
  return { first: t("web.hero.titleLine1"), before, emphasis: t("web.hero.titleEmphasis"), after };
}

function Title({ t }: { t: Translate }) {
  const { first, before, emphasis, after } = titleParts(t);
  return (
    <h1 id="hero-title" className="display hero__title" data-measure={measure([first, before + emphasis + after])}>
      <span className="line line--1">
        <span className="line__inner">{first}</span>
      </span>{" "}
      <span className="line line--2">
        <span className="line__inner">
          {before}
          <em className="hero__emphasis">{emphasis}</em>
          {after}
        </span>
      </span>
    </h1>
  );
}

/** Every catalog key the hero renders (the i18n coverage test checks them). */
export const HERO_KEYS = [
  "web.hero.label",
  "web.hero.titleLine1",
  "web.hero.titleLine2",
  "web.hero.titleEmphasis",
  "web.hero.subhead",
  "web.hero.ctaPrimary",
  "web.hero.ctaSecondary",
  "web.hero.trustLabel",
  "web.hero.trust.spec",
  "web.hero.trust.threatModel",
  "web.hero.trust.telemetry",
  "web.hero.trust.audit",
] as const satisfies readonly WebKey[];

const TRUST = [
  "web.hero.trust.spec",
  "web.hero.trust.threatModel",
  "web.hero.trust.telemetry",
  "web.hero.trust.audit",
] as const;

export function Hero({ t }: { t: Translate }) {
  return (
    <section className="hero" aria-labelledby="hero-title">
      <GlyphSprite />
      <div className="hero__copy">
        <p className="label hero__label">{t("web.hero.label")}</p>
        <div className="hero__title-wrap">
          <Title t={t} />
        </div>
        <p className="lead hero__subhead">{t("web.hero.subhead")}</p>
        <div className="hero__actions">
          <a className="button button--lg holo" href="#download">
            <canvas className="holo__foil" aria-hidden="true" />
            <span className="holo__label">
              {t("web.hero.ctaPrimary")}
              <Icon name="download" />
            </span>
          </a>
          <a className="button button--lg button--outline" href="#server-view">
            {t("web.hero.ctaSecondary")}
            <Icon name="arrowRight" className="icon--nudge" />
          </a>
        </div>
        <Mosaic />
        <ul className="trust" aria-label={t("web.hero.trustLabel")}>
          {TRUST.map((key) => (
            <li key={key} className={key === "web.hero.trust.audit" ? "trust__item trust__item--audit" : "trust__item"}>
              {t(key)}
            </li>
          ))}
        </ul>
      </div>
    </section>
  );
}
