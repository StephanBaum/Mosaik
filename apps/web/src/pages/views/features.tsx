/**
 * Features (website v2 §5.2, prototype 4395-4443): everything it does, grouped by where you use
 * it. Visuals are real screens and crops of them, the fill popup built from the app's tokens,
 * and the terminal demo.
 */
import type { Ctx } from "../i18n.ts";
import { SHOTS } from "../Shot.tsx";
import { CliDemo } from "../demos/CliDemo.tsx";
import { Badges, Checks, Crop, Eyebrow, Head, IconTile, Point, ProductFrame, Section, Tile } from "../ui.tsx";
import { ALL_PLATFORMS, FillMock } from "./mocks.tsx";
import { PageCta, PageHero } from "./parts.tsx";

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
        <div className="wrap" data-reveal="">
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
          <article className="tile" data-reveal="">
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
        <div data-reveal="">
          <ProductFrame ctx={ctx} shot={SHOTS.vault} alt={t("web.features.desktopAlt")} />
        </div>
        <div className="ruled ruled--2 devices__notes">
          <div className="ruled__item" data-reveal="">
            <IconTile name="phone" />
            <h3 className="ruled__title">{t("web.features.phone.title")}</h3>
            <p className="ruled__text">{t("web.features.phone.text")}</p>
          </div>
          <div className="ruled__item" data-reveal="">
            <p className="eyebrow">{t("web.features.platformsLabel")}</p>
            <Badges ctx={ctx} items={ALL_PLATFORMS.slice(0, 8)} />
          </div>
        </div>
        <div className="ruled ruled--2 devices__notes">
          <div className="ruled__item" data-reveal="">
            <Point icon="layers" title={t("web.features.programs.title")} text={t("web.features.programs.text")} />
          </div>
          <div className="ruled__item" data-reveal="">
            <Point icon="face" title={t("web.features.androidLink.title")} text={t("web.features.androidLink.text")} />
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
        <div className="bento bento--more">
          <Tile
            wide
            icon="spark"
            title={t("web.features.generator.title")}
            text={t("web.features.generator.text")}
            visual={<Crop ctx={ctx} shot={SHOTS.insights} crop="insights-cards" alt={t("web.shot.insights")} />}
            visualClassName="tile__visual--bleed"
          />
          <Tile icon="users" title={t("web.features.sharing.title")} text={t("web.features.sharing.text")} />
          <Tile icon="buoy" title={t("web.features.emergency.title")} text={t("web.features.emergency.text")} />
          <Tile icon="cloud" title={t("web.features.folders.title")} text={t("web.features.folders.text")} />
          <Tile
            icon="import"
            title={t("web.switch.bring.title")}
            text={t("web.switch.bring.text")}
            visual={<Crop ctx={ctx} shot={SHOTS.import} crop="import-preview" alt={t("web.shot.import")} />}
          />
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
        <div className="dev__copy" data-reveal="">
          <Eyebrow>{t("web.featuresPage.developers")}</Eyebrow>
          <h2 id="developers-title" className="h2">
            {t("web.features.ssh.title")}
          </h2>
          <p className="lead">{t("web.features.ssh.text")}</p>
        </div>
        <div data-reveal="">
          <CliDemo ctx={ctx} withTitle />
        </div>
      </div>
    </Section>
  );
}
