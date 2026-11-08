/**
 * The story half of the page: the everyday problem, where vaults live (with the two incidents
 * and a sorted detail of the painting), and how Mosaic works in three steps.
 */
import type { Locale } from "@mosaic/i18n";
import { Icon, type IconName } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";
import { Head, Point, Section } from "./Section.tsx";
import { Shot, SHOTS } from "./Shot.tsx";

const PROBLEMS: readonly (readonly [IconName, WebKey, WebKey])[] = [
  ["door", "web.problem.leak.title", "web.problem.leak.text"],
  ["copy", "web.problem.fake.title", "web.problem.fake.text"],
  ["timer", "web.problem.codes.title", "web.problem.codes.text"],
  ["devices", "web.problem.passkeys.title", "web.problem.passkeys.text"],
];

export function Problem({ t }: { t: Translate }) {
  return (
    <Section id="problem">
      <div className="split">
        <Head id="problem" eyebrow={t("web.problem.eyebrow")} title={t("web.problem.title")} />
        <p className="body-lg split__aside">{t("web.problem.body")}</p>
      </div>
      <ul className="points points--4">
        {PROBLEMS.map(([icon, title, text]) => (
          <Point key={title} icon={icon} title={t(title)} text={t(text)} />
        ))}
      </ul>
      <p className="statement">{t("web.problem.closing")}</p>
    </Section>
  );
}

const STORIES = [
  ["web.where.story1.label", "web.where.story1.title", "web.where.story1.text"],
  ["web.where.story2.label", "web.where.story2.title", "web.where.story2.text"],
] as const satisfies readonly (readonly [WebKey, WebKey, WebKey])[];

export function WhereItLives({ t }: { t: Translate }) {
  return (
    <Section id="where-it-lives" className="where">
      <div className="where__lead">
        <figure className="where__art">
          <picture>
            <source
              type="image/avif"
              srcSet="/img/story-barn-night-sort-800.avif 800w, /img/story-barn-night-sort-1200.avif 1200w"
              sizes="(min-width: 900px) 40vw, 100vw"
            />
            <source
              type="image/webp"
              srcSet="/img/story-barn-night-sort-800.webp 800w, /img/story-barn-night-sort-1200.webp 1200w"
              sizes="(min-width: 900px) 40vw, 100vw"
            />
            <img
              src="/img/story-barn-night-sort-800.webp"
              alt={t("web.where.imageAlt")}
              width={800}
              height={994}
              loading="lazy"
              decoding="async"
            />
          </picture>
        </figure>
        <div className="where__text">
          <Head id="where-it-lives" eyebrow={t("web.where.eyebrow")} title={t("web.where.title")} />
          <p className="body-lg">{t("web.where.body")}</p>
        </div>
      </div>
      <ol className="stories">
        {STORIES.map(([label, title, text]) => (
          <li key={label} className="story">
            <p className="story__year">{t(label)}</p>
            <h3 className="story__title">{t(title)}</h3>
            <p className="story__text">{t(text)}</p>
          </li>
        ))}
      </ol>
      <p className="statement">{t("web.where.closing")}</p>
      <p className="fineprint">{t("web.where.sources")}</p>
    </Section>
  );
}

const STEPS = [
  ["web.how.step1.number", "web.how.step1.title", "web.how.step1.text"],
  ["web.how.step2.number", "web.how.step2.title", "web.how.step2.text"],
  ["web.how.step3.number", "web.how.step3.title", "web.how.step3.text"],
] as const satisfies readonly (readonly [WebKey, WebKey, WebKey])[];

/** Step 3's visual: the app's warning in calm amber, labelled as a sample. */
function WarningSample({ t }: { t: Translate }) {
  return (
    <div className="sample" role="img" aria-label={t("web.how.warningAlt")}>
      <span className="sample__tag" aria-hidden="true">
        {t("web.how.sample")}
      </span>
      <div className="sample__banner" aria-hidden="true">
        <Icon name="warning" className="sample__icon" />
        <p>{t("web.how.warning")}</p>
      </div>
    </div>
  );
}

export function HowItWorks({ t, locale }: { t: Translate; locale: Locale }) {
  return (
    <Section id="how-it-works">
      <Head id="how-it-works" eyebrow={t("web.how.eyebrow")} title={t("web.how.title")} intro={t("web.how.intro")} className="head--wide" />
      <ol className="steps">
        {STEPS.map(([number, title, text], index) => (
          <li key={number} className="step">
            <p className="step__number" aria-hidden="true">
              {t(number)}
            </p>
            <h3 className="step__title">{t(title)}</h3>
            <p className="step__text">{t(text)}</p>
            {index === STEPS.length - 1 ? <WarningSample t={t} /> : null}
          </li>
        ))}
      </ol>
      <div className="how__proof">
        <Shot spec={SHOTS.sync} locale={locale} alt={t("web.how.syncAlt")} className="how__shot" />
        <aside className="technical" aria-label={t("web.how.technicalLabel")}>
          <p className="technical__label">{t("web.how.technicalLabel")}</p>
          <p className="technical__text">{t("web.how.technical")}</p>
        </aside>
      </div>
    </Section>
  );
}
