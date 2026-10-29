/** The item list: searched with a 150 ms debounce, rendered progressively past 200 rows. */
import { useEffect, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError, ItemRequest, ItemSummary, VaultId } from "../../ipc/contract/index.ts";

export const SEARCH_DEBOUNCE_MS = 150;
const PAGE = 200;

interface Query {
  vaultId: VaultId | null;
  trash: boolean;
  query: string;
  reloadKey: number;
}

function useItems({ vaultId, trash, query, reloadKey }: Query, onLoaded?: (count: number) => void) {
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
              setItems(result.value);
              onLoaded?.(result.value.length);
            }
          });
      },
      query === "" ? 0 : SEARCH_DEBOUNCE_MS,
    );
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [ipc, vaultId, trash, query, reloadKey, onLoaded]);
  return { items, error };
}

export function ItemList({
  vaultId,
  trash,
  query,
  selected,
  reloadKey,
  onSelect,
  onLoaded,
}: {
  vaultId: VaultId | null;
  trash: boolean;
  query: string;
  selected: ItemRequest | null;
  /** Changes when something was added, edited or removed. */
  reloadKey: number;
  onSelect: (item: ItemRequest) => void;
  onLoaded?: (count: number) => void;
}) {
  const { t } = useI18n();
  const { items, error } = useItems({ vaultId, trash, query, reloadKey }, onLoaded);
  const [shown, setShown] = useState(PAGE);

  return (
    <div className="list-pane">
      <ErrorAlert error={error} />
      {items !== null && items.length === 0 && (query.trim() !== "" || trash) && (
        <p className="empty muted">
          {query.trim() !== "" ? t("vault.noResults", { query }) : t("vault.trashEmpty")}
        </p>
      )}
      {items !== null && items.length > 0 && (
        <ul className="item-list" aria-label={t("vault.itemCount", { count: items.length })}>
          {items.slice(0, shown).map((item) => {
            const isSelected =
              selected?.vaultId === item.vaultId && selected.itemId === item.itemId;
            return (
              <li key={`${item.vaultId}/${item.itemId}`}>
                <button
                  type="button"
                  className="item-list__item"
                  aria-current={isSelected ? "true" : undefined}
                  onClick={() => {
                    onSelect({ vaultId: item.vaultId, itemId: item.itemId });
                  }}
                >
                  <span className="item-list__title">{item.title ?? t("vault.untitled")}</span>
                  <span className="item-list__subtitle muted">
                    {item.subtitle ?? t(`item.kind.${item.kind}`)}
                  </span>
                </button>
              </li>
            );
          })}
        </ul>
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
