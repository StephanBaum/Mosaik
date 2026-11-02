/** The vault and trash views: the list pane beside the detail or editor, or the empty state. */
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
import { ItemList } from "./ItemList.tsx";
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
  if (editor.mode !== "closed") {
    return (
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
            selected={selected}
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
