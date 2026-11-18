/**
 * Report a problem and the security challenge (website plan, Task 10). Both only link out to
 * GitHub (issues, private advisories, the rules and targets); the site itself takes no input and
 * asks for no secret (ADR-0087, docs/BUG_BOUNTY.md, docs/CHALLENGE.md).
 */
import type { ReactNode } from "react";
import type { Ctx, WebKey } from "../i18n.ts";
import { LINKS, site } from "../site.ts";
import { ArrowLink, Checks, LineButton, Section } from "../ui.tsx";
import { PageHero } from "./parts.tsx";

function ReportCard({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <article id={id} className="card report__card" aria-labelledby={`${id}-title`}>
      <h2 id={`${id}-title`} className="h3">
        {title}
      </h2>
      {children}
    </article>
  );
}

export function Report({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  return (
    <>
      <PageHero ctx={ctx} eyebrow={t("web.report.eyebrow")} title={t("web.report.title")} sub={t("web.report.sub")} />
      <Section id="report" label={t("web.report.eyebrow")}>
        <div className="wrap report__grid">
          <ReportCard id="bug" title={t("web.report.bug.title")}>
            <p>{t("web.report.bug.text")}</p>
            <ArrowLink href={LINKS.issues} label={t("web.report.bug.link")} external />
          </ReportCard>
          <ReportCard id="vuln" title={t("web.report.vuln.title")}>
            <p>{t("web.report.vuln.text")}</p>
            <ArrowLink href={LINKS.advisory} label={t("web.report.vuln.link")} external />
            <ArrowLink href={LINKS.policy} label={t("web.report.vuln.policy")} external />
          </ReportCard>
          <ReportCard id="never" title={t("web.report.never.title")}>
            <p>{t("web.report.never.text")}</p>
          </ReportCard>
        </div>
        <div className="wrap">
          <ArrowLink href={ctx.href("challenge")} label={t("web.report.challenge")} />
        </div>
      </Section>
    </>
  );
}

const TARGETS = ["C1", "C2", "C3", "C4", "C5", "C6", "C7"] as const;

export function Challenge({ ctx }: { ctx: Ctx }) {
  const { t } = ctx;
  const how = ["web.challenge.how1", "web.challenge.how2", "web.challenge.how3", "web.challenge.how4"] as const;
  return (
    <>
      <PageHero
        ctx={ctx}
        eyebrow={t("web.challenge.eyebrow")}
        title={t("web.challenge.title")}
        sub={t("web.challenge.sub")}
        crumbs={[
          ["security", t("web.nav.security")],
          ["challenge", t("web.challenge.eyebrow")],
        ]}
        actions={
          <>
            <LineButton href={LINKS.rules} label={t("web.challenge.rules")} large external />
            <LineButton href={LINKS.targets} label={t("web.challenge.targetsLink")} large external />
          </>
        }
      />
      <Section id="how" label={t("web.challenge.how.title")}>
        <div className="wrap">
          <p className="lead challenge__status" role="status">
            {t(site.challengeOpen ? "web.challenge.open" : "web.challenge.soon")}
          </p>
          <h2 id="how-title" className="h2">
            {t("web.challenge.how.title")}
          </h2>
          <Checks items={how.map((k) => t(k))} />
        </div>
      </Section>
      <Section id="targets">
        <div className="wrap">
          <h2 id="targets-title" className="h2">
            {t("web.challenge.targets.title")}
          </h2>
          <ol className="targets">
            {TARGETS.map((id) => (
              <li key={id} className="targets__row">
                <span className="targets__id">{id}</span>
                <span>{t(`web.challenge.${id.toLowerCase()}` as WebKey)}</span>
              </li>
            ))}
          </ol>
          <p className="fineprint">{t("web.challenge.scope")}</p>
          <p>{t("web.challenge.reward")}</p>
          <div className="actions">
            <ArrowLink href={LINKS.hallOfFame} label={t("web.challenge.hallOfFame")} external />
            <ArrowLink href={ctx.href("report", "vuln")} label={t("web.challenge.report")} />
          </div>
        </div>
      </Section>
    </>
  );
}
