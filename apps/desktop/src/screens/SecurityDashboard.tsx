/**
 * The security report (D13): counts per kind, the findings to fix first, and the
 * attribution for the 2FA data. Everything is computed on this device.
 */
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { Icon } from "../components/icons.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import {
  type InsightFinding,
  type InsightKind,
  type IpcError,
  type ItemRequest,
  type SecurityReport,
} from "../ipc/contract/index.ts";
import { Counts, FindingRow } from "./SecurityParts.tsx";

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

/** The findings to list: the kind filter first, then high severity, else the first five. */
export function listFindings(
  findings: readonly InsightFinding[],
  filter: InsightKind | null,
  showAll: boolean,
): { listed: readonly InsightFinding[]; total: number } {
  const base = filter === null ? findings : findings.filter((f) => f.issues.some((i) => i.kind === filter));
  const urgent = base.filter((f) => f.issues.some((issue) => issue.severity === "high"));
  const listed = showAll ? base : urgent.length > 0 ? urgent : base.slice(0, 5);
  return { listed, total: base.length };
}

export function SecurityDashboard({ onOpenItem }: { onOpenItem: (item: ItemRequest) => void }) {
  const { t } = useI18n();
  const [round, setRound] = useState(0);
  const [showAll, setShowAll] = useState(false);
  const [filter, setFilter] = useState<InsightKind | null>(null);
  const { report, titles, error } = useReport(round);

  const findings = report?.findings ?? [];
  const { listed, total } = listFindings(findings, filter, showAll);
  return (
    <section className="security" aria-label={t("insights.title")}>
      <div className="page__header">
        <div className="page__heading">
          <h1 className="display">{t("insights.title")}</h1>
          {report !== null && (
            <p className="lede">
              {t("insights.intro", { checkedAt: report.generatedAt, count: report.itemsChecked })}
            </p>
          )}
        </div>
        {report !== null && (
          <Button
            onClick={() => {
              setRound((n) => n + 1);
            }}
          >
            <Icon name="refresh" />
            {t("insights.checkAgain")}
          </Button>
        )}
      </div>
      <ErrorAlert error={error} />
      {report !== null && (
        <>
          <section className="security__section">
            <h2 className="subtitle">{t("insights.summary")}</h2>
            <Counts report={report} filter={filter} onFilter={setFilter} />
          </section>
          {findings.length === 0 ? (
            <p className="lede">{t("insights.empty")}</p>
          ) : (
            <section className="security__section">
              <h2 className="subtitle">{t("insights.fixFirst")}</h2>
              <ol className="rows rows--closed">
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
              </ol>
              {listed.length === 0 && <p className="caption">{t("insights.empty")}</p>}
              {!showAll && total > listed.length && (
                <div>
                  <Button
                    onClick={() => {
                      setShowAll(true);
                    }}
                  >
                    {t("insights.showAll", { count: total })}
                  </Button>
                </div>
              )}
            </section>
          )}
          <p className="caption">{t("insights.attribution.twoFactor")}</p>
        </>
      )}
    </section>
  );
}
