/** The vault and trash views: the list pane beside the detail or editor, or the empty state. */
import { useCallback, useState } from "react";
import { useIpcEvent } from "../../hooks/useIpcEvent.ts";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  InsightFinding,
  ItemRequest,
  VaultId,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { EmptyVault } from "./EmptyVault.tsx";
import { ItemDetail } from "./ItemDetail.tsx";
import { ItemEditor } from "./ItemEditor.tsx";
import { HardwareVaultBar, HardwareVaultPanel, type VaultTransition } from "./HardwareVaultPanel.tsx";
import { ItemList } from "./ItemList.tsx";
import { useEditorFocusReturn } from "./useEditorFocusReturn.ts";
import type { Category } from "./useVaultData.ts";
import type { Editor } from "./VaultLayout.tsx";

export interface ListViewProps {
  vaults: readonly VaultSummary[];
  vaultId: VaultId | null;
  trash: boolean;
  query: string;
  category: Category;
  findings: readonly InsightFinding[];
  selected: ItemRequest | null;
  editor: Editor;
  reloadKey: number;
  /** The empty state replaces the whole split. */
  empty: boolean;
  onSelect: (item: ItemRequest | null) => void;
  onEditor: (editor: Editor) => void;
  onChanged: () => void;
  onImport: () => void;
  onLoaded: (count: number) => void;
}

function Detail(props: ListViewProps) {
  const { t } = useI18n();
  const { editor, selected, trash, vaults, vaultId, reloadKey, onSelect, onEditor, onChanged } = props;
  // The item saved last as new: its detail puts the cursor on "Add file" (documents).
  const [created, setCreated] = useState<string | null>(null);
  useEditorFocusReturn(editor.mode !== "closed");
  if (editor.mode !== "closed") {
    return (
      <ItemEditor
        // New items go to vaults that hold any item type; hardware vaults hold codes only.
        vaults={editor.mode === "new" ? vaults.filter((vault) => vault.protection === "accountKey") : vaults}
        vaultId={vaultId}
        editing={editor.mode === "edit" ? editor.detail : null}
        onSaved={(item) => {
          setCreated(editor.mode === "new" ? item.itemId : null);
          onEditor({ mode: "closed" });
          onSelect(item);
          onChanged();
        }}
        onCancel={() => {
          onEditor({ mode: "closed" });
          if (editor.mode === "edit") {
            // Files added or removed in the editor took effect at once.
            onChanged();
          }
        }}
      />
    );
  }
  if (selected === null) {
    return <p className="detail-empty">{t("vault.selectItem")}</p>;
  }
  return (
    <ItemDetail
      item={selected}
      trash={trash}
      vaults={vaults}
      reloadKey={reloadKey}
      justCreated={created === selected.itemId}
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
  );
}

export function ListView(props: ListViewProps) {
  const { vaultId, trash, vaults } = props;
  // A vault that just opened or closed moves focus and is announced (once).
  const [transition, setTransition] = useState<VaultTransition | null>(null);
  useIpcEvent((event) => {
    if (event.type === "hardwareVaultClosed") {
      setTransition({ kind: "closed", vaultId: event.vaultId });
    }
  });
  // Looking at another vault: a close that happened elsewhere is no longer news.
  const [shownVault, setShownVault] = useState(vaultId);
  if (shownVault !== vaultId) {
    setShownVault(vaultId);
    setTransition(null);
  }
  const seen = useCallback(() => {
    setTransition(null);
  }, []);
  const hardware = vaults.find((vault) => vault.vaultId === vaultId && vault.protection === "hardwareKey");
  if (hardware !== undefined && !trash) {
    const is = (kind: VaultTransition["kind"]) =>
      transition?.kind === kind && transition.vaultId === hardware.vaultId;
    // A closed hardware vault shows no items; an open one shows them under its own bar.
    return hardware.available ? (
      <div className="hw-open">
        <HardwareVaultBar
          vault={hardware}
          focusLock={is("opened")}
          onSeen={seen}
          onClosed={() => {
            setTransition({ kind: "closed", vaultId: hardware.vaultId });
            props.onChanged();
          }}
        />
        <Split {...props} />
      </div>
    ) : (
      <HardwareVaultPanel
        vault={hardware}
        justClosed={is("closed")}
        onSeen={seen}
        onOpened={() => {
          setTransition({ kind: "opened", vaultId: hardware.vaultId });
          props.onChanged();
        }}
      />
    );
  }
  return <Split {...props} />;
}

function Split(props: ListViewProps) {
  const { t } = useI18n();
  const { vaultId, trash, query, category, findings, selected, reloadKey, onSelect, onEditor } = props;
  return (
    <>
      {props.empty && (
        <EmptyVault
          onNew={() => {
            onEditor({ mode: "new" });
          }}
          onImport={props.onImport}
        />
      )}
      <div className="split" hidden={props.empty}>
        <section className="split__list" aria-label={t("vault.items")}>
          <h1 className="visually-hidden">{t(trash ? "nav.trash" : "vault.items")}</h1>
          <ItemList
            vaultId={vaultId}
            trash={trash}
            query={query}
            category={category}
            findings={findings}
            selected={props.editor.mode === "new" ? null : selected}
            reloadKey={reloadKey}
            onSelect={(item) => {
              onEditor({ mode: "closed" });
              onSelect(item);
            }}
            onLoaded={props.onLoaded}
          />
        </section>
        <section className="split__detail">
          <Detail {...props} />
        </section>
      </div>
    </>
  );
}
