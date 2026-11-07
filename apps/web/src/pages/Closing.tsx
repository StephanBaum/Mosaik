/**
 * The closing sections: the FAQ (native `<details>` accordion, first item open) and the
 * download section (holo primary action, the platform list with the visitor's system first,
 * a sorted crop of the painting's sky behind it).
 */
import { Icon } from "./Icon.tsx";
import type { Translate, WebKey } from "./i18n.ts";
import { PLATFORM_KEYS } from "./Product.tsx";
import { Head, HoloButton, LineButton, Section } from "./Section.tsx";

const FAQ = [
  ["web.faq.q1", "web.faq.a1"],
  ["web.faq.q2", "web.faq.a2"],
  ["web.faq.q3", "web.faq.a3"],
  ["web.faq.q4", "web.faq.a4"],
  ["web.faq.q5", "web.faq.a5"],
  ["web.faq.q6", "web.faq.a6"],
  ["web.faq.q7", "web.faq.a7"],
  ["web.faq.q8", "web.faq.a8"],
  ["web.faq.q9", "web.faq.a9"],
  ["web.faq.q10", "web.faq.a10"],
] as const satisfies readonly (readonly [WebKey, WebKey])[];

export function Faq({ t }: { t: Translate }) {
  return (
    <Section id="faq" className="faq">
      <div className="split split--top">
        <Head id="faq" eyebrow={t("web.faq.eyebrow")} title={t("web.faq.title")} className="faq__head" />
        <div className="faq__list split__aside">
          {FAQ.map(([question, answer], index) => (
            <details key={question} className="qa" open={index === 0}>
              <summary className="qa__q">
                <h3 className="qa__title">{t(question)}</h3>
                <Icon name="plus" className="qa__icon" />
              </summary>
              <p className="qa__a">{t(answer)}</p>
            </details>
          ))}
        </div>
      </div>
    </Section>
  );
}

export function Download({ t }: { t: Translate }) {
  return (
    <Section id="download" className="download">
      <div className="download__sky" aria-hidden="true">
        <picture>
          <source type="image/avif" srcSet="/img/sky-sorted-1600.avif 1600w, /img/sky-sorted-2560.avif 2560w" sizes="100vw" />
          <source type="image/webp" srcSet="/img/sky-sorted-1600.webp 1600w, /img/sky-sorted-2560.webp 2560w" sizes="100vw" />
          <img src="/img/sky-sorted-1600.webp" alt="" width={1600} height={376} loading="lazy" decoding="async" />
        </picture>
      </div>
      <div className="download__body">
        <Head id="download" eyebrow={t("web.download.eyebrow")} title={t("web.download.title")} intro={t("web.download.body")} className="head--center" />
        <div className="actions actions--center">
          <HoloButton href="#platforms" label={t("web.download.ctaPrimary")} />
          <LineButton href="#pricing" label={t("web.download.ctaSecondary")} large />
        </div>
        <ul className="platforms" id="platforms" aria-label={t("web.download.platformsLabel")}>
          {PLATFORM_KEYS.map(([id, key, icon]) => (
            <li key={id} className="platform" data-platform={id}>
              <Icon name={icon} className="platform__icon" />
              <span>{t(key)}</span>
            </li>
          ))}
        </ul>
        <p className="download__note" id="verify">
          {t("web.download.note")}
        </p>
      </div>
    </Section>
  );
}
