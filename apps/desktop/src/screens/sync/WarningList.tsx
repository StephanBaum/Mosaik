/**
 * What sync wants the user to know: one callout per warning with what to do. The warnings a
 * person has to look at (a halt, or the server showing different data) offer "Acknowledge…",
 * which opens a confirmation that explains the risk; it clears all of them at once, and has to
 * be done on every device.
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SyncOverview, SyncWarningView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "./hooks.ts";

const ACKNOWLEDGEABLE: ReadonlySet<SyncWarningView["kind"]> = new Set([
  "rollbackDetected",
  "identityDuplicated",
  "serverInconsistent",
  "serverRollback",
  "noCommonView",
]);

function useWarningText() {
  const { t } = useI18n();
  const device = (name: string | null) =>
    name === null ? t("sync.warning.unknownDevice") : isolate(name);
  return (warning: SyncWarningView): string => {
    switch (warning.kind) {
      case "freezeSuspected":
        return t("sync.warning.freezeSuspected", { device: device(warning.deviceName) });
      case "missingFiles":
        return t("sync.warning.missingFiles", { device: device(warning.deviceName) });
      case "noCommonView":
        return t("sync.warning.noCommonView", { device: device(warning.deviceName) });
      case "serverRollback":
        return t("sync.warning.serverRollback", {
          vault:
            warning.vaultName === null
              ? t("sync.warning.unknownVault")
              : isolate(warning.vaultName),
        });
      case "serverInconsistent":
        return t("sync.warning.serverInconsistent");
      case "rollbackDetected":
        return t("sync.warning.rollbackDetected");
      case "identityDuplicated":
        return t("sync.warning.identityDuplicated");
      case "undecryptable":
        return t("sync.warning.undecryptable", { count: warning.count });
      case "hardwareVaultsLocal":
        return t("sync.warning.hardwareVaultsLocal", { count: warning.count });
      case "attachmentsNotRepaired":
        return t("sync.warning.attachmentsNotRepaired", { count: warning.count });
    }
  };
}

function AcknowledgeDialog({
  onDone,
  onClose,
}: {
  onDone: (overview: SyncOverview) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function acknowledge() {
    setBusy(true);
    const result = await ipc.call("sync_acknowledge", null);
    setBusy(false);
    if (result.ok) {
      onDone(result.value);
      onClose();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("sync.acknowledge.title")} onClose={onClose}>
      <div className="stack">
        <p className="body">{t("sync.acknowledge.body")}</p>
        <p className="body">
          <strong>{t("sync.acknowledge.eachDevice")}</strong>
        </p>
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("action.cancel")}
          </Button>
          <Button
            variant="dangerSolid"
            disabled={busy}
            onClick={() => {
              void acknowledge();
            }}
          >
            {t("sync.acknowledge.confirm")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

export function WarningList({
  warnings,
  onChange,
}: {
  warnings: readonly SyncWarningView[];
  onChange: (overview: SyncOverview) => void;
}) {
  const { t } = useI18n();
  const text = useWarningText();
  const [asking, setAsking] = useState(false);
  if (warnings.length === 0) {
    return null;
  }
  return (
    <div className="stack" role="group" aria-label={t("sync.warning.region")}>
      {warnings.map((warning, index) => (
        <Callout key={`${warning.kind}-${String(index)}`} tone={warning.kind === "hardwareVaultsLocal" ? "info" : "warning"}>
          <p className="body">{text(warning)}</p>
          {ACKNOWLEDGEABLE.has(warning.kind) && (
            <div className="actions">
              <Button
                size="sm"
                onClick={() => {
                  setAsking(true);
                }}
              >
                {t("sync.warning.acknowledge")}
              </Button>
            </div>
          )}
        </Callout>
      ))}
      {asking && (
        <AcknowledgeDialog
          onDone={onChange}
          onClose={() => {
            setAsking(false);
          }}
        />
      )}
    </div>
  );
}
