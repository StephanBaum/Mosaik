/**
 * A login's password rotation on its detail (M7b): the banner of each open change, the entry to the
 * guided panel, the password history with "Restore" and the reminder interval. Nothing here opens
 * a page: the link to the change page is copied, and the browser extension does the filling.
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  HistoryEntryView,
  IpcError,
  ItemDetail as Detail,
  ItemRequest,
  PolicyDays,
  RotationOutcomeKey,
} from "../../ipc/contract/index.ts";
import { RotationPanel } from "../rotation/RotationPanel.tsx";
import { PasswordHistory } from "./PasswordHistory.tsx";
import { PendingRotationBanner } from "./PendingRotationBanner.tsx";
import { RotationPolicySelect } from "./RotationPolicySelect.tsx";

/** The commands of this section: each runs, says so in a toast where it is not obvious, and reads the item again. */
function useRotationCommands(item: ItemRequest, onDetail: (detail: Detail) => void) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function refresh() {
    const result = await ipc.call("get_item", item);
    if (result.ok) {
      onDetail(result.value);
    } else {
      setError(result.error);
    }
  }

  async function run(command: () => Promise<{ ok: true } | { ok: false; error: IpcError }>, message?: string) {
    setBusy(true);
    setError(null);
    const result = await command();
    setBusy(false);
    if (result.ok) {
      if (message !== undefined) {
        toast(message);
      }
      await refresh();
    } else {
      setError(result.error);
    }
  }

  return {
    busy,
    error,
    refresh,
    closeAttempt: (attemptId: string, outcome: RotationOutcomeKey) => {
      void run(() => ipc.call("rotation_close", { ...item, attemptId, outcome }));
    },
    restore: (entry: HistoryEntryView) =>
      run(
        () => ipc.call("password_history_restore", { ...item, historyId: entry.historyId }),
        t("credentialRotation.history.restored"),
      ),
    setPolicy: (everyDays: PolicyDays | null) => {
      void run(() => ipc.call("rotation_set_policy", { ...item, everyDays }));
    },
  };
}

export function DetailRotation({
  detail,
  item,
  trash,
  onDetail,
}: {
  detail: Detail;
  item: ItemRequest;
  trash: boolean;
  /** The item changed: the detail as Rust has it now. */
  onDetail: (detail: Detail) => void;
}) {
  const { t } = useI18n();
  const [panel, setPanel] = useState(false);
  const commands = useRotationCommands(item, onDetail);
  if (detail.summary.kind !== "login") {
    return null;
  }
  const editable = !trash;
  return (
    <>
      {detail.rotationAttempts.map((attempt) => (
        <PendingRotationBanner
          key={attempt.attemptId}
          item={item}
          attempt={attempt}
          url={detail.changeUrl}
          busy={commands.busy || !editable}
          onClose={commands.closeAttempt}
        />
      ))}
      <ErrorAlert error={commands.error} />
      {editable && detail.changeUrl !== null && (
        <div className="actions">
          <Button
            onClick={() => {
              setPanel(true);
            }}
          >
            {t(detail.rotationAttempts.length > 0 ? "credentialRotation.queue.continue" : "credentialRotation.queue.change")}
          </Button>
        </div>
      )}
      <PasswordHistory
        item={item}
        history={detail.history}
        canRestore={editable}
        busy={commands.busy}
        onRestore={commands.restore}
      />
      {editable && detail.secrets.includes("password") && (
        <RotationPolicySelect value={detail.rotationPolicy} disabled={commands.busy} onChange={commands.setPolicy} />
      )}
      {panel && (
        <RotationPanel
          entry={{
            vaultId: item.vaultId,
            itemId: item.itemId,
            title: detail.summary.title,
            host: detail.summary.host,
            attempts: detail.rotationAttempts,
          }}
          onClose={() => {
            setPanel(false);
            void commands.refresh();
          }}
        />
      )}
    </>
  );
}
