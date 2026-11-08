/**
 * Features (website v2 §5.2): everything it does, grouped by where you use it. Visuals are
 * real screens and crops of them, plus the fill popup built from the app's tokens.
 */
import type { ReactNode } from "react";
import type { Ctx, WebKey } from "../i18n.ts";
import { type IconName } from "../Icon.tsx";
import { SHOTS } from "../Shot.tsx";
import { Checks, Crop, Eyebrow, Head, IconTile, ProductFrame, Rich, Section } from "../ui.tsx";
import { ALL_PLATFORMS, FillMock, PlatformBadges } from "./mocks.tsx";
import { PageCta, PageHero } from "./parts.tsx";

type Item = readonly [icon: IconName, title: WebKey, text: WebKey];

function TextTile({ ctx, item, children, className }: { ctx: Ctx; item: Item; children?: ReactNode; className?: string }) {
  const [icon, title, text] = item;
  return (
    <article className={["tile", className].filter(Boolean).join(" ")}>
      {children === undefined ? null : <div className="tile__visual">{children}</div>}
      <IconTile name={icon} />
      <h3 className="tile__title">{ctx.t(title)}</h3>
      <p className="tile__text">{ctx.t(text)}</p>
    </article>
  );
}

export function Features({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.featuresPage.eyebrow")}
        title={t("web.features.title")}
        sub={t("web.features.intro")}
        visual={
          <div className="pair">
            <ProductFrame ctx={ctx} shot={SHOTS.item} alt={t("web.shot.item")} eager className="pair__main" />
            <FillMock ctx={ctx} className="pair__float" />
          </div>
        }
      />

      <InBrowser ctx={ctx} />

      <OnDevices ctx={ctx} />

      <MoreThanPasswords ctx={ctx} />

      <ForDevelopers ctx={ctx} />

      <Section id="tracking" className="feat-tracking">
        <div className="wrap">
          <h2 id="tracking-title" className="tracking-line">
            <span>{t("web.features.tracking.title")}</span> <span className="tracking-line__rest">{t("web.features.tracking.text")}</span>
          </h2>
        </div>
      </Section>

      <PageCta ctx={ctx} title={t("web.featuresPage.cta.title")} secondary={[ctx.href("pricing"), t("web.featuresPage.cta.secondary")]} />
    </>
  );
}

function InBrowser({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="browser" className="feat-browser">
      <div className="wrap">
        <Head id="browser" eyebrow={t("web.featuresPage.browser")} title={t("web.features.autofill.title")} intro={t("web.features.autofill.text")} />
        <div className="bento">
          <TextTile ctx={ctx} item={["key", "web.features.passkeys.title", "web.features.passkeys.text"]}>
            <Crop ctx={ctx} shot={SHOTS.passkeys} crop="passkeys-row" alt={t("web.shot.passkeys")} />
          </TextTile>
          <TextTile ctx={ctx} item={["timer", "web.features.codes.title", "web.features.codes.text"]}>
            <Crop ctx={ctx} shot={SHOTS.vault} crop="vault-totp" alt={t("web.shot.totp")} />
          </TextTile>
          <article className="tile">
            <IconTile name="shield" />
            <h3 className="tile__title">{t("web.only.unlikeTitle")}</h3>
            <Checks items={[t("web.only.unlike1"), t("web.only.unlike2"), t("web.only.unlike3")]} />
          </article>
        </div>
      </div>
    </Section>
  );
}

function OnDevices({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="devices" className="feat-devices">
      <div className="wrap">
        <Head id="devices" eyebrow={t("web.featuresPage.devices")} title={t("web.features.desktop.title")} intro={t("web.features.desktop.text")} />
        <ProductFrame ctx={ctx} shot={SHOTS.vault} alt={t("web.features.desktopAlt")} />
        <div className="ruled ruled--2 devices__notes">
          <div className="ruled__item">
            <IconTile name="phone" />
            <h3 className="ruled__title">{t("web.features.phone.title")}</h3>
            <p className="ruled__text">{t("web.features.phone.text")}</p>
          </div>
          <div className="ruled__item">
            <p className="eyebrow">{t("web.features.platformsLabel")}</p>
            <PlatformBadges ctx={ctx} items={ALL_PLATFORMS.slice(0, 8)} />
          </div>
        </div>
      </div>
    </Section>
  );
}

function MoreThanPasswords({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="more" className="feat-more">
      <div className="wrap">
        <Head id="more" eyebrow={t("web.featuresPage.more")} title={t("web.features.more.title")} intro={t("web.features.more.text")} />
        <div className="bento">
          <article className="tile tile--wide">
            <div className="tile__copy">
              <IconTile name="spark" />
              <h3 className="tile__title">{t("web.features.generator.title")}</h3>
              <p className="tile__text">{t("web.features.generator.text")}</p>
            </div>
            <div className="tile__visual tile__visual--bleed">
              <Crop ctx={ctx} shot={SHOTS.insights} crop="insights-cards" alt={t("web.shot.insights")} />
            </div>
          </article>
          <TextTile ctx={ctx} item={["users", "web.features.sharing.title", "web.features.sharing.text"]} />
          <TextTile ctx={ctx} item={["buoy", "web.features.emergency.title", "web.features.emergency.text"]} />
          <TextTile ctx={ctx} item={["import", "web.switch.bring.title", "web.switch.bring.text"]}>
            <Crop ctx={ctx} shot={SHOTS.import} crop="import-preview" alt={t("web.shot.import")} />
          </TextTile>
        </div>
      </div>
    </Section>
  );
}

function ForDevelopers({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="developers" className="feat-dev">
      <div className="wrap dev">
        <div className="dev__copy">
          <Eyebrow>{t("web.featuresPage.developers")}</Eyebrow>
          <h2 id="developers-title" className="h2">
            {t("web.features.ssh.title")}
          </h2>
          <p className="lead">{t("web.features.ssh.text")}</p>
        </div>
        <div className="terminal" role="group" aria-labelledby="terminal-title">
          <div className="terminal__bar" aria-hidden="true">
            <i />
            <i />
            <i />
          </div>
          <div className="terminal__body">
            <h3 id="terminal-title" className="terminal__title">
              {t("web.features.terminal.title")}
            </h3>
            <p className="terminal__text">
              <Rich t={t} k="web.features.terminal.text" />
            </p>
            <p className="terminal__line" aria-hidden="true">
              <span className="terminal__prompt">$</span> mosaic<span className="terminal__caret" />
            </p>
          </div>
        </div>
      </div>
    </Section>
  );
}
