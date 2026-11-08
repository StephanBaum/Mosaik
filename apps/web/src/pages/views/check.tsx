/**
 * The self-check "How exposed are you?" (website v2 §2.1): seven multiple-choice questions,
 * no text inputs, no secrets. The island (islands/selfcheck.ts) scores the ticked answers in
 * the page and reveals one of three results with the matching next steps; nothing is stored or
 * sent (`connect-src 'none'`, `form-action 'none'`, no form element at all).
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { ArrowLink, CaptureCard, Eyebrow, HoloButton, Section } from "../ui.tsx";
import { PageHero } from "./parts.tsx";
import { LEVELS, QUESTIONS, TIPS } from "../checkData.ts";

export function SelfCheck({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero ctx={ctx} eyebrow={t("web.check.eyebrow")} title={t("web.check.title")} sub={t("web.check.sub")} className="hero--check" />
      <Section id="questions" className="check-section" label={t("web.check.formLabel")}>
        <div className="wrap check-wrap">
          <div className="check" data-island="selfcheck">
            <ol className="check__list">
              {QUESTIONS.map(([n, options]) => (
                <li key={n} className="check__q">
                  <fieldset>
                    <legend>
                      <span className="check__n">{String(n).padStart(2, "0")}</span>
                      {t(`web.check.q${String(n)}` as WebKey)}
                    </legend>
                    <div className="check__opts">
                      {options.map(([suffix, points, tip]) => (
                        <label key={suffix} className="check__opt">
                          <input type="radio" name={`q${String(n)}`} value={suffix} data-points={points} data-tip={tip} />
                          <span>{t(`web.check.q${String(n)}.${suffix}` as WebKey)}</span>
                        </label>
                      ))}
                    </div>
                  </fieldset>
                </li>
              ))}
            </ol>
            <div className="check__submit">
              <button type="button" className="button button--lg button--solid check__go" disabled>
                {t("web.check.submit")}
              </button>
              <p className="check__hint" aria-live="polite">
                {t("web.check.incomplete")}
              </p>
              <p className="check__noscript">{t("web.check.noscript")}</p>
            </div>
            <div className="check__result" hidden tabIndex={-1}>
              <Eyebrow>{t("web.check.result.eyebrow")}</Eyebrow>
              {LEVELS.map(([level]) => (
                <div key={level} className="check__level" data-level={level} hidden>
                  <h2 className="h2">{t(`web.check.${level}.title` as WebKey)}</h2>
                  <p className="lead">{t(`web.check.${level}.text` as WebKey)}</p>
                </div>
              ))}
              <h3 className="check__next">{t("web.check.next")}</h3>
              <ol className="check__tips">
                {TIPS.map((tip) => (
                  <li key={tip} data-tip={tip} hidden>
                    {t(`web.check.tip.${tip}` as WebKey)}
                  </li>
                ))}
              </ol>
              <div className="actions">
                <HoloButton href={ctx.href("download")} label={t("web.download.ctaPrimary")} />
                <ArrowLink href={ctx.href("switch")} label={t("web.check.kit")} />
              </div>
              <p className="fineprint">{t("web.check.private")}</p>
              <button type="button" className="button button--line check__again">
                {t("web.check.restart")}
              </button>
            </div>
          </div>
        </div>
      </Section>
      <Section id="kit" className="kit" label={t("web.capture.soon.title")}>
        <div className="wrap kit__wrap">
          <CaptureCard ctx={ctx} src="check" />
        </div>
      </Section>
    </>
  );
}
