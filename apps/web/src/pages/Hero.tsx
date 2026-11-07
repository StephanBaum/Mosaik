/**
 * Hero: decryption you can see. A full-height art panel (the island paints it; a CSS still
 * stands in without JS or WebGL2), the headline across it in difference blend, then the
 * subhead, the two CTAs and the facts strip. Every string comes from the `web` catalog. The
 * headline is one bold weight from the first frame; its only entrance is the scramble-decode.
 */
import { Art, Dither, Pixels } from "./Art.tsx";
import { Icon } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";

/** A private-use placeholder for the emphasis word while the line is split around it. */
const MARK = "\uE000";

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
      <span className="line line--1">{first}</span>{" "}
      <span className="line line--2">
        {before}
        <em className="hero__emphasis">{emphasis}</em>
        {after}
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
] as const satisfies readonly WebKey[];

const TRUST = ["web.hero.trust.spec", "web.hero.trust.threatModel", "web.hero.trust.telemetry"] as const;

export function Hero({ t }: { t: Translate }) {
  return (
    <section className="hero" aria-labelledby="hero-title">
      <Art />
      <Pixels />
      <Dither seed={3} className="dither--a" />
      <Dither seed={11} className="dither--b" />
      <div className="hero__copy">
        <p className="label hero__label">{t("web.hero.label")}</p>
        <div className="hero__title-wrap">
          <Title t={t} />
        </div>
        <div className="hero__foot">
          <p className="lead hero__subhead">{t("web.hero.subhead")}</p>
          <div className="hero__actions">
            <a className="button button--lg button--primary" href="#download">
              {t("web.hero.ctaPrimary")}
              <Icon name="download" />
            </a>
            <a className="button button--lg button--outline" href="#server-view">
              {t("web.hero.ctaSecondary")}
              <Icon name="arrowRight" className="icon--nudge" />
            </a>
          </div>
          <ul className="trust" aria-label={t("web.hero.trustLabel")}>
            {TRUST.map((key) => (
              <li key={key} className="trust__item">
                {t(key)}
              </li>
            ))}
          </ul>
        </div>
      </div>
    </section>
  );
}
