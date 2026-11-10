/**
 * "Give up on the rest" for a committed key job that cannot finish (an attachment re-seal keeps
 * failing). The account's new keys are already in use; what is still waiting keeps its old file
 * key. The person confirms first, in words that say what stays unprotected.
 */
import { useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";

export function AbandonDialog({ onDone, onClose }: { onDone: () => void; onClose: () => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function abandon() {
    setBusy(true);
    const result = await ipc.call("rotation_abandon_committed", null);
    setBusy(false);
    if (result.ok) {
      onDone();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("rotation.abandon.title")} onClose={onClose}>
      <div className="stack">
        <p className="body">{t("rotation.abandon.body")}</p>
        <p className="body muted">{t("rotation.abandon.stays")}</p>
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("rotation.abandon.keep")}
          </Button>
          <Button
            variant="dangerSolid"
            disabled={busy}
            onClick={() => {
              void abandon();
            }}
          >
            {t("rotation.abandon.confirm")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
