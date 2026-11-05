/** The command palette (D11): Ctrl+K / ⌘K or the global shortcut. Items and every command. */
import { useEffect, useId, useMemo, useState } from "react";
import { Avatar } from "../components/Avatar.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { Icon, type IconName } from "../components/icons.tsx";
import { Kbd } from "../components/Kbd.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { ItemRequest, ItemSummary } from "../ipc/contract/index.ts";
import { SEARCH_DEBOUNCE_MS } from "./vault/ItemList.tsx";
import { usePlatform } from "../platform/platform.ts";
import { ariaKeyshortcuts, SHORTCUTS, type ShortcutName } from "../platform/shortcuts.ts";
import type { View } from "./vault/VaultLayout.tsx";
import { UserText } from "../components/UserText.tsx";
import { titleOrUntitled } from "./vault/title.ts";

interface Entry {
  id: string;
  group: "items" | "commands";
  label: string;
  hint?: string;
  icon?: IconName;
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
    const command = (spec: Omit<Entry, "group">): Entry => ({ ...spec, group: "commands" });
    const all: Entry[] = [
      command({ id: "new", label: t("palette.cmd.newItem"), icon: "plus", run: done(onNewItem), shortcut: "newItem" }),
      command({ id: "generator", label: t("palette.cmd.generator"), icon: "refresh", run: go("generator") }),
      command({ id: "security", label: t("palette.cmd.security"), icon: "shield", run: go("security") }),
      command({ id: "settings", label: t("palette.cmd.settings"), icon: "sliders", run: go("settings"), shortcut: "settings" }),
      command({ id: "import", label: t("palette.cmd.import"), icon: "download", run: go("import") }),
      command({ id: "lock", label: t("palette.cmd.lock"), icon: "lock", run: done(onLock), shortcut: "lock" }),
    ];
    const commands = all.filter((entry) => entry.label.toLowerCase().includes(query.trim().toLowerCase()));
    const found: Entry[] = items.map((item) => ({
      id: `item-${item.itemId}`,
      group: "items",
      label: titleOrUntitled(item.title, t("vault.untitled")),
      hint:
        item.subtitle === null
          ? t(`item.kind.${item.kind}`)
          : `${t(`item.kind.${item.kind}`)} · ${item.subtitle}`,
      run: done(() => {
        onOpenItem({ vaultId: item.vaultId, itemId: item.itemId });
      }),
    }));
    return [...found, ...commands];
  }, [items, query, t, onClose, onNewItem, onNavigate, onLock, onOpenItem]);
}

/** The label with the first occurrence of the query marked (plain string search). */
function Highlight({ text, query }: { text: string; query: string }) {
  const needle = query.trim();
  const at = needle === "" ? -1 : text.toLowerCase().indexOf(needle.toLowerCase());
  if (at < 0) {
    return <>{text}</>;
  }
  return (
    <>
      {text.slice(0, at)}
      <mark className="palette__match">{text.slice(at, at + needle.length)}</mark>
      {text.slice(at + needle.length)}
    </>
  );
}

function Option({
  id,
  entry,
  query,
  selected,
  onHover,
}: {
  id: string;
  entry: Entry;
  query: string;
  selected: boolean;
  onHover: () => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <li
      id={id}
      role="option"
      aria-selected={selected}
      aria-keyshortcuts={
        entry.shortcut === undefined ? undefined : ariaKeyshortcuts(SHORTCUTS[entry.shortcut], platform)
      }
      className="palette__option"
      onMouseEnter={onHover}
      onClick={entry.run}
    >
      {entry.group === "items" ? (
        <Avatar title={entry.label} size="sm" selected={selected} />
      ) : (
        <span className="palette__icon">
          <Icon name={entry.icon ?? "plus"} />
        </span>
      )}
      <span className="palette__text">
        <span className="palette__label">
          {entry.group === "items" ? (
            <UserText>
              <Highlight text={entry.label} query={query} />
            </UserText>
          ) : (
            entry.label
          )}
        </span>
        {entry.hint !== undefined && (
          <span className="caption">
            <UserText>{entry.hint}</UserText>
          </span>
        )}
      </span>
      {entry.shortcut !== undefined && <Kbd shortcut={entry.shortcut} hidden />}
      {entry.group === "items" && selected && (
        <Kbd hidden className="kbd--on-selected">
          {t("keys.enter")}
        </Kbd>
      )}
    </li>
  );
}

function Options({
  id,
  entries,
  query,
  current,
  onHover,
}: {
  id: string;
  entries: readonly Entry[];
  query: string;
  current: number;
  onHover: (index: number) => void;
}) {
  const { t } = useI18n();
  const groups = [
    { name: "items", label: t("palette.items") },
    { name: "commands", label: t("palette.commands") },
  ] as const;
  return (
    <ul id={id} role="listbox" aria-label={t("palette.title")} className="palette__list">
      {groups.map(({ name, label }) => {
        const inGroup = entries.map((entry, index) => ({ entry, index })).filter((e) => e.entry.group === name);
        return inGroup.length === 0 ? null : (
          <li key={name} role="presentation">
            <div id={`${id}-${name}`} className="palette__group">
              {label}
            </div>
            <ul role="group" aria-labelledby={`${id}-${name}`} className="palette__options">
              {inGroup.map(({ entry, index }) => (
                <Option
                  key={entry.id}
                  id={`${id}-${entry.id}`}
                  entry={entry}
                  query={query}
                  selected={index === current}
                  onHover={() => {
                    onHover(index);
                  }}
                />
              ))}
            </ul>
          </li>
        );
      })}
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
    <Dialog title={t("palette.title")} onClose={actions.onClose} size="palette" hideTitle>
      <div className="palette__search">
        <Icon name="search" size="lg" className="muted" />
        <input
          className="palette__input"
          type="text"
          role="combobox"
          aria-expanded={entries.length > 0}
          aria-controls={entries.length > 0 ? listId : undefined}
          aria-activedescendant={selected === undefined ? undefined : `${listId}-${selected.id}`}
          aria-label={t("palette.search")}
          placeholder={t("palette.search")}
          data-autofocus=""
          value={query}
          onChange={(event) => {
            setQuery(event.target.value);
            setActive(0);
          }}
          onKeyDown={onKeyDown}
        />
        <Kbd hidden>{t("keys.esc")}</Kbd>
      </div>
      {entries.length === 0 ? (
        <p className="caption palette__empty">{t("palette.empty")}</p>
      ) : (
        <Options id={listId} entries={entries} query={query} current={current} onHover={setActive} />
      )}
      <div className="palette__footer caption" aria-hidden="true">
        <span>
          <Kbd>↑↓</Kbd> {t("palette.hint.move")}
        </span>
        <span>
          <Kbd>{t("keys.enter")}</Kbd> {t("palette.hint.open")}
        </span>
        <span className="palette__footer-end">
          <Kbd>{t("keys.esc")}</Kbd> {t("palette.hint.close")}
        </span>
      </div>
    </Dialog>
  );
}
