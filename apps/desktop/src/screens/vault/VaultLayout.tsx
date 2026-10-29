/**
 * The unlocked app (D08 to D10, D19): a header with the main sections, a vault sidebar,
 * the item list and detail, and the other sections (security, generator, settings,
 * import) in the main area. The palette and the SSH approval dialog live here too.
 */
import { useCallback, useEffect, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { Button } from "../../components/Button.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { usePlatform } from "../../platform/platform.ts";
import { ariaKeyshortcuts, SHORTCUTS, useAppShortcuts } from "../../platform/shortcuts.ts";
import type {
  ItemDetail as Detail,
  ItemRequest,
  SessionState,
  VaultId,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { CommandPalette } from "../CommandPalette.tsx";
import { GeneratorPanel } from "../GeneratorPanel.tsx";
import { ImportFlow } from "../import/ImportFlow.tsx";
import { SecurityDashboard } from "../SecurityDashboard.tsx";
import { SettingsScreen } from "../SettingsScreen.tsx";
import { SshApprovalDialog } from "../SshApprovalDialog.tsx";
import { ItemDetail } from "./ItemDetail.tsx";
import { ItemEditor } from "./ItemEditor.tsx";
import { ItemList } from "./ItemList.tsx";

export type View = "vault" | "trash" | "security" | "generator" | "settings" | "import";

const SECTIONS = [
  { view: "vault", label: "nav.vault" },
  { view: "trash", label: "nav.trash" },
  { view: "security", label: "nav.security" },
  { view: "generator", label: "nav.generator" },
  { view: "settings", label: "nav.settings" },
] as const;

type Editor = { mode: "closed" } | { mode: "new" } | { mode: "edit"; detail: Detail };

function Header({
  view,
  onGo,
  onNew,
  onLock,
}: {
  view: View;
  onGo: (view: View) => void;
  onNew: () => void;
  onLock: () => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <header className="vault__header">
      <span className="vault__brand">{t("app.name")}</span>
      <nav aria-label={t("nav.main")} className="tabs">
        {SECTIONS.map((section) => (
          <button
            key={section.view}
            type="button"
            className="tabs__tab"
            aria-current={view === section.view ? "page" : undefined}
            onClick={() => {
              onGo(section.view);
            }}
          >
            {t(section.label)}
          </button>
        ))}
      </nav>
      <div className="actions">
        <Button onClick={onNew} aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.newItem, platform)}>
          {t("vault.new")}
        </Button>
        <Button
          variant="ghost"
          onClick={onLock}
          aria-keyshortcuts={ariaKeyshortcuts(SHORTCUTS.lock, platform)}
        >
          {t("action.lock")}
        </Button>
      </div>
    </header>
  );
}

function Sidebar({
  vaults,
  vaultId,
  onPick,
}: {
  vaults: readonly VaultSummary[];
  vaultId: VaultId | null;
  onPick: (vaultId: VaultId | null) => void;
}) {
  const { t } = useI18n();
  const entries = [
    { id: null, name: t("vault.allItems") },
    ...vaults.map((vault) => ({ id: vault.vaultId, name: vault.name ?? t("vault.unnamed") })),
  ];
  return (
    <nav className="vault__sidebar" aria-label={t("vault.vaults")}>
      <ul className="nav-list">
        {entries.map((entry) => (
          <li key={entry.id ?? "all"}>
            <button
              type="button"
              className="nav-list__item"
              aria-current={vaultId === entry.id ? "page" : undefined}
              onClick={() => {
                onPick(entry.id);
              }}
            >
              {entry.name}
            </button>
          </li>
        ))}
      </ul>
    </nav>
  );
}

function EmptyVault({ onNew, onImport }: { onNew: () => void; onImport: () => void }) {
  const { t } = useI18n();
  return (
    <div className="empty-vault">
      <p className="body muted">{t("vault.empty")}</p>
      <div className="actions">
        <HoloButton onClick={onNew}>{t("vault.new")}</HoloButton>
        <Button onClick={onImport}>{t("palette.cmd.import")}</Button>
      </div>
    </div>
  );
}

interface ListViewProps {
  vaults: readonly VaultSummary[];
  vaultId: VaultId | null;
  trash: boolean;
  selected: ItemRequest | null;
  editor: Editor;
  reloadKey: number;
  onSelect: (item: ItemRequest | null) => void;
  onEditor: (editor: Editor) => void;
  onChanged: () => void;
  onImport: () => void;
}

