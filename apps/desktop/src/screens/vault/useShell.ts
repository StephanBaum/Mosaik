/** The vault shell's state (view, filters, selection, editor) and its actions. */
import { useCallback, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import type { ItemRequest, SessionState, VaultId } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { usePlatform } from "../../platform/platform.ts";
import { useAppShortcuts } from "../../platform/shortcuts.ts";
import type { Category } from "./useVaultData.ts";
import type { Editor, View } from "./VaultLayout.tsx";

export function useShell(onLocked: (session: SessionState) => void) {
  const ipc = useIpc();
  const platform = usePlatform();
  const { setPaletteOpen } = useEvents();
  const [view, setView] = useState<View>("vault");
  const [vaultId, setVaultId] = useState<VaultId | null>(null);
  const [category, setCategory] = useState<Category>("all");
  const [query, setQuery] = useState("");
  const [itemCount, setItemCount] = useState<number | null>(null);
  const [selected, setSelected] = useState<ItemRequest | null>(null);
  const [editor, setEditor] = useState<Editor>({ mode: "closed" });
  const [reloadKey, setReloadKey] = useState(0);

  const changed = useCallback(() => {
    setReloadKey((n) => n + 1);
  }, []);
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
  const search = (next: string) => {
    setQuery(next);
    if (view !== "vault" && view !== "trash") {
      go("vault");
    }
  };

  useAppShortcuts(platform, {
    palette: () => {
      setPaletteOpen(true);
    },
    newItem: startNew,
    settings: () => {
      go("settings");
    },
    lock,
  });

  return {
    view,
    vaultId,
    category,
    query,
    itemCount,
    selected,
    editor,
    reloadKey,
    changed,
    go,
    open,
    startNew,
    lock,
    search,
    setVaultId,
    setCategory,
    setItemCount,
    setSelected,
    setEditor,
  };
}

export type Shell = ReturnType<typeof useShell>;
