/**
 * The security report (D13): counts per kind, the findings to fix first, and the
 * attribution for the 2FA data. Everything is computed on this device.
 */
import type { MessageKey, MessageValues } from "@mosaic/i18n";
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import {
  insightKinds,
  type InsightFinding,
  type InsightIssue,
  type IpcError,
  type ItemRequest,
  type SecurityReport,
} from "../ipc/contract/index.ts";

type Translate = (key: MessageKey, values?: MessageValues) => string;

function messageOf(t: Translate, finding: InsightFinding, issue: InsightIssue): string {
  switch (issue.kind) {
    case "weak":
    case "old":
      return t(`insights.finding.${issue.kind}`);
    case "reused":
      return t("insights.finding.reused", { count: finding.reusedWith.length });
    case "missingTotp":
      return t("insights.finding.missingTotp", { site: finding.totpSite?.name ?? "" });
    case "insecureUrl":
      return t("insights.finding.insecureUrl", { url: finding.insecureHost ?? "" });
  }
}

function Counts({ report }: { report: SecurityReport }) {
  const { t } = useI18n();
  return (
    <ul className="counts">
      {insightKinds.map((kind) => (
        <li key={kind} className="counts__item">
          <span className="counts__number">{report.counts[kind]}</span>
          <span>{t(`insights.kind.${kind}`)}</span>
          <span className="muted">
            {kind === "reused"
              ? t("insights.kind.reused.description", { groups: report.counts.reusedGroups })
              : t(`insights.kind.${kind}.description`)}
          </span>
        </li>
      ))}
    </ul>
  );
}

function FindingRow({
  finding,
  title,
  onOpen,
}: {
  finding: InsightFinding;
  title: string;
  onOpen: () => void;
}) {
  const { t } = useI18n();
  return (
    <li className="findings__item">
      <strong>{title}</strong>
      <ul>
        {finding.issues.map((issue) => (
          <li key={issue.kind}>
            <span className="badge">{t(`insights.severity.${issue.severity}`)}</span>{" "}
            {messageOf(t, finding, issue)}
            {issue.kind === "missingTotp" && finding.totpSite?.documentation != null && (
              <span className="muted">
                {" "}
                {t("insights.documentation", { url: finding.totpSite.documentation })}
              </span>
            )}
          </li>
        ))}
      </ul>
      <Button aria-label={t("insights.openItem", { title })} onClick={onOpen}>
        {t(`insights.action.${finding.issues[0]?.action ?? "reviewUrl"}`)}
      </Button>
    </li>
  );
}

function useReport(round: number) {
  const ipc = useIpc();
  const { t } = useI18n();
  const [report, setReport] = useState<SecurityReport | null>(null);
  const [titles, setTitles] = useState<ReadonlyMap<string, string>>(new Map());
  const [error, setError] = useState<IpcError | null>(null);
  useEffect(() => {
    let current = true;
    void Promise.all([
      ipc.call("security_report", null),
      ipc.call("list_items", { vaultId: null, trash: "exclude", favoritesOnly: false, query: null }),
    ]).then(([reportResult, items]) => {
      if (!current) {
        return;
      }
      setError(reportResult.ok ? null : reportResult.error);
      if (reportResult.ok) {
        setReport(reportResult.value);
      }
      if (items.ok) {
        setTitles(new Map(items.value.map((item) => [item.itemId, item.title ?? t("vault.untitled")])));
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, t, round]);
  return { report, titles, error };
}

export function SecurityDashboard({ onOpenItem }: { onOpenItem: (item: ItemRequest) => void }) {
  const { t } = useI18n();
  const [round, setRound] = useState(0);
  const [showAll, setShowAll] = useState(false);
  const { report, titles, error } = useReport(round);

  const findings = report?.findings ?? [];
  const urgent = findings.filter((f) => f.issues.some((issue) => issue.severity === "high"));
  const listed = showAll ? findings : urgent;
  return (
    <section className="stack" aria-label={t("insights.title")}>
      <h1 className="title">{t("insights.title")}</h1>
      <ErrorAlert error={error} />
      {report !== null && (
        <>
          <p className="body muted">
            {t("insights.intro", { checkedAt: report.generatedAt, count: report.itemsChecked })}
          </p>
          <Button
            onClick={() => {
              setRound((n) => n + 1);
            }}
          >
            {t("insights.checkAgain")}
          </Button>
          <h2 className="subtitle">{t("insights.summary")}</h2>
          <Counts report={report} />
          {findings.length === 0 ? (
            <p className="body muted">{t("insights.empty")}</p>
          ) : (
            <>
              <h2 className="subtitle">{t("insights.fixFirst")}</h2>
              <ul className="findings">
                {listed.map((finding) => (
                  <FindingRow
                    key={finding.itemId}
                    finding={finding}
                    title={titles.get(finding.itemId) ?? t("vault.untitled")}
                    onOpen={() => {
                      onOpenItem({ vaultId: finding.vaultId, itemId: finding.itemId });
                    }}
                  />
                ))}
              </ul>
              {!showAll && findings.length > urgent.length && (
                <Button
                  onClick={() => {
                    setShowAll(true);
                  }}
                >
                  {t("insights.showAll", { count: findings.length })}
                </Button>
              )}
            </>
          )}
          <p className="footnote muted">{t("insights.attribution.twoFactor")}</p>
        </>
      )}
    </section>
  );
}
