/**
 * The activity list of the agent settings: what agents asked for, newest first, with the
 * refusals marked and their reason in plain words. An entry holds names, sites and sizes, never
 * a value, a header, a body or a query string. Filters by client and by key; "Load more" pages
 * back in time with `beforeMs`.
 */
import { useEffect, useMemo, useState } from "react";
import { Button } from "../components/Button.tsx";
import { Select } from "../components/Select.tsx";
import { UserText } from "../components/UserText.tsx";
import { formatDate, formatTime } from "../i18n/format.ts";
import { useI18n, type I18n } from "../i18n/I18nProvider.tsx";
import type { AgentAccessStatus, AgentLogPage, AgentLogView } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";

const PAGE = 50;
const ALL = "";

/** The facts of an entry after the sentence: the request, the answer, what was hidden, why refused. */
function detailsOf(entry: AgentLogView, t: I18n["t"]): string[] {
  const parts: (string | null)[] = [
    entry.method !== null && entry.origin !== null ? `${entry.method} ${entry.origin}${entry.path ?? ""}` : null,
    entry.status === null ? null : t("agent.activity.status", { status: entry.status }),
    entry.masked === null || entry.masked === 0 ? null : t("agent.activity.masked", { count: entry.masked }),
    entry.code === null ? null : t("agent.activity.refusedWith", { reason: t(`agent.refusal.${entry.code}`) }),
  ];
  return parts.filter((part): part is string => part !== null);
}

function Entry({ entry }: { entry: AgentLogView }) {
  const { t, locale } = useI18n();
  const client = entry.pairingName ?? t("agent.activity.unknownClient");
  const item = entry.itemTitle ?? t("agent.activity.unknownItem");
  const refused = entry.kind === "refused" || entry.kind === "accessDenied";
  const details = detailsOf(entry, t);
  return (
    <li className={refused ? "agent-activity__entry agent-activity__entry--refused" : "agent-activity__entry"}>
      <p className="agent-activity__line">
        <UserText>{t(`agent.activity.kind.${entry.kind}`, { client, item })}</UserText>
      </p>
      {details.length > 0 && (
        <p className="caption" translate="no">
          {details.join(" · ")}
        </p>
      )}
      <p className="caption">
        {formatDate(locale, entry.atMs)} {formatTime(locale, entry.atMs)}
      </p>
    </li>
  );
}

interface Filters {
  pairingId: string;
  itemId: string;
}

/** The newest page for the filters (read again when they, the pairings or the grants change). */
function useActivity(filters: Filters, refresh: number) {
  const ipc = useIpc();
  const [page, setPage] = useState<AgentLogPage | null>(null);
  const [older, setOlder] = useState<AgentLogPage[]>([]);
  const { pairingId, itemId } = filters;
  const request = useMemo(
    () => ({
      pairingId: pairingId === ALL ? null : pairingId,
      itemId: itemId === ALL ? null : itemId,
      limit: PAGE,
    }),
    [pairingId, itemId],
  );

  useEffect(() => {
    let current = true;
    void ipc.call("agent_log", { ...request, beforeMs: null }).then((result) => {
      if (current && result.ok) {
        setPage(result.value);
        setOlder([]);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, request, refresh]);

  const pages = useMemo(() => (page === null ? [] : [page, ...older]), [page, older]);
  const entries = useMemo(() => pages.flatMap((each) => each.entries), [pages]);
  const skipped = pages.reduce((sum, each) => sum + each.skipped, 0);
  const last = entries[entries.length - 1];
  const lastPage = pages[pages.length - 1];
  const more = lastPage !== undefined && lastPage.entries.length >= PAGE;

  async function loadMore() {
    if (last === undefined) {
      return;
    }
    const result = await ipc.call("agent_log", { ...request, beforeMs: last.atMs });
    if (result.ok) {
      setOlder((old) => [...old, result.value]);
    }
  }
  return { entries, skipped, more, loadMore };
}

export function AgentActivity({ status }: { status: AgentAccessStatus }) {
  const { t } = useI18n();
  const [filters, setFilters] = useState<Filters>({ pairingId: ALL, itemId: ALL });
  const { grants, pairings } = status;
  const { entries, skipped, more, loadMore } = useActivity(filters, pairings.length + grants.length);

  // The keys that appear in the list so far, for the key filter.
  const keys = useMemo(() => {
    const seen = new Map<string, string>();
    for (const entry of entries) {
      if (entry.itemId !== null) {
        seen.set(entry.itemId, entry.itemTitle ?? t("agent.activity.unknownItem"));
      }
    }
    return [...seen.entries()].map(([value, label]) => ({ value, label }));
  }, [entries, t]);

  return (
    <div className="setting-row setting-row--stacked">
      <h3 className="setting-row__label">{t("agent.activity.title")}</h3>
      <p className="field__hint">{t("agent.activity.body")}</p>
      <div className="agent-activity__filters">
        <Select
          label={t("agent.activity.filterClient")}
          value={filters.pairingId}
          options={[
            { value: ALL, label: t("agent.activity.all") },
            ...pairings.map((pairing) => ({ value: pairing.pairingId, label: pairing.name })),
          ]}
          onChange={(event) => {
            setFilters({ ...filters, pairingId: event.target.value });
          }}
        />
        <Select
          label={t("agent.activity.filterSecret")}
          value={filters.itemId}
          options={[{ value: ALL, label: t("agent.activity.all") }, ...keys]}
          onChange={(event) => {
            setFilters({ ...filters, itemId: event.target.value });
          }}
        />
      </div>
      {entries.length === 0 ? (
        <p className="field__hint">{t("agent.activity.empty")}</p>
      ) : (
        <ul className="rows">
          {entries.map((entry, index) => (
            <Entry key={`${String(entry.atMs)}-${String(index)}`} entry={entry} />
          ))}
        </ul>
      )}
      {skipped > 0 && <p className="caption">{t("agent.activity.skipped", { count: skipped })}</p>}
      {more && (
        <div>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => {
              void loadMore();
            }}
          >
            {t("agent.activity.loadMore")}
          </Button>
        </div>
      )}
    </div>
  );
}
