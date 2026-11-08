/**
 * Home (website v2 §5.1): say what it is in five seconds, give three reasons and route
 * visitors. The product is the imagery: the real app window over the live holo-foil plane in
 * the hero, close-up crops of real screens as feature visuals, no illustrations. The free
 * self-check is the soft exit for visitors who are not ready yet.
 */
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon, type IconName } from "../Icon.tsx";
import { SHOTS } from "../Shot.tsx";
import { ArrowLink, Crop, Eyebrow, Head, HoloButton, IconTile, LineButton, ProductFrame, Rich, Section, Sentences } from "../ui.tsx";
import { ALL_PLATFORMS, FillMock, PlatformBadges, WarnMock } from "./mocks.tsx";

/** The live foil plane (islands/foil.ts) with its designed static gradient underneath. */
export function Foil({ className }: { className?: string }) {
  return (
    <div className={["foil", className].filter(Boolean).join(" ")} data-island="foil" aria-hidden="true">
      <canvas className="foil__canvas" />
    </div>
  );
}

export function HomeHero({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <section className="hero hero--home" aria-labelledby="hero-title">
      <div className="wrap hero__center">
        <Eyebrow>{t("web.hero.eyebrow")}</Eyebrow>
        <h1 id="hero-title" className="display display--xl">
          <span className="line">{t("web.hero.title1")}</span> <span className="line">{t("web.hero.title2")}</span>{" "}
          <span className="line line--soft">{t("web.hero.title3")}</span>
        </h1>
        <p className="lead">
          {t("web.hero.sub1")} {t("web.hero.sub2")} {t("web.hero.sub3")}
        </p>
        <div className="actions actions--center">
          <HoloButton href={ctx.href("download")} label={t("web.hero.ctaPrimary")} />
          <LineButton href={ctx.href("security")} label={t("web.hero.ctaSecondary")} large />
        </div>
        <p className="hero__note">{t("web.hero.note")}</p>
      </div>
      <div className="wrap showcase">
        <Foil className="foil--hero" />
        <ProductFrame ctx={ctx} shot={SHOTS.vault} alt={t("web.features.desktopAlt")} eager tilt glow={false} className="showcase__window" />
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
            <li key={title} className="ruled__item">
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
    <Section id="where-it-lives" className="where">
      <div className="wrap split">
        <div className="split__lead">
          <Head id="where-it-lives" eyebrow={t("web.where.eyebrow")} title={t("web.where.title")} intro={t("web.where.body")} />
        </div>
        <ol className="timeline">
          {stories.map(([label, title, text]) => (
            <li key={label} className="timeline__item">
              <p className="timeline__year">{t(label)}</p>
              <h3 className="timeline__title">{t(title)}</h3>
              <p className="timeline__text">{t(text)}</p>
            </li>
          ))}
        </ol>
      </div>
      <div className="wrap where__foot">
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
          <article className="tile tile--wide">
            <div className="tile__copy">
              <IconTile name="cursor" />
              <h3 className="tile__title">{t("web.features.autofill.title")}</h3>
              <p className="tile__text">{t("web.features.autofill.text")}</p>
            </div>
            <div className="tile__visual tile__visual--center">
              <FillMock ctx={ctx} />
            </div>
          </article>
          <article className="tile">
            <div className="tile__visual">
              <Crop ctx={ctx} shot={SHOTS.passkeys} crop="passkeys-row" alt={t("web.shot.passkeys")} />
            </div>
            <IconTile name="key" />
            <h3 className="tile__title">{t("web.features.passkeys.title")}</h3>
            <p className="tile__text">{t("web.features.passkeys.text")}</p>
          </article>
          <article className="tile">
            <div className="tile__visual">
              <Crop ctx={ctx} shot={SHOTS.vault} crop="vault-totp" alt={t("web.shot.totp")} />
            </div>
            <IconTile name="timer" />
            <h3 className="tile__title">{t("web.features.codes.title")}</h3>
            <p className="tile__text">{t("web.features.codes.text")}</p>
          </article>
          <article className="tile tile--wide tile--reverse">
            <div className="tile__copy">
              <IconTile name="monitor" />
              <h3 className="tile__title">{t("web.features.desktop.title")}</h3>
              <p className="tile__text">{t("web.features.desktop.text")}</p>
            </div>
            <div className="tile__visual tile__visual--bleed">
              <Crop ctx={ctx} shot={SHOTS.vault} crop="vault-list" alt={t("web.shot.list")} />
            </div>
          </article>
          <article className="tile">
            <IconTile name="phone" />
            <h3 className="tile__title">{t("web.features.phone.title")}</h3>
            <p className="tile__text">{t("web.features.phone.text")}</p>
            <PlatformBadges ctx={ctx} items={ALL_PLATFORMS.slice(3, 5)} className="badges--small" />
          </article>
          <article className="tile">
            <IconTile name="eyeOff" />
            <h3 className="tile__title">{t("web.features.tracking.title")}</h3>
            <p className="tile__text">{t("web.features.tracking.text")}</p>
          </article>
        </div>
        <ArrowLink href={ctx.href("features")} label={t("web.home.features.link")} className="section__link" />
      </div>
    </Section>
  );
}

function SelfCheckStrip({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="self-check" className="tool-strip">
      <div className="wrap">
        <a className="tool-card" href={ctx.href("check")}>
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
      <li className="plan plan--free">
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
      <li className="plan plan--hosted">
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
      <div className="wrap final__inner">
        <Eyebrow>{t("web.download.eyebrow")}</Eyebrow>
        <h2 id={`${id}-title`} className="display final__title">
          <Sentences text={t("web.download.title")} />
        </h2>
        <p className="lead">{t("web.download.body")}</p>
        <div className="actions actions--center">
          <HoloButton href={ctx.href("download")} label={t("web.download.ctaPrimary")} />
        </div>
        <PlatformBadges ctx={ctx} items={ALL_PLATFORMS} className="badges--center" />
      </div>
    </Section>
  );
}

export function Home({ ctx }: { ctx: Ctx }) {
  return (
    <>
      <HomeHero ctx={ctx} />
      <Reasons ctx={ctx} />
      <WhereStories ctx={ctx} />
      <Bento ctx={ctx} />
      <SelfCheckStrip ctx={ctx} />
      <PricingTeaser ctx={ctx} />
      <FinalCta ctx={ctx} />
    </>
  );
}