function ListView(props: ListViewProps) {
  const { t } = useI18n();
  const { vaults, vaultId, trash, selected, editor, reloadKey, onSelect, onEditor, onChanged } = props;
  const [query, setQuery] = useState("");
  const [count, setCount] = useState<number | null>(null);
  return (
    <div className="split">
      <section className="split__list" aria-label={t("vault.items")}>
        <h1 className="visually-hidden">{t(trash ? "nav.trash" : "vault.items")}</h1>
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
        <ItemList
          vaultId={vaultId}
          trash={trash}
          query={query}
          selected={selected}
          reloadKey={reloadKey}
          onSelect={(item) => {
            onEditor({ mode: "closed" });
            onSelect(item);
          }}
          onLoaded={setCount}
        />
        {count === 0 && query.trim() === "" && !trash && editor.mode === "closed" && (
          <EmptyVault
            onNew={() => {
              onEditor({ mode: "new" });
            }}
            onImport={props.onImport}
          />
        )}
      </section>
      <section className="split__detail">
        {editor.mode !== "closed" ? (
          <ItemEditor
            vaults={vaults}
            vaultId={vaultId}
            editing={editor.mode === "edit" ? editor.detail : null}
            onSaved={(item) => {
              onEditor({ mode: "closed" });
              onSelect(item);
              onChanged();
            }}
            onCancel={() => {
              onEditor({ mode: "closed" });
            }}
          />
        ) : selected === null ? (
          <p className="empty muted">{t("vault.selectItem")}</p>
        ) : (
          <ItemDetail
            item={selected}
            trash={trash}
            vaults={vaults}
            reloadKey={reloadKey}
            onChanged={(gone) => {
              if (gone) {
                onSelect(null);
              }
              onChanged();
            }}
            onEdit={(detail) => {
              onEditor({ mode: "edit", detail });
            }}
          />
        )}
      </section>
    </div>
  );
}

function useVaults(reloadKey: number): VaultSummary[] {
  const ipc = useIpc();
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  useEffect(() => {
    void ipc.call("list_vaults", null).then((result) => {
      if (result.ok) {
        setVaults(result.value);
      }
    });
  }, [ipc, reloadKey]);
  return vaults;
}

export function VaultLayout({ onLocked }: { onLocked: (session: SessionState) => void }) {
  const ipc = useIpc();
  const events = useEvents();
  const [view, setView] = useState<View>("vault");
  const [vaultId, setVaultId] = useState<VaultId | null>(null);
  const [selected, setSelected] = useState<ItemRequest | null>(null);
  const [editor, setEditor] = useState<Editor>({ mode: "closed" });
  const [reloadKey, setReloadKey] = useState(0);
  const { setPaletteOpen } = events;

  const vaults = useVaults(reloadKey);
  const platform = usePlatform();

  const changed = useCallback(() => { setReloadKey((n) => n + 1); }, []);
  const go = (next: View) => {
    setView(next);
    setEditor({ mode: "closed" });
    setSelected(null);
  };
  const open = (item: ItemRequest) => {
    setView("vault");
    setEditor({ mode: "closed" });
    setSelected(item);
  };
  const startNew = () => {
    setView("vault");
    setSelected(null);
    setEditor({ mode: "new" });
  };
  const lock = () => {
    void ipc.call("lock", null).then((result) => {
      if (result.ok) {
        onLocked(result.value);
      }
    });
  };

  useAppShortcuts(platform, {
    palette: () => { setPaletteOpen(true); },
    newItem: startNew,
    settings: () => { go("settings"); },
    lock,
  });

  return (
    <div className="vault">
      <Header view={view} onGo={go} onNew={startNew} onLock={lock} />
      <Sidebar vaults={vaults} vaultId={vaultId} onPick={(id) => { setVaultId(id); setView("vault"); }} />
      <main id="main" className="vault__main">
        {(view === "vault" || view === "trash") && (
          <ListView
            vaults={vaults}
            vaultId={vaultId}
            trash={view === "trash"}
            selected={selected}
            editor={editor}
            reloadKey={reloadKey}
            onSelect={setSelected}
            onEditor={setEditor}
            onChanged={changed}
            onImport={() => { go("import"); }}
          />
        )}
        {view === "security" && <SecurityDashboard onOpenItem={open} />}
        {view === "generator" && <GeneratorPanel />}
        {view === "settings" && <SettingsScreen />}
        {view === "import" && (
          <ImportFlow embedded onClose={() => { changed(); go("vault"); }} />
        )}
      </main>
      {events.paletteOpen && (
        <CommandPalette
          onClose={() => { setPaletteOpen(false); }}
          onOpenItem={open}
          onNavigate={go}
          onNewItem={startNew}
          onLock={lock}
        />
      )}
      <SshApprovalDialog />
    </div>
  );
}
