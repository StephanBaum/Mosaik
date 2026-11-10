/** The security report's building blocks: the summary cards and one finding row. */
import type { MessageKey, MessageValues } from "@mosaic/i18n";
import { Avatar } from "../components/Avatar.tsx";
import { Button } from "../components/Button.tsx";
import { Icon, type IconName } from "../components/icons.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import {
  insightKinds,
  type InsightFinding,
  type InsightIssue,
  type InsightKind,
  type SecurityReport,
} from "../ipc/contract/index.ts";

type Translate = (key: MessageKey, values?: MessageValues) => string;

const KIND_ICON = {
  weak: { icon: "alert", tone: "danger" },
  reused: { icon: "copy", tone: "warning" },
  old: { icon: "clock", tone: "muted" },
  missingTotp: { icon: "smartphone", tone: "muted" },
  insecureUrl: { icon: "lockOpen", tone: "warning" },
  breached: { icon: "shieldAlert", tone: "danger" },
} as const satisfies Record<InsightKind, { icon: IconName; tone: "danger" | "warning" | "muted" }>;

export function messageOf(t: Translate, finding: InsightFinding, issue: InsightIssue): string {
  switch (issue.kind) {
    case "weak":
    case "old":
    case "breached":
      return t(`insights.finding.${issue.kind}`);
    case "reused":
      return t("insights.finding.reused", { count: finding.reusedWith.length });
    case "missingTotp":
      return t("insights.finding.missingTotp", { site: finding.totpSite?.name ?? "" });
    case "insecureUrl":
      return t("insights.finding.insecureUrl", { url: finding.insecureHost ?? "" });
  }
}

export function Counts({
  report,
  filter,
  onFilter,
}: {
  report: SecurityReport;
  filter: InsightKind | null;
  onFilter: (kind: InsightKind | null) => void;
}) {
  const { t } = useI18n();
  return (
    <ul className="stat-grid">
      {insightKinds.map((kind) => (
        <li key={kind} className="stat-grid__item">
          <button
            type="button"
            className="card stat"
            aria-pressed={filter === kind}
            onClick={() => {
              onFilter(filter === kind ? null : kind);
            }}
          >
            <Icon name={KIND_ICON[kind].icon} className={`stat__icon stat__icon--${KIND_ICON[kind].tone}`} />
            <span className="stat__number tabular">{report.counts[kind]}</span>
            <span className="stat__label">{t(`insights.kind.${kind}`)}</span>
            <span className="caption">
              {kind === "reused"
                ? t("insights.kind.reused.description", { groups: report.counts.reusedGroups })
                : t(`insights.kind.${kind}.description`)}
            </span>
          </button>
        </li>
      ))}
    </ul>
  );
}

export function FindingRow({
  finding,
  title,
  onOpen,
}: {
  finding: InsightFinding;
  title: string;
  onOpen: () => void;
}) {
  const { t } = useI18n();
  const action = t(`insights.action.${finding.issues[0]?.action ?? "reviewUrl"}`);
  return (
    <li className="row">
      <Avatar title={title} />
      <div className="row__main">
        <span className="row__value">{title}</span>
        {finding.issues.map((issue) => (
          <span key={issue.kind} className={`issue issue--${issue.severity}`}>
            <Icon name={KIND_ICON[issue.kind].icon} size="sm" />
            <span className="visually-hidden">{t(`insights.severity.${issue.severity}`)}: </span>
            {messageOf(t, finding, issue)}
            {issue.kind === "missingTotp" && finding.totpSite?.documentation != null && (
              <span className="muted">
                {" "}
                {t("insights.documentation", { url: finding.totpSite.documentation })}
              </span>
            )}
          </span>
        ))}
      </div>
      {/* The accessible name starts with the visible text (WCAG 2.5.3) and names the item. */}
      <Button size="sm" aria-label={t("insights.actionFor", { action, title })} onClick={onOpen}>
        {action}
      </Button>
    </li>
  );
}
