/**
 * Switch guide and switch kit (website v2 §2.1, §5.7): export, import, clean up, per source,
 * over the real import preview. The capture card offers the kit as a PDF plus five mails
 * (coming soon until the endpoint exists). The checklist prints on its own (print stylesheet).
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { SHOTS } from "../Shot.tsx";
import { CaptureCard, Eyebrow, Head, HoloButton, IconTile, ProductFrame, Questions, Section, StepCard } from "../ui.tsx";
import { PageHero } from "./parts.tsx";

export const SOURCES_SWITCH = [
  ["1password", "web.switchPage.src.1password", "web.switchPage.src.1password.text"],
  ["bitwarden", "web.switchPage.src.bitwarden", "web.switchPage.src.bitwarden.text"],
  ["lastpass", "web.switchPage.src.lastpass", "web.switchPage.src.lastpass.text"],
  ["keepass", "web.switchPage.src.keepass", "web.switchPage.src.keepass.text"],
  ["enpass", "web.switchPage.src.enpass", "web.switchPage.src.enpass.text"],
  ["browsers", "web.switchPage.src.browsers", "web.switchPage.src.browsers.text"],
  ["other", "web.switchPage.src.other", "web.switchPage.src.other.text"],
] as const satisfies readonly (readonly [string, WebKey, WebKey])[];

export function Switch({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const steps = [
    ["01", "web.switchPage.step1.title", "web.switchPage.step1.text"],
    ["02", "web.switchPage.step2.title", "web.switchPage.step2.text"],
    ["03", "web.switchPage.step3.title", "web.switchPage.step3.text"],
  ] as const;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.switch.eyebrow")}
        title={t("web.switch.title")}
        sub={t("web.switchPage.sub")}
        className="hero--switch"
        actions={<HoloButton href={ctx.href("download")} label={t("web.hero.ctaPrimary")} />}
        visual={
          <div className="next-stage">
            <ProductFrame ctx={ctx} shot={SHOTS.import} alt={t("web.shot.import")} eager />
            <ol className="float-steps float-steps--three">
              {steps.map(([n, title, text]) => (
                <StepCard key={n} n={n} title={t(title)}>
                  <p>{t(text)}</p>
                </StepCard>
              ))}
            </ol>
          </div>
        }
      />

      <PickSource ctx={ctx} />

      <Section id="safety" className="safety">
        <div className="wrap">
          <div className="card wide-card wide-card--warn">
            <IconTile name="warning" />
            <h2 id="safety-title" className="h2">
              {t("web.switchPage.safety.title")}
            </h2>
            <p className="lead">{t("web.switchPage.safety.text")}</p>
          </div>
        </div>
      </Section>

      <Section id="passkeys" className="passkeys-note">
        <div className="wrap">
          <div className="card wide-card">
            <IconTile name="key" />
            <h2 id="passkeys-title" className="h3">
              {t("web.switchPage.passkeys.title")}
            </h2>
            <p>{t("web.switchPage.passkeys.text")}</p>
            <p className="fineprint">
              <strong>{t("web.switch.leave.title")}</strong> {t("web.switch.leave.text")}
            </p>
          </div>
        </div>
      </Section>

      <Checklist ctx={ctx} />

      <Section id="kit" className="kit" label={t("web.capture.soon.title")}>
        <div className="wrap kit__wrap">
          <CaptureCard ctx={ctx} src="switch" />
        </div>
      </Section>

      <SwitchFaq ctx={ctx} />
    </>
  );
}

function PickSource({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="sources" className="sources-pick">
      <div className="wrap">
        <Head id="sources" eyebrow={t("web.switch.bring.title")} title={t("web.switchPage.pick")} intro={t("web.switch.bring.text")} />
        <nav className="seg" aria-labelledby="sources-title">
          <ul>
            {SOURCES_SWITCH.map(([id, name]) => (
              <li key={id}>
                <a href={`#from-${id}`}>{t(name)}</a>
              </li>
            ))}
          </ul>
        </nav>
        <ul className="source-panels">
          {SOURCES_SWITCH.map(([id, name, text]) => (
            <li key={id} id={`from-${id}`} className="card source-panel">
              <h3 className="h3">{t(name)}</h3>
              <p>{t(text)}</p>
            </li>
          ))}
        </ul>
      </div>
    </Section>
  );
}

function Checklist({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const checklist = [
    "web.switchPage.checklist.c1",
    "web.switchPage.checklist.c2",
    "web.switchPage.checklist.c3",
    "web.switchPage.checklist.c4",
    "web.switchPage.checklist.c5",
    "web.switchPage.checklist.c6",
    "web.switchPage.checklist.c7",
    "web.switchPage.checklist.c8",
  ] as const;
  return (
    <Section id="checklist" className="checklist">
      <div className="wrap checklist__grid">
        <div>
          <Eyebrow>{t("web.switch.eyebrow")}</Eyebrow>
          <h2 id="checklist-title" className="h2">
            {t("web.switchPage.checklist.title")}
          </h2>
          <button className="button button--line checklist__print" type="button" data-island="print" data-print="" hidden>
            {t("web.switchPage.checklist.print")}
          </button>
        </div>
        <ol className="checklist__list">
          {checklist.map((k) => (
            <li key={k}>
              <span className="checklist__box" aria-hidden="true" />
              <span>{t(k)}</span>
            </li>
          ))}
        </ol>
      </div>
    </Section>
  );
}

function SwitchFaq({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="switch-faq" className="switch-faq">
      <div className="wrap pricing-faq__grid">
        <h2 id="switch-faq-title" className="h2">
          {t("web.switchPage.faq.title")}
        </h2>
        <Questions
          t={t}
          items={[
            ["web.switchPage.faq.q1", "web.switchPage.faq.a1"],
            ["web.switchPage.faq.q2", "web.switchPage.faq.a2"],
            ["web.switchPage.faq.q3", "web.switchPage.faq.a3"],
          ]}
        />
      </div>
    </Section>
  );
}
