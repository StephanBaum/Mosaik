/**
 * An account file in the sync folder that this device cannot verify (M7): a stray file, or another
 * device's re-key made at the same moment as this one (`conflict`). It is dismissible and never
 * halts anything; a conflict is explained and its only resolution is to re-key again from here.
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SyncOverview, SyncWarningView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { RotationWizard } from "../settings/security/RotationWizard.tsx";
import { useFocusSection } from "./hooks.ts";

type FileWarning = Extract<SyncWarningView, { kind: "unverifiedAccountFile" }>;

export function AccountFileFinding({
  warning,
  onChange,
}: {
  warning: FileWarning;
  onChange: (overview: SyncOverview) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const focusSection = useFocusSection();
  const [busy, setBusy] = useState(false);
  const [rekeying, setRekeying] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function dismiss() {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_dismiss_account_file", { name: warning.name, sha256: warning.sha256 });
    setBusy(false);
    if (result.ok) {
      onChange(result.value);
      // The button that had focus is gone.
      focusSection();
    } else {
      setError(result.error);
    }
  }

  return (
    <Callout tone="warning">
      {warning.conflict && (
        <p className="body">
          <strong>{t("rotation.file.conflict.title")}</strong>
        </p>
      )}
      <p className="body">{t(warning.conflict ? "rotation.file.conflict" : "rotation.file.unknown")}</p>
      <ErrorAlert error={error} />
      <div className="actions">
        {warning.conflict && (
          <Button
            disabled={busy}
            onClick={() => {
              setRekeying(true);
            }}
          >
            {t("rotation.file.conflict.rekey")}
          </Button>
        )}
        <Button
          variant="ghost"
          disabled={busy}
          onClick={() => {
            void dismiss();
          }}
        >
          {warning.conflict ? t("rotation.file.conflict.keep") : t("rotation.file.dismiss")}
        </Button>
      </div>
      {rekeying && (
        <RotationWizard
          kind="rekey"
          onClose={() => {
            setRekeying(false);
          }}
        />
      )}
    </Callout>
  );
}
