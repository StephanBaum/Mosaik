/** The command palette (D11): Ctrl+K / ⌘K or the global shortcut. Items and every command. */
import { useEffect, useId, useMemo, useState } from "react";
import { Dialog } from "../components/Dialog.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { ItemRequest, ItemSummary } from "../ipc/contract/index.ts";
import { SEARCH_DEBOUNCE_MS } from "./vault/ItemList.tsx";
import { usePlatform } from "../platform/platform.ts";
import { ariaKeyshortcuts, format, SHORTCUTS, type ShortcutName } from "../platform/shortcuts.ts";
import type { View } from "./vault/VaultLayout.tsx";

interface Entry {
  id: string;
  label: string;
  hint?: string;
  /** The in-app shortcut, shown at the end of the row. */
  shortcut?: ShortcutName;
  run: () => void;
}

interface Actions {
  onClose: () => void;
  onOpenItem: (item: ItemRequest) => void;
  onNavigate: (view: View) => void;
  onNewItem: () => void;
  onLock: () => void;
}

function useFoundItems(query: string): ItemSummary[] {
  const ipc = useIpc();
  const [items, setItems] = useState<ItemSummary[]>([]);
  useEffect(() => {
    let current = true;
    const timer = setTimeout(() => {
      void ipc
        .call("list_items", {
          vaultId: null,
          trash: "exclude",
          favoritesOnly: false,
          query: query.trim() === "" ? null : query,
        })
        .then((result) => {
          if (current && result.ok) {
            setItems(result.value.slice(0, 8));
          }
        });
    }, SEARCH_DEBOUNCE_MS);
    return () => {
      current = false;
      clearTimeout(timer);
    };
  }, [ipc, query]);
  return items;
}

function useEntries(query: string, actions: Actions): Entry[] {
  const { t } = useI18n();
  const items = useFoundItems(query);
  const { onClose, onOpenItem, onNavigate, onNewItem, onLock } = actions;
  return useMemo<Entry[]>(() => {
    const done = (action: () => void) => () => {
      onClose();
      action();
    };
    const go = (view: View) => done(() => { onNavigate(view); });
    const all: Entry[] = [
      { id: "new", label: t("palette.cmd.newItem"), shortcut: "newItem", run: done(onNewItem) },
      { id: "generator", label: t("palette.cmd.generator"), run: go("generator") },
      { id: "security", label: t("palette.cmd.security"), run: go("security") },
      { id: "settings", label: t("palette.cmd.settings"), shortcut: "settings", run: go("settings") },
      { id: "import", label: t("palette.cmd.import"), run: go("import") },
      { id: "lock", label: t("palette.cmd.lock"), shortcut: "lock", run: done(onLock) },
    ];
    const commands = all.filter((entry) => entry.label.toLowerCase().includes(query.trim().toLowerCase()));
    const found: Entry[] = items.map((item) => ({
      id: `item-${item.itemId}`,
      label: item.title ?? t("vault.untitled"),
      hint: item.subtitle ?? t(`item.kind.${item.kind}`),
      run: done(() => {
        onOpenItem({ vaultId: item.vaultId, itemId: item.itemId });
      }),
    }));
    return [...found, ...commands];
  }, [items, query, t, onClose, onNewItem, onNavigate, onLock, onOpenItem]);
}

function Options({ id, entries, current, onHover }: {
  id: string;
  entries: readonly Entry[];
  current: number;
  onHover: (index: number) => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <ul id={id} role="listbox" aria-label={t("palette.title")} className="palette__list">
      {entries.map((entry, index) => (
        <li
          key={entry.id}
          id={`${id}-${entry.id}`}
          role="option"
          aria-selected={index === current}
          aria-keyshortcuts={
            entry.shortcut === undefined ? undefined : ariaKeyshortcuts(SHORTCUTS[entry.shortcut], platform)
          }
          className="palette__option"
          onMouseEnter={() => {
            onHover(index);
          }}
          onClick={entry.run}
        >
          <span>{entry.label}</span>
          {entry.hint !== undefined && <span className="muted">{entry.hint}</span>}
          {entry.shortcut !== undefined && (
            <kbd className="palette__shortcut">{format(SHORTCUTS[entry.shortcut], platform, t)}</kbd>
          )}
        </li>
      ))}
    </ul>
  );
}

export function CommandPalette(actions: Actions) {
  const { t } = useI18n();
  const listId = useId();
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const entries = useEntries(query, actions);
  const current = Math.min(active, Math.max(entries.length - 1, 0));
  const selected = entries[current];

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setActive(Math.min(current + 1, entries.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      setActive(Math.max(current - 1, 0));
    } else if (event.key === "Enter") {
      event.preventDefault();
      selected?.run();
    } else if (event.key === "Escape") {
      // A search input would clear itself on the first Esc; one press closes the palette.
      event.preventDefault();
      actions.onClose();
    }
  }

  return (
    <Dialog title={t("palette.title")} onClose={actions.onClose}>
      <input
        className="field__input search"
        type="text"
        role="combobox"
        aria-expanded={entries.length > 0}
        aria-controls={entries.length > 0 ? listId : undefined}
        aria-activedescendant={selected === undefined ? undefined : `${listId}-${selected.id}`}
        aria-label={t("palette.search")}
        placeholder={t("palette.search")}
        autoFocus
        value={query}
        onChange={(event) => {
          setQuery(event.target.value);
          setActive(0);
        }}
        onKeyDown={onKeyDown}
      />
      {entries.length === 0 ? (
        <p className="empty muted">{t("palette.empty")}</p>
      ) : (
        <Options id={listId} entries={entries} current={current} onHover={setActive} />
      )}
    </Dialog>
  );
}
