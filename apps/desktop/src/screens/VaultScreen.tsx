/**
 * The vault (placeholder for the vault UI track): sidebar with vaults, searchable item
 * list, lock. Shows summaries only; secrets stay in Rust.
 */
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type {
  IpcError,
  ItemSummary,
  SessionState,
  VaultId,
  VaultSummary,
} from "../ipc/contract/index.ts";

export function VaultScreen({ onLocked }: { onLocked: (session: SessionState) => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  const [items, setItems] = useState<ItemSummary[] | null>(null);
  const [vaultId, setVaultId] = useState<VaultId | null>(null);
  const [query, setQuery] = useState("");
  const [error, setError] = useState<IpcError | null>(null);

  useEffect(() => {
    void ipc.call("list_vaults", null).then((result) => {
      if (result.ok) {
        setVaults(result.value);
      } else {
        setError(result.error);
      }
    });
  }, [ipc]);

  useEffect(() => {
    let current = true;
    void ipc
      .call("list_items", {
        vaultId,
        trash: "exclude",
        favoritesOnly: false,
        query: query.trim() === "" ? null : query,
      })
      .then((result) => {
        if (!current) {
          return;
        }
        if (result.ok) {
          setItems(result.value);
        } else {
          setError(result.error);
        }
      });
    return () => {
      current = false;
    };
  }, [ipc, vaultId, query]);

  async function lock() {
    const result = await ipc.call("lock", null);
    if (result.ok) {
      onLocked(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="vault">
      <header className="vault__header">
        <span className="vault__brand">{t("app.name")}</span>
        <Button
          variant="ghost"
          onClick={() => {
            void lock();
          }}
        >
          {t("action.lock")}
        </Button>
      </header>
      <nav className="vault__sidebar" aria-label={t("vault.vaults")}>
        <ul className="nav-list">
          <li>
            <button
              type="button"
              className="nav-list__item"
              aria-current={vaultId === null ? "page" : undefined}
              onClick={() => {
                setVaultId(null);
              }}
            >
              {t("vault.allItems")}
            </button>
          </li>
          {vaults.map((vault) => (
            <li key={vault.vaultId}>
              <button
                type="button"
                className="nav-list__item"
                aria-current={vaultId === vault.vaultId ? "page" : undefined}
                onClick={() => {
                  setVaultId(vault.vaultId);
                }}
              >
                {vault.name ?? t("vault.unnamed")}
              </button>
            </li>
          ))}
        </ul>
      </nav>
      <main id="main" className="vault__main">
        <h1 className="visually-hidden">{t("vault.items")}</h1>
        <input
          className="field__input search"
          type="search"
          aria-label={t("vault.search")}
          placeholder={t("vault.search")}
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        {items !== null && items.length === 0 && (
          <p className="empty muted">
            {query.trim() === "" ? t("vault.empty") : t("vault.noResults", { query })}
          </p>
        )}
        {items !== null && items.length > 0 && (
          <ul className="item-list" aria-label={t("vault.itemCount", { count: items.length })}>
            {items.map((item) => (
              <li key={`${item.vaultId}/${item.itemId}`} className="item-list__item">
                <span className="item-list__title">{item.title ?? t("vault.untitled")}</span>
                {item.subtitle !== null && (
                  <span className="item-list__subtitle muted">{item.subtitle}</span>
                )}
              </li>
            ))}
          </ul>
        )}
      </main>
    </div>
  );
}
