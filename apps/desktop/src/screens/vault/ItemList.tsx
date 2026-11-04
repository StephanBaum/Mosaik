/** The item list: searched with a 150 ms debounce, windowed (virtual scrolling) past 200 rows. */
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { Avatar } from "../../components/Avatar.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { Icon } from "../../components/icons.tsx";
import { useLatest } from "../../hooks/useLatest.ts";
import { renderedIndices, useWindowedRows } from "../../hooks/useWindowedRows.ts";
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
import { UserText } from "../../components/UserText.tsx";

export const SEARCH_DEBOUNCE_MS = 150;
/** Longer lists render only the rows near the viewport. */
export const WINDOW_AFTER = 200;

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
        <span className="item-row__title">
          <UserText>{title}</UserText>
        </span>
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
          <span className="item-row__subtitle">
            <UserText>{item.subtitle ?? t(`item.kind.${item.kind}`)}</UserText>
          </span>
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

/**
 * When the open item leaves the list (trashed, moved, purged) the button that was focused
 * is gone: continue at the row that took its place, or at "New item".
 */
function useFocusAfterRemoval(
  items: readonly ItemSummary[] | null,
  selected: ItemRequest | null,
  list: RefObject<HTMLUListElement | null>,
) {
  const open = useLatest(selected);
  const index = useRef<number | null>(null);
  useEffect(() => {
    if (selected !== null && items !== null) {
      const at = items.findIndex((item) => item.vaultId === selected.vaultId && item.itemId === selected.itemId);
      index.current = at >= 0 ? at : index.current;
    }
  }, [items, selected]);
  useEffect(() => {
    const last = index.current;
    if (items === null || open.current !== null || last === null) {
      return;
    }
    index.current = null;
    if (document.activeElement === null || document.activeElement === document.body) {
      const at = Math.min(last, items.length - 1);
      const row = list.current?.querySelector<HTMLElement>(`[data-row="${String(at)}"] .item-row`);
      (row ?? document.querySelector<HTMLElement>("[data-new-item]"))?.focus();
    }
  }, [items, open, list]);
}

/**
 * The rows themselves. Past `WINDOW_AFTER` only the rows near the viewport are rendered;
 * the open row and the focused row (with its neighbours, for Tab) stay rendered when
 * scrolled away, so neither the selection nor keyboard focus is lost.
 */
function Rows({
  items,
  selected,
  flags,
  now,
  list,
  onSelect,
}: {
  items: readonly ItemSummary[];
  selected: ItemRequest | null;
  flags: Map<string, InsightKind>;
  now: number | null;
  list: RefObject<HTMLUListElement | null>;
  onSelect: (item: ItemRequest) => void;
}) {
  const { t } = useI18n();
  const count = items.length;
  const windowed = count > WINDOW_AFTER;
  const range = useWindowedRows(list, ".item-row", count, windowed);
  const [focused, setFocused] = useState<number | null>(null);
  const isOpen = (item: ItemSummary) => selected?.vaultId === item.vaultId && selected.itemId === item.itemId;
  const open = useMemo(
    () => items.findIndex((item) => selected?.vaultId === item.vaultId && selected.itemId === item.itemId),
    [items, selected],
  );
  const pinned = focused === null ? [open] : [open, focused - 1, focused, focused + 1];
  const rows = windowed ? renderedIndices(range, pinned, count) : items.map((_, index) => index);

  return (
    <ul
      ref={list}
      className={windowed ? "item-list item-list--windowed" : "item-list"}
      aria-label={t("vault.itemCount", { count })}
      onFocus={(event) => {
        const row =
          event.target instanceof Element ? event.target.closest<HTMLElement>("[data-row]")?.dataset["row"] : undefined;
        setFocused(row === undefined ? null : Number(row));
      }}
      onBlur={(event) => {
        if (!(event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget))) {
          setFocused(null);
        }
      }}
    >
      {rows.map((index) => {
        const item = items[index];
        return (
          item !== undefined && (
            <li
              key={`${item.vaultId}/${item.itemId}`}
              data-row={index}
              aria-setsize={windowed ? count : undefined}
              aria-posinset={windowed ? index + 1 : undefined}
              ref={windowed ? (element) => element?.style.setProperty("--row", String(index)) : undefined}
            >
              <ItemRow
                item={item}
                selected={isOpen(item)}
                flag={flags.get(`${item.vaultId}/${item.itemId}`)}
                now={now}
                onSelect={() => {
                  onSelect({ vaultId: item.vaultId, itemId: item.itemId });
                }}
              />
            </li>
          )
        );
      })}
    </ul>
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
  const flags = useMemo(() => flagsOf(findings), [findings]);
  const list = useRef<HTMLUListElement>(null);
  useFocusAfterRemoval(items, selected, list);
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
          <Rows
            items={items}
            selected={selected}
            flags={flags}
            now={trash ? now : null}
            list={list}
            onSelect={onSelect}
          />
        </>
      )}
    </div>
  );
}
