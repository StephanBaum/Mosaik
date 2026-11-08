/**
 * Compare hub and the vendor pages (website v2 §5.4, tint yellow): fair, sourced tables. Every
 * cell links to its numbered source; unverified cells say so. The matrix is the hero visual;
 * the switch block shows the real import preview.
 */
import { citedSources, PRODUCTS, ROWS, SOURCES, type Row } from "../compareData.ts";
import type { Ctx, WebKey } from "../i18n.ts";
import { Icon } from "../Icon.tsx";
import type { RouteId } from "../routes.ts";
import { SHOTS } from "../Shot.tsx";
import { ArrowLink, CaptureCard, Checks, Crop, Eyebrow, Head, Section, StepCard } from "../ui.tsx";
import { PageCta, PageHero } from "./parts.tsx";

function Refs({ ctx, refs }: { ctx: Ctx; refs: readonly number[] }) {
  return (
    <>
      {refs.map((n) => (
        <a key={n} className="ref" href={`#source-${String(n)}`} aria-label={ctx.t("web.compare.matrix.source", { n })}>
          {n}
        </a>
      ))}
    </>
  );
}

function Matrix({ ctx, columns, caption }: { ctx: Ctx; columns: readonly number[]; caption: string }) {
  const { t } = ctx;
  return (
    <div className="matrix-scroll" role="region" aria-label={caption} tabIndex={0}>
      <table className={`matrix matrix--${String(columns.length)}`}>
        <caption className="sr-only">{caption}</caption>
        <thead>
          <tr>
            <th scope="col">{t("web.compare.matrix.feature")}</th>
            {columns.map((c) => (
              <th key={c} scope="col" className={c === 0 ? "is-us" : undefined}>
                {PRODUCTS[c]}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {ROWS.map((row: Row) => (
            <tr key={row.id}>
              <th scope="row">{t(row.label)}</th>
              {columns.map((c) => {
                const cell = row.cells[c] ?? null;
                return (
                  <td key={c} className={[c === 0 ? "is-us" : "", cell === null ? "is-unverified" : ""].filter(Boolean).join(" ") || undefined}>
                    {cell === null ? (
                      <span className="unverified">{t("web.compare.matrix.notVerified")}</span>
                    ) : (
                      <>
                        {t(cell.k)}
                        {cell.refs === undefined ? null : <Refs ctx={ctx} refs={cell.refs} />}
                      </>
                    )}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function Sources({ ctx, numbers }: { ctx: Ctx; numbers: readonly number[] }) {
  const { t } = ctx;
  return (
    <Section id="sources" className="sources">
      <div className="wrap">
        <h2 id="sources-title" className="h3">
          {t("web.compare.sources.title")}
        </h2>
        <ol className="sources__list">
          {numbers.map((n) => {
            const [title, url] = SOURCES[n - 1] ?? ["", ""];
            return (
              <li key={n} id={`source-${String(n)}`} value={n}>
                <a href={url} rel="noopener noreferrer external">
                  {title}
                </a>
              </li>
            );
          })}
        </ol>
        <p className="fineprint">{t("web.compare.sources.note")}</p>
      </div>
    </Section>
  );
}

/** "Decided? Moving takes minutes." with the puzzle mini stage and the capture card. */
function SwitchBlock({ ctx, src }: { ctx: Ctx; src: RouteId }) {
  const { t } = ctx;
  return (
    <Section id="switch" className="switch-block">
      <div className="wrap switch-block__grid">
        <a className="mini-stage" href={ctx.href("switch")}>
          <Crop ctx={ctx} shot={SHOTS.import} crop="import-preview" alt={t("web.shot.import")} className="mini-stage__art" />
          <span className="mini-stage__copy">
            <span id="switch-title" className="h2">
              {t("web.compare.switch.title")}
            </span>
            <span className="arrow-link">
              <span>{t("web.compare.switch.link")}</span>
              <Icon name="arrowRight" className="icon--nudge" />
            </span>
          </span>
        </a>
        <CaptureCard ctx={ctx} src={src} />
      </div>
    </Section>
  );
}

export function Compare({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const all = PRODUCTS.map((_, i) => i);
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.compare.eyebrow")}
        title={t("web.compare.title")}
        sub={t("web.compare.sub")}
      />
      <Section id="matrix" className="matrix-section" label={t("web.compare.matrix.caption")}>
        <div className="wrap">
          <Matrix ctx={ctx} columns={all} caption={t("web.compare.matrix.caption")} />
        </div>
      </Section>
      <OnlyMosaic ctx={ctx} />
      <Ahead ctx={ctx} />
      <Pick ctx={ctx} />
      <DetailCards ctx={ctx} />
      <SwitchBlock ctx={ctx} src="compare" />
      <PageCta ctx={ctx} title={t("web.download.title")} text={t("web.download.body")} />
      <Sources ctx={ctx} numbers={citedSources(ROWS, all)} />
    </>
  );
}

interface Vendor {
  id: "vs1password" | "vsBitwarden";
  name: string;
  column: number;
  ahead: readonly WebKey[];
  different: readonly WebKey[];
  steps: WebKey;
}

const VENDORS: Readonly<Record<Vendor["id"], Vendor>> = {
  vs1password: {
    id: "vs1password",
    name: "1Password",
    column: 1,
    ahead: ["web.vs1password.ahead1", "web.vs1password.ahead2"],
    different: ["web.vs1password.diff1", "web.vs1password.diff2", "web.vs1password.diff3", "web.vs1password.diff4"],
    steps: "web.switchPage.src.1password.text",
  },
  vsBitwarden: {
    id: "vsBitwarden",
    name: "Bitwarden",
    column: 2,
    ahead: ["web.vsBitwarden.ahead1", "web.vsBitwarden.ahead2"],
    different: ["web.vsBitwarden.diff1", "web.vsBitwarden.diff2", "web.vsBitwarden.diff3", "web.vsBitwarden.diff4", "web.vsBitwarden.diff5"],
    steps: "web.switchPage.src.bitwarden.text",
  },
};

export function VendorPage({ ctx, id }: { ctx: Ctx; id: Vendor["id"] }) {
  const { t } = ctx;
  const vendor = VENDORS[id];
  const columns = [0, vendor.column];
  const title = t("web.compare.vsName", { vendor: vendor.name });
  return (
    <>
      <PageHero
        ctx={ctx}
        crumbs={[
          ["compare", t("web.compare.eyebrow")],
          [id, title],
        ]}
        title={title}
        sub={t(`web.${id}.sub` as WebKey)}
        className="hero--vendor"
      />
      <Section id="take" className="take">
        <div className="wrap take__grid">
          <p className="statement">{t(`web.${id}.take` as WebKey)}</p>
          <Matrix ctx={ctx} columns={columns} caption={title} />
        </div>
      </Section>
      <Section id="differences" className="diffs" label={title}>
        <div className="wrap diffs__grid">
          <div className="card diffs__card">
            <h2 className="h3">{t(`web.${id}.aheadTitle` as WebKey)}</h2>
            <Checks items={vendor.ahead.map((k) => t(k))} />
          </div>
          <div className="card diffs__card diffs__card--us" data-tint="cobalt">
            <h2 className="h3">{t(`web.${id}.differentTitle` as WebKey)}</h2>
            <Checks items={vendor.different.map((k) => t(k))} />
          </div>
        </div>
      </Section>
      <Section id="move" className="move">
        <div className="wrap move__grid">
          <div>
            <Eyebrow>{t("web.switch.eyebrow")}</Eyebrow>
            <h2 id="move-title" className="h2">
              {t("web.compare.detail.switchTitle", { vendor: vendor.name })}
            </h2>
          </div>
          <div className="card move__card">
            <p className="lead">{t(`web.${id}.cta` as WebKey)}</p>
            <p>{t(vendor.steps)}</p>
            <ArrowLink href={ctx.href("switch")} label={t("web.compare.switch.link")} />
          </div>
        </div>
      </Section>
      <SwitchBlock ctx={ctx} src={id} />
      <PageCta ctx={ctx} title={t("web.download.title")} text={t("web.download.body")} />
      <Sources ctx={ctx} numbers={citedSources(ROWS, columns)} />
    </>
  );
}

function OnlyMosaic({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const only = [
    ["01", "web.only.cloud.title", "web.only.cloud.text"],
    ["02", "web.only.notice.title", "web.only.notice.text"],
    ["03", "web.only.guess.title", "web.only.guess.text"],
  ] as const;
  return (
    <Section id="only" className="only">
      <div className="wrap">
        <Head id="only" eyebrow={t("web.only.eyebrow")} title={t("web.compare.only.title")} />
        <ol className="reasons__list">
          {only.map(([n, title, text]) => (
            <StepCard key={n} n={n} title={t(title)}>
              <p>{t(text)}</p>
            </StepCard>
          ))}
        </ol>
      </div>
    </Section>
  );
}

function Ahead({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="ahead" className="ahead">
      <div className="wrap ahead__grid">
        <h2 id="ahead-title" className="h2">
          {t("web.compare.ahead.title")}
        </h2>
        <ul className="ahead__list">
          {(["web.compare.ahead.track", "web.compare.ahead.open", "web.compare.ahead.aliases"] as const).map((k) => {
            const [lead = "", ...rest] = t(k).split(/(?<=\.)\s/);
            return (
              <li key={k} className="card">
                <strong>{lead}</strong> {rest.join(" ")}
              </li>
            );
          })}
        </ul>
      </div>
    </Section>
  );
}

function Pick({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const picks = [
    ["web.compare.pick.r1", t("web.compare.pick.or", { a: "1Password", b: "Bitwarden" })],
    ["web.compare.pick.r2", "Bitwarden"],
    ["web.compare.pick.r3", "Proton Pass"],
    ["web.compare.pick.r4", "KeePassXC"],
    ["web.compare.pick.r5", t("web.brand.name")],
  ] as const satisfies readonly (readonly [WebKey, string])[];
  return (
    <Section id="pick" className="pick">
      <div className="wrap">
        <Head id="pick" title={t("web.compare.pick.title")} intro={t("web.compare.pick.intro")} />
        <table className="pick__table">
          <thead>
            <tr>
              <th scope="col">{t("web.compare.pick.ifYou")}</th>
              <th scope="col">{t("web.compare.pick.pick")}</th>
            </tr>
          </thead>
          <tbody>
            {picks.map(([k, pick]) => (
              <tr key={k}>
                <td>{t(k)}</td>
                <th scope="row">{pick}</th>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Section>
  );
}

function DetailCards({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <Section id="detail" className="detail-cards">
      <div className="wrap">
        <h2 id="detail-title" className="h2">
          {t("web.compare.detail.title")}
        </h2>
        <ul className="detail-cards__list">
          {(
            [
              ["vs1password", "1Password", "web.compare.detail.vs1password"],
              ["vsBitwarden", "Bitwarden", "web.compare.detail.vsBitwarden"],
            ] as const
          ).map(([id, vendor, line]) => (
            <li key={id}>
              <a className="vs-card" href={ctx.href(id)}>
                <span className="vs-card__title">{t("web.compare.vsName", { vendor })}</span>
                <span className="vs-card__line">{t(line)}</span>
                <Icon name="arrowRight" className="icon--nudge" />
              </a>
            </li>
          ))}
        </ul>
      </div>
    </Section>
  );
}
