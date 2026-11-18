/**
 * Download (website v2 §5.8, tint violet), Next steps and Check a download. The visitor's
 * system comes first (downloads island). A platform tile links to its release file once there
 * is one (site.files) and otherwise straight to the next-steps page. The hero shows the real
 * app window and the platform badges.
 */
import type { Platform as PlatformId } from "../../downloads.ts";
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon, type IconName } from "../Icon.tsx";
import { SHOTS } from "../Shot.tsx";
import { site } from "../site.ts";
import { ArrowLink, Badges, HoloButton, IconTile, LineButton, ProductFrame, Section } from "../ui.tsx";
import { ALL_PLATFORMS } from "./mocks.tsx";
import { PageHero } from "./parts.tsx";
import { NextSteps } from "../demos/NextSteps.tsx";

const GROUPS = [
  ["computer", "web.downloadPage.computer", [["windows", "web.platform.windows", "monitor"], ["macos", "web.platform.macos", "monitor"]]],
  ["phone", "web.downloadPage.phone", [["android", "web.platform.android", "phone"]]],
  ["browser", "web.downloadPage.browser", [["chrome", "web.platform.chrome", "globe"], ["edge", "web.platform.edge", "globe"], ["firefox", "web.platform.firefox", "globe"]]],
  ["cli", "web.downloadPage.cli", [["cli", "web.platform.cli", "terminal"]]],
] as const satisfies readonly (readonly [string, WebKey, readonly (readonly [PlatformId, WebKey, IconName])[]])[];

function Tile({ ctx, id, label, icon }: { ctx: Ctx; id: PlatformId; label: WebKey; icon: IconName }) {
  const { t } = ctx;
  const store = id === "android";
  if (store && !site.stores[id]) {
    return (
      <li className="platform platform--soon" data-platform={id}>
        <Icon name={icon} className="platform__icon" />
        <span className="platform__name">{t(label)}</span>
        <span className="platform__state">{t("web.downloadPage.soon")}</span>
      </li>
    );
  }
  const file = site.files[id];
  return (
    <li className="platform" data-platform={id}>
      <a className="platform__link" href={file ?? ctx.href("next")} data-next={file === undefined ? undefined : ctx.href("next")} download={file === undefined ? undefined : ""}>
        <Icon name={icon} className="platform__icon" />
        <span className="platform__name">{t(label)}</span>
        <span className="platform__state">
          <span className="platform__detected">{t("web.downloadPage.detected")}</span>
          {t("web.downloadPage.get")}
          <Icon name="download" />
        </span>
      </a>
    </li>
  );
}

export function Download({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.downloadPage.eyebrow")}
        title={t("web.downloadPage.title")}
        sub={t("web.downloadPage.sub")}
        visual={
          <div className="dl-visual">
            <ProductFrame ctx={ctx} shot={SHOTS.vault} alt={t("web.features.desktopAlt")} eager />
            <Badges ctx={ctx} items={ALL_PLATFORMS} center />
          </div>
        }
        actions={
          <div className="dl-cta" data-island="downloads" data-label={t("web.downloadPage.cta", { os: "%os%" })}>
            <HoloButton href="#platforms" label={t("web.downloadPage.ctaAny")} icon="arrowRight" jump />
            <p className="hero__note">{t("web.downloadPage.other")}</p>
          </div>
        }
      />
      <Section id="platforms" tabIndex={-1} className="platform-section" label={t("web.footer.download")}>
        <div className="wrap platform-groups">
          {GROUPS.map(([group, title, items]) => (
            <div key={group} id={group} className={`platform-group platform-group--${group}`} data-reveal="">
              <h2 className="platform-group__title">{t(title)}</h2>
              <ul className="platforms">
                {items.map(([id, label, icon]) => (
                  <Tile key={id} ctx={ctx} id={id} label={label} icon={icon} />
                ))}
              </ul>
            </div>
          ))}
        </div>
      </Section>
      <Section id="signed" className="signed">
        <div className="wrap signed__grid">
          <div className="card signed__card" data-reveal="">
            <IconTile name="shield" />
            <h2 id="signed-title" className="h3">
              {t("web.downloadPage.signed.title")}
            </h2>
            <p>{t("web.downloadPage.signed.text")}</p>
            <ArrowLink href={ctx.href("verify")} label={t("web.downloadPage.signed.link")} />
          </div>
          <div className="card signed__card" data-reveal="">
            <IconTile name="spark" />
            <h2 className="h3">{t("web.downloadPage.next.title")}</h2>
            <p>{t("web.next.s1")}</p>
            <ArrowLink href={ctx.href("next")} label={t("web.downloadPage.next.link")} />
          </div>
        </div>
      </Section>
    </>
  );
}

export function Next({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero
        ctx={ctx}
        crumbs={[
          ["download", t("web.downloadPage.eyebrow")],
          ["next", t("web.next.title")],
        ]}
        title={t("web.next.title")}
        className="hero--next"
      />
      <Section id="steps" label={t("web.next.title")} className="section--flush">
        <div className="wrap">
          <NextSteps ctx={ctx} />
        </div>
      </Section>
    </>
  );
}

/** A static command in the terminal style: no typing, no demo. */
function Command({ prompt, command }: { prompt: string; command: string }) {
  return (
    <div className="terminal terminal--static">
      <div className="terminal__bar" aria-hidden="true">
        <i />
        <i />
        <i />
      </div>
      <div className="terminal__body">
        <p className="terminal__line">
          <span className="terminal__prompt" aria-hidden="true">
            {prompt}
          </span>{" "}
          <code>{command}</code>
        </p>
      </div>
    </div>
  );
}

export function Verify({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const file = `<${t("web.verify.file")}>`;
  return (
    <>
      <PageHero
        ctx={ctx}
        crumbs={[
          ["download", t("web.downloadPage.eyebrow")],
          ["verify", t("web.footer.verify")],
        ]}
        title={t("web.verify.title")}
        sub={t("web.verify.text")}
        className="hero--verify"
      />
      <Section id="commands" className="commands" label={t("web.verify.title")}>
        <div className="wrap commands__grid">
          <div className="card">
            <h2 className="h3">{t("web.verify.windows")}</h2>
            <Command prompt=">" command={`Get-FileHash .\\${file}`} />
          </div>
          <div className="card">
            <h2 className="h3">{t("web.verify.unix")}</h2>
            <Command prompt="$" command={`shasum -a 256 ${file}`} />
          </div>
          <p className="statement commands__match">{t("web.verify.match")}</p>
          <div className="actions">
            <LineButton href={ctx.href("download")} label={t("web.verify.back")} large />
          </div>
        </div>
      </Section>
    </>
  );
}
