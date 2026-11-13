/**
 * "Rotate vault key…" (M7-R2): a new key for one vault, with the attachments optionally
 * encrypted again under new keys. The master password is asked for after the click, through the
 * re-authentication pattern (`useReauth`). The progress of the attachment step is a polite live
 * region. If Mosaic closes before the step is done, "Continue key rotation" (the banner in the
 * vault) finishes it.
 */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Switch } from "../../components/Switch.tsx";
import { useIpcEvent } from "../../hooks/useIpcEvent.ts";
import { useReauth } from "../../hooks/useReauth.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, RotationDone, VaultSummary } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { useWizardOpenMarker } from "../settings/security/hooks.ts";
import { isolate } from "../sync/hooks.ts";
import { vaultLabel } from "./Sidebar.tsx";

type Step = { kind: "ask" } | { kind: "working" } | { kind: "done"; result: RotationDone; resealed: boolean };

function useRotation(vault: VaultSummary) {
  const ipc = useIpc();
  const { guard, dialog } = useReauth("vaultKey.reauth", "vaultKeyRotation");
  const [reseal, setReseal] = useState(false);
  const [step, setStep] = useState<Step>({ kind: "ask" });
  const [error, setError] = useState<IpcError | null>(null);
  const [progress, setProgress] = useState<{ done: number; total: number } | null>(null);

  useIpcEvent((event) => {
    if (event.type === "rotationProgress" && event.kind === "vault" && event.total > 0) {
      setProgress({ done: event.done, total: event.total });
    }
  });

  async function rotate() {
    setError(null);
    setProgress(null);
    setStep({ kind: "working" });
    const result = await guard(() =>
      ipc.call("vault_rotate_key", { vaultId: vault.vaultId, resealAttachments: reseal, proof: null }),
    );
    if (result.ok) {
      setStep({ kind: "done", result: result.value, resealed: reseal });
    } else {
      // A closed password dialog is the user stopping: back to the question without an error.
      setStep({ kind: "ask" });
      setError(result.error.code === "cancelled" ? null : result.error);
    }
  }
  return { reseal, setReseal, step, error, progress, rotate, dialog };
}

function Outcome({ result, resealed }: { result: RotationDone; resealed: boolean }) {
  const { t } = useI18n();
  const said = !resealed ? "vaultKey.done" : result.finished ? "vaultKey.doneAttachments" : "vaultKey.unfinished";
  return (
    <>
      <p className="body">{t(said)}</p>
      {result.unreachableAttachments > 0 && (
        <p className="caption">{t("vaultKey.unreachable", { count: result.unreachableAttachments })}</p>
      )}
    </>
  );
}

export function RotateVaultKeyDialog({ vault, onDone, onClose }: { vault: VaultSummary; onDone: () => void; onClose: () => void }) {
  const { t } = useI18n();
  const { reseal, setReseal, step, error, progress, rotate, dialog } = useRotation(vault);
  const working = step.kind === "working";
  // This dialog owns the job while it runs: "Continue key rotation" stays out of the way.
  useWizardOpenMarker();
  const name = isolate(vaultLabel(vault, (key) => t(key)));
  return (
    <>
      <Dialog
        title={t("vaultKey.title", { vault: name })}
        onClose={() => {
          // Esc and the backdrop do nothing while the rotation runs: it cannot be taken back.
          if (working) {
            return;
          }
          if (step.kind === "done") {
            onDone();
          }
          onClose();
        }}
      >
        <div className="stack">
          {step.kind === "done" ? (
            <Outcome result={step.result} resealed={step.resealed} />
          ) : (
            <>
              <p className="body muted">{t("vaultKey.body")}</p>
              <Switch
                label={t("vaultKey.attachments")}
                hint={t("vaultKey.attachments.hint")}
                checked={reseal}
                disabled={working}
                onChange={(event) => {
                  setReseal(event.target.checked);
                }}
              />
            </>
          )}
          {/* One live region that stays mounted: progress and the working state are announced from it. */}
          <p className="caption" role="status">
            {working
              ? progress === null
                ? t("vaultKey.working")
                : t("vaultKey.progress", { done: progress.done, total: progress.total })
              : ""}
          </p>
          <ErrorAlert error={error} />
          <div className="dialog__actions">
            {step.kind === "done" ? (
              <HoloButton
                data-autofocus=""
                onClick={() => {
                  onDone();
                  onClose();
                }}
              >
                {t("vaultKey.close")}
              </HoloButton>
            ) : (
              <>
                <Button disabled={working} data-autofocus="" onClick={onClose}>
                  {t("action.cancel")}
                </Button>
                <HoloButton
                  disabled={working}
                  onClick={() => {
                    void rotate();
                  }}
                >
                  {t("vaultKey.submit")}
                </HoloButton>
              </>
            )}
          </div>
        </div>
      </Dialog>
      {dialog}
    </>
  );
}
