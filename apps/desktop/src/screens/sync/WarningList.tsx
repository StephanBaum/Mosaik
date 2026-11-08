/**
 * What sync wants the user to know: one callout per warning with what to do. The warnings a
 * person has to look at (a halt, or the server showing different data) share one button below
 * the list, "I've checked…". It clears all of them at once and has to be done on every device,
 * so the risk is asked in a native dialog that Rust opens (the webview cannot skip it).
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SyncOverview, SyncWarningView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate, useFocusSection } from "./hooks.ts";

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
  const vault = (name: string | null) =>
    name === null ? t("sync.warning.unknownVault") : isolate(name);
  return (warning: SyncWarningView): string => {
    switch (warning.kind) {
      case "freezeSuspected":
        return t("sync.warning.freezeSuspected", { device: device(warning.deviceName) });
      case "missingFiles":
        return t("sync.warning.missingFiles", { device: device(warning.deviceName) });
      case "noCommonView":
        return t("sync.warning.noCommonView", { device: device(warning.deviceName) });
      case "serverRollback":
        return t("sync.warning.serverRollback", { vault: vault(warning.vaultName) });
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
      case "rotationBlocked":
        return t("sharing.finding.rotationBlocked", { vault: vault(warning.vaultName) });
    }
  };
}

export function WarningList({
  warnings,
  onChange,
}: {
  warnings: readonly SyncWarningView[];
  onChange: (overview: SyncOverview) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const text = useWarningText();
  const focusSection = useFocusSection();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  if (warnings.length === 0) {
    return null;
  }

  async function acknowledge() {
    setBusy(true);
    setError(null);
    // Rust shows the confirmation; "no" returns the overview unchanged.
    const result = await ipc.call("sync_acknowledge", {});
    setBusy(false);
    if (result.ok) {
      onChange(result.value);
      if (!result.value.warnings.some((w) => ACKNOWLEDGEABLE.has(w.kind))) {
        // The button that had focus is gone.
        focusSection();
      }
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="stack" role="group" aria-label={t("sync.warning.region")}>
      {warnings.map((warning, index) => (
        <Callout key={`${warning.kind}-${String(index)}`} tone={warning.kind === "hardwareVaultsLocal" ? "info" : "warning"}>
          <p className="body">{text(warning)}</p>
        </Callout>
      ))}
      {warnings.some((w) => ACKNOWLEDGEABLE.has(w.kind)) && (
        <div className="actions">
          <Button
            disabled={busy}
            onClick={() => {
              void acknowledge();
            }}
          >
            {t("sync.warning.acknowledge")}
          </Button>
        </div>
      )}
      <ErrorAlert error={error} />
    </div>
  );
}
