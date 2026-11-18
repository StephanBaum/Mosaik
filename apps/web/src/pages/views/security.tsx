/**
 * Security (website v2 §5.3): where vaults live, two keys, lying-server detection, what we
 * see. Visuals are the product: the unlock screen with the sync warning, the three steps with
 * real screens, and what a stolen server holds.
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon } from "../Icon.tsx";
import { Shot, SHOTS, type ShotSpec } from "../Shot.tsx";
import { ArrowLink, Eyebrow, Head, HoloButton, LineButton, ProductFrame, Section } from "../ui.tsx";
import { WhereStories } from "./home.tsx";
import { SyncDemo } from "../demos/SyncDemo.tsx";
import { CipherMock, WarnMock } from "./mocks.tsx";
import { PageCta, PageHero } from "./parts.tsx";

const STEPS = [
  [SHOTS.unlock, "web.shot.unlock", "web.how.step1.number", "web.how.step1.title", "web.how.step1.text"],
  [SHOTS.sync, "web.how.syncAlt", "web.how.step2.number", "web.how.step2.title", "web.how.step2.text"],
  [null, "web.how.warningAlt", "web.how.step3.number", "web.how.step3.title", "web.how.step3.text"],
] as const satisfies readonly (readonly [ShotSpec | null, WebKey, WebKey, WebKey, WebKey])[];

/** Three steps, each beside its picture; the third picture is the sync warning demo. */
function HowItWorks({ ctx }: { ctx: Ctx }) {
  const { t, locale } = ctx;
  return (
    <Section id="how-it-works" className="how">
      <div className="wrap">
        <Head id="how-it-works" eyebrow={t("web.how.eyebrow")} title={t("web.security.how")} intro={t("web.how.intro")} />
        <ol className="how-steps">
          {STEPS.map(([shot, alt, number, title, text]) => (
            <li key={number} className="how-step" data-reveal="">
              <div className="how-step__visual">
                {shot === null ? (
                  <SyncDemo ctx={ctx} />
                ) : (
                  <div className="frame">
                    <Shot spec={shot} locale={locale} alt={t(alt)} />
                  </div>
                )}
              </div>
              <div className="how-step__copy">
                <span className="step-card__n" aria-hidden="true">
                  {t(number)}
                </span>
                <h3 className="how-step__title">{t(title)}</h3>
                <p className="how-step__text">{t(text)}</p>
              </div>
            </li>
          ))}
        </ol>
        <aside className="technical card" data-reveal="">
          <p className="technical__label">{t("web.how.technicalLabel")}</p>
          <p>{t("web.how.technical")}</p>
          <p>{t("web.only.technical")}</p>
        </aside>
      </div>
    </Section>
  );
}

function StolenServer({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="stolen" className="stolen">
      <div className="wrap overlap">
        <div className="overlap__visual" data-reveal="">
          <CipherMock ctx={ctx} />
        </div>
        <div className="overlap__text card" data-reveal="">
          <Eyebrow>{t("web.security.stolen.eyebrow")}</Eyebrow>
          <h2 id="stolen-title" className="h2">
            {t("web.security.stolen.title")}
          </h2>
          <p className="lead">{t("web.faq.a4")}</p>
          <p className="lead">{t("web.security.stolen.add")}</p>
        </div>
      </div>
    </Section>
  );
}

function WhatWeSee({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const yes = ["web.security.see.yes1", "web.security.see.yes2", "web.security.see.yes3"] as const;
  const no = ["web.security.see.no1", "web.security.see.no2", "web.security.see.no3"] as const;
  return (
    <Section id="what-we-see" className="see">
      <div className="wrap">
        <h2 id="what-we-see-title" className="h2 see__title" data-reveal="">
          {t("web.security.see.title")}
        </h2>
        <div className="see__card" data-reveal="">
          <div className="see__col">
            <h3 className="see__label">{t("web.security.see.yes")}</h3>
            <ul>
              {yes.map((k) => (
                <li key={k}>
                  <Icon name="check" />
                  <span>{t(k)}</span>
                </li>
              ))}
            </ul>
          </div>
          <div className="see__col see__col--no">
            <h3 className="see__label">{t("web.security.see.no")}</h3>
            <ul>
              {no.map((k) => (
                <li key={k}>
                  <Icon name="eyeOff" />
                  <span>{t(k)}</span>
                </li>
              ))}
            </ul>
          </div>
          <p className="see__foot">{t("web.security.see.footer")}</p>
        </div>
      </div>
    </Section>
  );
}

function ChallengeBlock({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="challenge" className="challenge-block">
      <div className="wrap">
        <div className="card" data-reveal="">
          <h2 id="challenge-title" className="h2">
            {t("web.security.challenge.title")}
          </h2>
          <p className="lead">{t("web.security.challenge.text")}</p>
          <ArrowLink href={ctx.href("challenge")} label={t("web.security.challenge.link")} />
        </div>
      </div>
    </Section>
  );
}

export function Security({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.security.eyebrow")}
        title={t("web.security.title")}
        sub={t("web.security.sub")}
        actions={
          <>
            <HoloButton href={ctx.href("download")} label={t("web.hero.ctaPrimary")} />
            <LineButton href={ctx.href("compare")} label={t("web.security.cta.secondary")} large />
          </>
        }
        visual={
          <div className="pair">
            <ProductFrame ctx={ctx} shot={SHOTS.unlock} alt={t("web.shot.unlock")} eager className="pair__main" />
            <WarnMock ctx={ctx} className="pair__float" />
          </div>
        }
      />
      <WhereStories ctx={ctx} link={false} />
      <HowItWorks ctx={ctx} />
      <StolenServer ctx={ctx} />
      <WhatWeSee ctx={ctx} />
      <ChallengeBlock ctx={ctx} />
      <PageCta ctx={ctx} title={t("web.download.title")} text={t("web.download.body")} secondary={[ctx.href("compare"), t("web.security.cta.secondary")]} />
    </>
  );
}
