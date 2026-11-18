/**
 * Home (website v2 §5.1), after the approved prototype: say what it is in five seconds, give
 * three reasons, show where the data lives, then the features with a live fill demo, the agent
 * and tools sections, the plans and the closing call. The product is the imagery: the real app
 * window under the hero, close-ups of real screens, and interface demos that rest on their final
 * frame without JS. The free self-check is the soft exit for visitors who are not ready yet.
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon, type IconName } from "../Icon.tsx";
import { SHOTS } from "../Shot.tsx";
import { AgentDemo, AGENT_REF } from "../demos/AgentDemo.tsx";
import { FillDemo } from "../demos/FillDemo.tsx";
import { ImportDemo } from "../demos/ImportDemo.tsx";
import { PhoneDemo } from "../demos/PhoneDemo.tsx";
import { ArrowLink, Badges, Crop, Eyebrow, Head, HoloButton, IconTile, LineButton, ProductFrame, Rich, Section, Sentences, Tile, withCode } from "../ui.tsx";
import { ALL_PLATFORMS, WarnMock } from "./mocks.tsx";

export function HomeHero({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <section className="hero hero--home" aria-labelledby="hero-title">
      <div className="wrap hero__center">
        <Eyebrow intro="1">{t("web.hero.eyebrow")}</Eyebrow>
        <h1 id="hero-title" className="display display--xl" data-intro="2" data-decode="" data-island="decode">
          <span className="line">{t("web.hero.title1")}</span> <span className="line">{t("web.hero.title2")}</span>{" "}
          <span className="line line--soft">{t("web.hero.title3")}</span>
        </h1>
        <p className="lead" data-intro="3">
          {t("web.hero.sub1")} {t("web.hero.sub2")} {t("web.hero.sub3")}
        </p>
        <div className="actions actions--center" data-intro="4">
          <HoloButton href={ctx.href("download")} label={t("web.hero.ctaPrimary")} ambient />
          <LineButton href={ctx.href("security")} label={t("web.hero.ctaSecondary")} large />
        </div>
        <p className="hero__note" data-intro="5">
          {t("web.hero.note")}
        </p>
      </div>
    </section>
  );
}

function Showcase({ ctx }: { ctx: Ctx }) {
  const alt = ctx.t("web.features.desktopAlt");
  return (
    <section className="showcase" aria-label={alt}>
      <div className="wrap">
        <div className="showcase__glow" />
        <ProductFrame ctx={ctx} shot={SHOTS.vault} alt={alt} eager tilt glow={false} showcase className="showcase__window" />
      </div>
    </section>
  );
}

const REASONS = [
  ["cloud", "web.only.cloud.title", "web.only.cloud.text"],
  ["warning", "web.only.notice.title", "web.only.notice.text"],
  ["shield", "web.only.guess.title", "web.only.guess.text"],
] as const satisfies readonly (readonly [IconName, WebKey, WebKey])[];

function Reasons({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="reasons" className="reasons">
      <div className="wrap">
        <Head id="reasons" eyebrow={t("web.only.eyebrow")} title={t("web.only.title")} intro={t("web.only.intro")} />
        <ol className="ruled ruled--3">
          {REASONS.map(([icon, title, text], i) => (
            <li key={title} className="ruled__item" data-reveal="">
              <div className="ruled__visual">
                {i === 0 ? <Crop ctx={ctx} shot={SHOTS.sync} crop="sync-backend" alt={t("web.how.syncAlt")} /> : null}
                {i === 1 ? <WarnMock ctx={ctx} /> : null}
                {i === 2 ? <Crop ctx={ctx} shot={SHOTS.unlock} crop="unlock-field" alt={t("web.shot.unlock")} /> : null}
              </div>
              <IconTile name={icon} />
              <h3 className="ruled__title">{t(title)}</h3>
              <p className="ruled__text">{t(text)}</p>
            </li>
          ))}
        </ol>
        <ArrowLink href={ctx.href("compare")} label={t("web.home.reasons.link")} className="section__link" />
      </div>
    </Section>
  );
}

export function WhereStories({ ctx, link = true }: { ctx: Ctx; link?: boolean }) {
  const { t } = ctx;
  const stories = [
    ["web.where.story1.label", "web.where.story1.title", "web.where.story1.text"],
    ["web.where.story2.label", "web.where.story2.title", "web.where.story2.text"],
  ] as const;
  return (
    <Section id="where" className="where">
      <div className="wrap split">
        <div className="split__lead">
          <Head id="where" eyebrow={t("web.where.eyebrow")} title={t("web.where.title")} intro={t("web.where.body")} />
        </div>
        <ol className="timeline">
          {stories.map(([label, title, text]) => (
            <li key={label} className="timeline__item" data-reveal="">
              <p className="timeline__year">{t(label)}</p>
              <h3 className="timeline__title">{t(title)}</h3>
              <p className="timeline__text">{t(text)}</p>
            </li>
          ))}
        </ol>
      </div>
      <div className="wrap where__foot" data-reveal="">
        <p className="statement">{t("web.where.closing")}</p>
        <p className="fineprint">{t("web.where.sources")}</p>
        {link ? <ArrowLink href={ctx.href("security")} label={t("web.home.where.link")} /> : null}
      </div>
    </Section>
  );
}

function Bento({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="features" className="bento-section">
      <div className="wrap">
        <Head id="features" eyebrow={t("web.features.eyebrow")} title={t("web.features.title")} intro={t("web.features.intro")} />
        <div className="bento bento--home">
          <Tile wide icon="cursor" title={t("web.features.autofill.title")} text={t("web.features.autofill.text")} visual={<FillDemo ctx={ctx} />} />
          <Tile
            icon="key"
            title={t("web.features.passkeys.title")}
            text={t("web.features.passkeys.text")}
            visual={<Crop ctx={ctx} shot={SHOTS.passkeys} crop="passkeys-row" alt={t("web.shot.passkeys")} />}
          />
          <Tile
            icon="timer"
            title={t("web.features.codes.title")}
            text={t("web.features.codes.text")}
            visual={<Crop ctx={ctx} shot={SHOTS.vault} crop="vault-totp" alt={t("web.shot.totp")} />}
          />
          <Tile
            wide
            icon="monitor"
            title={t("web.features.desktop.title")}
            text={t("web.features.desktop.text")}
            visual={<Crop ctx={ctx} shot={SHOTS.vault} crop="vault-list" alt={t("web.shot.list")} />}
            visualClassName="tile__visual--bleed"
          />
          <article className="tile" data-reveal="">
            <IconTile name="phone" />
            <h3 className="tile__title">{t("web.features.phone.title")}</h3>
            <p className="tile__text">{t("web.features.phone.text")}</p>
            <Badges ctx={ctx} items={ALL_PLATFORMS.slice(3, 5)} small />
          </article>
          <Tile icon="eyeOff" title={t("web.features.tracking.title")} text={t("web.features.tracking.text")} />
        </div>
        <ArrowLink href={ctx.href("features")} label={t("web.home.features.link")} className="section__link" />
      </div>
    </Section>
  );
}

function Agent({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="agent">
      <div className="wrap dev">
        <div className="dev__copy" data-reveal="">
          <Eyebrow>{t("web.agent.eyebrow")}</Eyebrow>
          <h2 id="agent-title" className="h2">
            {t("web.agent.title")}
          </h2>
          <p className="lead">{withCode(t, "web.agent.intro", AGENT_REF)}</p>
          <ul className="checks">
            {(["web.agent.p1", "web.agent.p2", "web.agent.p3"] as const).map((key) => (
              <li key={key}>
                <Icon name="check" className="checks__icon" />
                <span>{t(key)}</span>
              </li>
            ))}
          </ul>
        </div>
        <div data-reveal="">
          <AgentDemo ctx={ctx} />
        </div>
      </div>
    </Section>
  );
}

const TOOLS = [
  ["globe", "web.tools.ext"],
  ["monitor", "web.tools.desktop"],
  ["layers", "web.tools.programs"],
  ["phone", "web.tools.mobile"],
  ["terminal", "web.tools.cli"],
  ["import", "web.tools.import"],
] as const satisfies readonly (readonly [IconName, WebKey])[];

function Tools({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="tools">
      <div className="wrap">
        <Head id="tools" eyebrow={t("web.tools.eyebrow")} title={t("web.tools.title")} intro={t("web.tools.intro")} />
        <div className="tools__grid">
          <div className="tools__col">
            <div data-reveal="">
              <ImportDemo ctx={ctx} />
            </div>
            <div className="tools__caption" data-reveal="">
              <h3>{t("web.switch.title")}</h3>
              <p>
                {t("web.switch.bring.title")} {t("web.switch.bring.text")} {t("web.switch.leave.title")} {t("web.switch.leave.text")}
              </p>
            </div>
          </div>
          <div className="tools__col">
            <div data-reveal="">
              <PhoneDemo ctx={ctx} />
            </div>
            <div className="tools__caption" data-reveal="">
              <h3>{t("web.features.phone.title")}</h3>
              <p>{t("web.features.phone.text")}</p>
            </div>
            <div data-reveal="">
              <Badges ctx={ctx} items={TOOLS} />
            </div>
          </div>
        </div>
      </div>
    </Section>
  );
}

function SelfCheckStrip({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="self-check" className="tool-strip">
      <div className="wrap">
        <a className="tool-card" href={ctx.href("check")} data-reveal="">
          <div className="tool-card__body">
            <Eyebrow>{t("web.home.tool.eyebrow")}</Eyebrow>
            <h2 id="self-check-title" className="h2">
              {t("web.home.tool.title")}
            </h2>
            <p className="lead">{t("web.home.tool.text")}</p>
            <span className="arrow-link">
              <span>{t("web.home.tool.link")}</span>
              <Icon name="arrowRight" className="icon--nudge" />
            </span>
          </div>
          <div className="tool-card__sample" aria-hidden="true">
            <span className="tool-card__q">{t("web.check.q4")}</span>
            <span className="tool-card__opts">
              <span className="tool-card__opt is-on">{t("web.check.q4.a")}</span>
              <span className="tool-card__opt">{t("web.check.q4.b")}</span>
              <span className="tool-card__opt">{t("web.check.q4.c")}</span>
            </span>
          </div>
        </a>
      </div>
    </Section>
  );
}

export function PlanCards({ ctx, full = false }: { ctx: Ctx; full?: boolean }) {
  const { t } = ctx;
  const free = ["web.pricing.free.item1", "web.pricing.free.item2", "web.pricing.free.item3", "web.pricing.free.item4", "web.pricing.free.item5", "web.pricing.free.item6"] as const;
  const hosted = ["web.pricing.hosted.item1", "web.pricing.hosted.item2", "web.pricing.hosted.item3", "web.pricing.hosted.item4", "web.pricing.hosted.item5", "web.pricing.hosted.item6"] as const;
  const check = (key: WebKey) => (
    <li key={key}>
      <Icon name="check" className="checks__icon" />
      <span>{t(key)}</span>
    </li>
  );
  return (
    <ul className={full ? "plans plans--full" : "plans"}>
      <li className="plan plan--free" data-reveal="">
        <h3 className="plan__name">{t("web.pricing.free.name")}</h3>
        <p className="plan__price">{t("web.pricing.free.price")}</p>
        <p className="plan__tagline">{t("web.pricing.free.tagline")}</p>
        {full ? (
          <ul className="checks plan__items">
            {free.map(check)}
            <li>
              <Icon name="check" className="checks__icon" />
              <span>
                <Rich t={t} k="web.pricing.free.item7" />
              </span>
            </li>
          </ul>
        ) : null}
        {full ? <LineButton href={ctx.href("download")} label={t("web.pricing.free.cta")} /> : null}
      </li>
      <li className="plan plan--hosted" data-reveal="">
        <h3 className="plan__name">{t("web.pricing.hosted.name")}</h3>
        <p className="plan__price plan__price--soon">{t("web.pricing.hosted.price")}</p>
        <p className="plan__tagline">{t("web.pricing.hosted.tagline")}</p>
        {full ? <ul className="checks plan__items">{hosted.map(check)}</ul> : null}
        {full ? <LineButton href={ctx.href("download")} label={t("web.pricing.hosted.cta")} /> : null}
      </li>
    </ul>
  );
}

function PricingTeaser({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="pricing" className="pricing-teaser">
      <div className="wrap split">
        <div className="split__lead">
          <Head id="pricing" eyebrow={t("web.pricing.eyebrow")} title={t("web.pricing.title")} intro={t("web.pricing.intro")} />
          <ArrowLink href={ctx.href("pricing")} label={t("web.home.pricing.link")} />
        </div>
        <PlanCards ctx={ctx} />
      </div>
    </Section>
  );
}

/** The closing call to action over a soft holo glow. */
export function FinalCta({ ctx, id = "get" }: { ctx: Ctx; id?: string }) {
  const { t } = ctx;
  return (
    <Section id={id} className="final">
      <div className="wrap final__inner" data-reveal="">
        <Eyebrow>{t("web.download.eyebrow")}</Eyebrow>
        <h2 id={`${id}-title`} className="display final__title">
          <Sentences text={t("web.download.title")} />
        </h2>
        <p className="lead">{t("web.download.body")}</p>
        <div className="actions actions--center">
          <HoloButton href={ctx.href("download")} label={t("web.download.ctaPrimary")} />
        </div>
        <Badges ctx={ctx} items={ALL_PLATFORMS} center />
      </div>
    </Section>
  );
}

export function Home({ ctx }: { ctx: Ctx }) {
  return (
    <>
      <HomeHero ctx={ctx} />
      <Showcase ctx={ctx} />
      <Reasons ctx={ctx} />
      <WhereStories ctx={ctx} />
      <Bento ctx={ctx} />
      <Agent ctx={ctx} />
      <Tools ctx={ctx} />
      <PricingTeaser ctx={ctx} />
      <SelfCheckStrip ctx={ctx} />
      <FinalCta ctx={ctx} />
    </>
  );
}
