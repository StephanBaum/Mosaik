/** The file commands of an item: add, save as and remove, one at a time, with their toasts. */
import { useRef, useState } from "react";
import { useToast } from "../../../components/ToastProvider.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type {
  AttachmentEntry,
  IpcError,
  ItemDetail,
  ItemRequest,
} from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";

export interface Files {
  /** An add is running (its native dialog is open); only one runs at a time. */
  adding: boolean;
  error: IpcError | null;
  add: () => void;
  saveAs: (entry: AttachmentEntry) => void;
  remove: (entry: AttachmentEntry) => Promise<boolean>;
}

export function useFiles(item: ItemRequest, onChanged: (detail: ItemDetail) => void): Files {
  const ipc = useIpc();
  const toast = useToast();
  const { t } = useI18n();
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const addRunning = useRef(false);

  function add() {
    if (addRunning.current) {
      return;
    }
    addRunning.current = true;
    setAdding(true);
    setError(null);
    void ipc.call("attachment_add", item).then((result) => {
      addRunning.current = false;
      setAdding(false);
      if (!result.ok) {
        setError(result.error);
      } else if (result.value.kind === "added") {
        onChanged(result.value.item);
        toast(t("toast.fileAdded"));
      }
    });
  }

  function saveAs(entry: AttachmentEntry) {
    setError(null);
    void ipc
      .call("attachment_save", { ...item, attachmentId: entry.attachmentId })
      .then((result) => {
        if (!result.ok) {
          setError(result.error);
        } else if (result.value.kind === "saved") {
          toast(t("toast.fileSaved"));
        }
      });
  }

  async function remove(entry: AttachmentEntry): Promise<boolean> {
    setError(null);
    const result = await ipc.call("attachment_remove", { ...item, attachmentId: entry.attachmentId });
    if (!result.ok) {
      setError(result.error);
      return false;
    }
    onChanged(result.value);
    toast(t("toast.fileRemoved"));
    return true;
  }

  return { adding, error, add, saveAs, remove };
}
