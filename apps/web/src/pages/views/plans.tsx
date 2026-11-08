/**
 * Pricing (website v2 §5.9): the plan cards are the hero visual. Teams (§5.10): a members list
 * built from the app's tokens over the real sync settings, the five team features, where team
 * vaults live, setup and contact.
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon, type IconName } from "../Icon.tsx";
import { SHOTS } from "../Shot.tsx";
import { site } from "../site.ts";
import { CaptureCard, Head, HoloButton, IconTile, LineButton, ProductFrame, Questions, Section, StepCard } from "../ui.tsx";
import { PlanCards } from "./home.tsx";
import { MembersMock } from "./mocks.tsx";
import { PageCta, PageHero } from "./parts.tsx";

export function Pricing({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.pricing.eyebrow")}
        title={t("web.pricing.title")}
        sub={t("web.pricing.intro")}
        actions={<HoloButton href={ctx.href("download")} label={t("web.hero.ctaPrimary")} />}
        visual={
          <>
            <PlanCards ctx={ctx} full />
            <p className="plans-note">{t("web.pricing.note")}</p>
          </>
        }
      />
      <Section id="pricing-faq" className="pricing-faq">
        <div className="wrap split">
          <h2 id="pricing-faq-title" className="h2">
            {t("web.pricingPage.faqTitle")}
          </h2>
          <Questions
            t={t}
            items={[
              ["web.faq.q6", "web.faq.a6"],
              ["web.faq.q5", "web.faq.a5"],
              ["web.faq.q2", "web.faq.a2"],
            ]}
          />
        </div>
      </Section>
      <Section id="for-teams" className="teams-link" label={t("web.nav.teams")}>
        <div className="wrap">
          <a className="link-card" href={ctx.href("teams")}>
            <IconTile name="users" />
            <span className="link-card__text">{t("web.pricingPage.teamsLink")}</span>
            <Icon name="arrowRight" className="icon--nudge" />
          </a>
        </div>
      </Section>
      <PageCta ctx={ctx} title={t("web.download.title")} text={t("web.download.body")} />
    </>
  );
}

const TEAM_ITEMS = [
  ["layers", "web.teams.shared.title", "web.teams.shared.text"],
  ["shield", "web.teams.verify.title", "web.teams.verify.text"],
  ["userMinus", "web.teams.goodbye.title", "web.teams.goodbye.text"],
  ["cloud", "web.teams.company.title", "web.teams.company.text"],
  ["buoy", "web.teams.emergency.title", "web.teams.emergency.text"],
] as const satisfies readonly (readonly [IconName, WebKey, WebKey])[];

export function Teams({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const mail = `mailto:${site.contactUser}@${t("web.site.domain")}`;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.teams.eyebrow")}
        title={t("web.teams.title")}
        sub={t("web.teams.body")}
        actions={
          <>
            <HoloButton href={ctx.href("download")} label={t("web.teams.cta")} />
            <LineButton href={mail} label={t("web.teamsPage.contact.button")} large />
          </>
        }
        visual={
          <div className="pair pair--members">
            <ProductFrame ctx={ctx} shot={SHOTS.sync} alt={t("web.how.syncAlt")} eager className="pair__main" />
            <MembersMock ctx={ctx} className="pair__float" />
          </div>
        }
      />
      <Section id="what" className="team-bento" label={t("web.teams.eyebrow")}>
        <div className="wrap">
          <ul className="ruled ruled--3 ruled--grid">
            {TEAM_ITEMS.map(([icon, title, text]) => (
              <li key={title} className="ruled__item">
                <IconTile name={icon} />
                <h3 className="ruled__title">{t(title)}</h3>
                <p className="ruled__text">{t(text)}</p>
              </li>
            ))}
          </ul>
        </div>
      </Section>
      <Storage ctx={ctx} />
      <Setup ctx={ctx} />
      <Section id="contact" className="contact">
        <div className="wrap contact__grid">
          <div className="contact__copy">
            <h2 id="contact-title" className="h2">
              {t("web.teamsPage.contact.title")}
            </h2>
            <p className="lead">{t("web.teamsPage.contact.text")}</p>
            <LineButton href={mail} label={t("web.teamsPage.contact.button")} large />
          </div>
          <CaptureCard ctx={ctx} src="teams" list="teams" title={t("web.teamsPage.contact.capture")} />
        </div>
      </Section>
    </>
  );
}

function Storage({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const storage = [
    ["cloud", "web.teamsPage.storage.company"],
    ["vault", "web.teamsPage.storage.hosted"],
    ["monitor", "web.teamsPage.storage.own"],
  ] as const satisfies readonly (readonly [IconName, WebKey])[];
  return (
    <Section id="storage" className="storage">
      <div className="wrap">
        <Head id="storage" title={t("web.teamsPage.storage.title")} />
        <ul className="storage__list">
          {storage.map(([icon, k]) => {
            const [lead = "", rest = ""] = t(k).split(/:\s/);
            return (
              <li key={k} className="card storage__card">
                <IconTile name={icon} />
                <h3 className="h3">{lead}</h3>
                <p>{rest}</p>
              </li>
            );
          })}
        </ul>
      </div>
    </Section>
  );
}

function Setup({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const steps = ["web.teamsPage.setup.s1", "web.teamsPage.setup.s2", "web.teamsPage.setup.s3", "web.teamsPage.setup.s4"] as const;
  return (
    <Section id="setup" className="setup">
      <div className="wrap">
        <Head id="setup" title={t("web.teamsPage.setup.title")} />
        <ol className="setup__list">
          {steps.map((k, i) => (
            <StepCard key={k} n={`0${String(i + 1)}`}>
              <p>{t(k)}</p>
            </StepCard>
          ))}
        </ol>
        <p className="statement setup__price">{t("web.teamsPage.price")}</p>
      </div>
    </Section>
  );
}
