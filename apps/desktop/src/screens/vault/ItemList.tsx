/** The item list: searched with a 150 ms debounce, rendered progressively past 200 rows. */
import { useEffect, useMemo, useState } from "react";
import { Avatar } from "../../components/Avatar.tsx";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { Icon } from "../../components/icons.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  InsightFinding,
  InsightKind,
  IpcError,
  ItemRequest,
  ItemSummary,
  VaultId,
} from "../../ipc/contract/index.ts";
import { daysLeft, TRASH_URGENT_DAYS } from "../../time/trash.ts";
import { matchesCategory, type Category } from "./useVaultData.ts";

export const SEARCH_DEBOUNCE_MS = 150;
const PAGE = 200;

interface Query {
  vaultId: VaultId | null;
  trash: boolean;
  query: string;
  category: Category;
  reloadKey: number;
}

function useItems({ vaultId, trash, query, category, reloadKey }: Query, onLoaded?: (count: number) => void) {
  const ipc = useIpc();
  const [items, setItems] = useState<ItemSummary[] | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  useEffect(() => {
    let current = true;
    const timer = setTimeout(
      () => {
        void ipc
          .call("list_items", {
            vaultId,
            trash: trash ? "only" : "exclude",
            favoritesOnly: false,
            query: query.trim() === "" ? null : query,
          })
          .then((result) => {
            if (!current) {
              return;
            }
            setError(result.ok ? null : result.error);
            if (result.ok) {
              const shown = result.value.filter((item) => matchesCategory(item, category));
              setItems(shown);
              onLoaded?.(shown.length);
            }
          });
      },
      query === "" ? 0 : SEARCH_DEBOUNCE_MS,
    );
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [ipc, vaultId, trash, query, category, reloadKey, onLoaded]);
  return { items, error };
}

const FLAGGED: readonly InsightKind[] = ["reused", "weak"];

/** The first reused or weak issue per item, for the row flag. */
function flagsOf(findings: readonly InsightFinding[]): Map<string, InsightKind> {
  const flags = new Map<string, InsightKind>();
  for (const finding of findings) {
    const issue = finding.issues.find((candidate) => FLAGGED.includes(candidate.kind));
    if (issue !== undefined) {
      flags.set(`${finding.vaultId}/${finding.itemId}`, issue.kind);
    }
  }
  return flags;
}

function ItemRow({
  item,
  selected,
  flag,
  now,
  onSelect,
}: {
  item: ItemSummary;
  selected: boolean;
  flag: InsightKind | undefined;
  /** Set in the trash view: the row then shows the deletion date and the days left. */
  now: number | null;
  onSelect: () => void;
}) {
  const { t } = useI18n();
  const title = item.title ?? t("vault.untitled");
  const trashed = now !== null && item.trashedAt !== null ? daysLeft(item.trashedAt, now) : null;
  return (
    <button
      type="button"
      className="item-row"
      aria-current={selected ? "true" : undefined}
      onClick={onSelect}
    >
      <Avatar title={title} selected={selected} />
      <span className="item-row__text">
        <span className="item-row__title">{title}</span>
        {trashed !== null && item.trashedAt !== null ? (
          <span
            className={
              trashed <= TRASH_URGENT_DAYS
                ? "item-row__subtitle item-row__subtitle--urgent"
                : "item-row__subtitle"
            }
          >
            {trashed <= TRASH_URGENT_DAYS && <Icon name="clock" size="sm" />}
            {t("trash.rowMeta", { deletedAt: new Date(item.trashedAt), days: trashed })}
          </span>
        ) : (
          <span className="item-row__subtitle">{item.subtitle ?? t(`item.kind.${item.kind}`)}</span>
        )}
      </span>
      {flag !== undefined ? (
        <span className="item-row__flag">
          <Icon name="alert" size="sm" />
          {t(`insights.kind.${flag}`)}
        </span>
      ) : (
        item.hasOtp && <span className="item-row__meta">{t("vault.flag.otp")}</span>
      )}
    </button>
  );
}

export function ItemList({
  vaultId,
  trash,
  query,
  category,
  findings,
  selected,
  reloadKey,
  onSelect,
  onLoaded,
}: {
  vaultId: VaultId | null;
  trash: boolean;
  query: string;
  category: Category;
  findings: readonly InsightFinding[];
  selected: ItemRequest | null;
  /** Changes when something was added, edited or removed. */
  reloadKey: number;
  onSelect: (item: ItemRequest) => void;
  onLoaded?: (count: number) => void;
}) {
  const { t } = useI18n();
  const { items, error } = useItems({ vaultId, trash, query, category, reloadKey }, onLoaded);
  const [shown, setShown] = useState(PAGE);
  const flags = useMemo(() => flagsOf(findings), [findings]);
  // Read once per mount: whole days left do not need a ticking clock.
  const [now] = useState(() => Date.now());

  return (
    <div className="list-pane">
      <ErrorAlert error={error} />
      {trash && (
        <div className="list-pane__title">
          <h2 className="subtitle">{t("nav.trash")}</h2>
          <p className="caption">{t("trash.retention")}</p>
        </div>
      )}
      {items !== null && items.length === 0 && (query.trim() !== "" || trash) && (
        <p className="empty muted">
          {query.trim() !== "" ? t("vault.noResults", { query }) : t("vault.trashEmpty")}
        </p>
      )}
      {items !== null && items.length > 0 && (
        <>
          <div className="list-pane__header">
            <span>{t("vault.itemCount", { count: items.length })}</span>
          </div>
          <ul className="item-list" aria-label={t("vault.itemCount", { count: items.length })}>
            {items.slice(0, shown).map((item) => (
              <li key={`${item.vaultId}/${item.itemId}`}>
                <ItemRow
                  item={item}
                  selected={selected?.vaultId === item.vaultId && selected.itemId === item.itemId}
                  flag={flags.get(`${item.vaultId}/${item.itemId}`)}
                  now={trash ? now : null}
                  onSelect={() => {
                    onSelect({ vaultId: item.vaultId, itemId: item.itemId });
                  }}
                />
              </li>
            ))}
          </ul>
        </>
      )}
      {items !== null && items.length > shown && (
        <Button
          onClick={() => {
            setShown((n) => n + PAGE);
          }}
        >
          {t("insights.showAll", { count: items.length })}
        </Button>
      )}
    </div>
  );
}
