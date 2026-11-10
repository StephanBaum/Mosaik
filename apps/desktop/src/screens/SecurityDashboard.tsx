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
  type BreachStatus,
  type InsightFinding,
  type InsightKind,
  type IpcError,
  type ItemRequest,
  type SecurityReport,
} from "../ipc/contract/index.ts";
import { BreachCheckCard } from "./rotation/BreachCheckCard.tsx";
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

/** "Change password" goes to the guided panel when the page offers one; every other action opens the item. */
function openFinding(
  finding: InsightFinding,
  onChangePassword: (item: ItemRequest) => void,
  onOpenItem: (item: ItemRequest) => void,
): void {
  const item = { vaultId: finding.vaultId, itemId: finding.itemId };
  (finding.issues[0]?.action === "changePassword" ? onChangePassword : onOpenItem)(item);
}

/** "Fix these first": the listed findings with their actions, and "Show all". */
function FindingsSection({
  findings,
  listed,
  total,
  showAll,
  titles,
  onShowAll,
  onOpen,
}: {
  findings: readonly InsightFinding[];
  listed: readonly InsightFinding[];
  total: number;
  showAll: boolean;
  titles: ReadonlyMap<string, string>;
  onShowAll: () => void;
  onOpen: (finding: InsightFinding) => void;
}) {
  const { t } = useI18n();
  if (findings.length === 0) {
    return <p className="lede">{t("insights.empty")}</p>;
  }
  return (
    <section className="security__section">
      <h2 className="subtitle">{t("insights.fixFirst")}</h2>
      <ol className="rows rows--closed">
        {listed.map((finding) => (
          <FindingRow
            key={finding.itemId}
            finding={finding}
            title={titles.get(finding.itemId) ?? t("vault.untitled")}
            onOpen={() => {
              onOpen(finding);
            }}
          />
        ))}
      </ol>
      {listed.length === 0 && <p className="caption">{t("insights.empty")}</p>}
      {!showAll && total > listed.length && (
        <div>
          <Button onClick={onShowAll}>{t("insights.showAll", { count: total })}</Button>
        </div>
      )}
    </section>
  );
}

export function SecurityDashboard({
  onOpenItem,
  onChangePassword,
  refreshKey = 0,
  breach = null,
  onBreachChanged,
}: {
  onOpenItem: (item: ItemRequest) => void;
  /** "Change password" opens the guided panel of the rotate-now queue; without it the item opens. */
  onChangePassword?: (item: ItemRequest) => void;
  /** Bumped by the page when something outside the dashboard changed the findings. */
  refreshKey?: number;
  /** The breach check's status; the card shows once it is known. */
  breach?: BreachStatus | null;
  onBreachChanged?: () => void;
}) {
  const { t } = useI18n();
  const [localRound, setRound] = useState(0);
  const round = localRound + refreshKey;
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
          {breach !== null && (
            <BreachCheckCard
              status={breach}
              found={report.counts.breached}
              onChanged={() => {
                setRound((n) => n + 1);
                onBreachChanged?.();
              }}
            />
          )}
          <FindingsSection
            findings={findings}
            listed={listed}
            total={total}
            showAll={showAll}
            titles={titles}
            onShowAll={() => {
              setShowAll(true);
            }}
            onOpen={(finding) => {
              openFinding(finding, onChangePassword ?? onOpenItem, onOpenItem);
            }}
          />
          <p className="caption">{t("insights.attribution.twoFactor")}</p>
        </>
      )}
    </section>
  );
}
