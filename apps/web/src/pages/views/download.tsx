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
import { ArrowLink, HoloButton, IconTile, LineButton, ProductFrame, Section, StepCard } from "../ui.tsx";
import { ALL_PLATFORMS, PlatformBadges } from "./mocks.tsx";
import { PageHero } from "./parts.tsx";

const GROUPS = [
  ["computer", "web.downloadPage.computer", [["windows", "web.platform.windows", "monitor"], ["macos", "web.platform.macos", "monitor"], ["linux", "web.platform.linux", "monitor"]]],
  ["phone", "web.downloadPage.phone", [["iphone", "web.platform.iphone", "phone"], ["android", "web.platform.android", "phone"]]],
  ["browser", "web.downloadPage.browser", [["chrome", "web.platform.chrome", "globe"], ["edge", "web.platform.edge", "globe"], ["firefox", "web.platform.firefox", "globe"]]],
  ["cli", "web.downloadPage.cli", [["cli", "web.platform.cli", "terminal"]]],
] as const satisfies readonly (readonly [string, WebKey, readonly (readonly [PlatformId, WebKey, IconName])[]])[];

function Tile({ ctx, id, label, icon }: { ctx: Ctx; id: PlatformId; label: WebKey; icon: IconName }) {
  const { t } = ctx;
  const store = id === "iphone" || id === "android";
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
            <PlatformBadges ctx={ctx} items={ALL_PLATFORMS} className="badges--center" />
          </div>
        }
        actions={
          <div className="dl-cta" data-island="downloads" data-label={t("web.downloadPage.cta", { os: "%os%" })}>
            <HoloButton href="#platforms" label={t("web.downloadPage.ctaAny")} />
            <p className="hero__note">{t("web.downloadPage.other")}</p>
          </div>
        }
      />
      <Section id="platforms" className="platform-section" label={t("web.footer.download")}>
        <div className="wrap platform-groups">
          {GROUPS.map(([group, title, items]) => (
            <div key={group} id={group} className={`platform-group platform-group--${group}`}>
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
          <div className="card signed__card">
            <IconTile name="shield" />
            <h2 id="signed-title" className="h3">
              {t("web.downloadPage.signed.title")}
            </h2>
            <p>{t("web.downloadPage.signed.text")}</p>
            <ArrowLink href={ctx.href("verify")} label={t("web.downloadPage.signed.link")} />
          </div>
          <div className="card signed__card">
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
  const steps = [
    ["web.next.s1", null],
    ["web.next.s2", null],
    ["web.next.s3", ["switch", "web.next.s3link"]],
    ["web.next.s4", null],
    ["web.next.s5", ["pricing", "web.next.s5link"]],
  ] as const;
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
        visual={
          <ol className="float-steps">
            {steps.map(([k, link], i) => {
              const [lead = "", ...rest] = t(k).split(/(?<=\.)\s/);
              return (
                <StepCard key={k} n={`0${String(i + 1)}`} title={lead}>
                  <p>{rest.join(" ")}</p>
                  {link === null ? null : <ArrowLink href={ctx.href(link[0])} label={t(link[1])} />}
                </StepCard>
              );
            })}
          </ol>
        }
      />
    </>
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
            <pre className="code">
              <code>{`Get-FileHash .\\${file}`}</code>
            </pre>
          </div>
          <div className="card">
            <h2 className="h3">{t("web.verify.unix")}</h2>
            <pre className="code">
              <code>{`shasum -a 256 ${file}`}</code>
            </pre>
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
