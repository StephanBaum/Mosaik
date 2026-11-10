/**
 * The guided password change of one login (M7b, D13), in three steps:
 *
 * 1. New password: generated for the site; "Use this password" stores it in the item
 *    (`rotation_begin`, synced from here on). The current password stays.
 * 2. Change it on the site: copy the link to the change page and the new password; the browser
 *    extension fills the form (Mosaic never opens a page and never submits).
 * 3. Result: "It worked" makes the new password the item's password, "It didn't work" keeps the
 *    current one (the new value goes to the history as not confirmed), "Not sure yet" leaves the
 *    change open on the item. No value is ever deleted.
 *
 * Focus moves to the step's heading on every step change; the outcome is one polite status.
 */
import { useEffect, useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { HoloButton } from "../../components/HoloButton.tsx";
import { Icon } from "../../components/icons.tsx";
import { SecretField } from "../../components/SecretField.tsx";
import { StrengthMeter } from "../../components/StrengthMeter.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type {
  ChangeUrlView,
  Generated,
  IpcError,
  RotationOutcomeKey,
  VaultId,
  ItemId,
} from "../../ipc/contract/index.ts";
import { StepHeading } from "../sharing/StepHeading.tsx";
import { useCopyAttempt } from "./useCopyAttempt.ts";

/** What the panel needs to know about the login. */
export interface PanelTarget {
  vaultId: VaultId;
  itemId: ItemId;
  title: string | null;
  host: string | null;
  /** Open attempts; the newest is continued. */
  attempts: readonly { attemptId: string }[];
}

type Step = "new" | "site" | "result" | "done";
type Finished = "worked" | "failed" | "notSure";

const STEP_NUMBER: Record<Exclude<Step, "done">, number> = { new: 1, site: 2, result: 3 };

/** The heading of a numbered step: "Step n of 3" is read with it, the eyes get it in the caption above. */
function NumberedHeading({ step, children }: { step: Exclude<Step, "done">; children: string }) {
  const { t } = useI18n();
  return (
    <StepHeading>
      <span className="visually-hidden">
        {t("credentialRotation.panel.step", { current: STEP_NUMBER[step], total: 3 })}.{" "}
      </span>
      {children}
    </StepHeading>
  );
}

function useGenerated(active: boolean, host: string | null, round: number) {
  const ipc = useIpc();
  const [generated, setGenerated] = useState<Generated | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  useEffect(() => {
    if (!active) {
      return undefined;
    }
    let current = true;
    void ipc
      .call(
        "generate_password",
        host === null
          ? { kind: "password", length: 20, lowercase: true, uppercase: true, digits: true, symbols: true, excludeAmbiguous: true }
          : { kind: "forSite", host },
      )
      .then((result) => {
        if (current) {
          setGenerated(result.ok ? result.value : null);
          setError(result.ok ? null : result.error);
        }
      });
    return () => {
      current = false;
    };
  }, [ipc, active, host, round]);
  return { generated, error };
}

/** The change-page URL of an item the panel continues (`get_item`). */
function useContinuedUrl(target: PanelTarget): ChangeUrlView | null {
  const ipc = useIpc();
  const [url, setUrl] = useState<ChangeUrlView | null>(null);
  const continuing = target.attempts.length > 0;
  useEffect(() => {
    if (!continuing) {
      return undefined;
    }
    let current = true;
    void ipc.call("get_item", { vaultId: target.vaultId, itemId: target.itemId }).then((result) => {
      if (current && result.ok) {
        setUrl(result.value.changeUrl);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, continuing, target.vaultId, target.itemId]);
  return url;
}

function NewStep({
  generated,
  busy,
  onRegenerate,
  onUse,
  onCancel,
}: {
  generated: Generated | null;
  busy: boolean;
  onRegenerate: () => void;
  onUse: () => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <NumberedHeading step="new">{t("credentialRotation.steps.new.title")}</NumberedHeading>
      <p className="caption">{t("credentialRotation.steps.new.intro")}</p>
      {generated !== null && (
        <>
          <SecretField
            label={t("credentialRotation.steps.new.label")}
            reveal={() => Promise.resolve({ ok: true, value: { value: generated.value, hideAfterSeconds: 60 } })}
          />
          <StrengthMeter strength={generated.strength} bits={generated.entropyBits} />
        </>
      )}
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("credentialRotation.panel.cancel")}</Button>
        <Button onClick={onRegenerate} disabled={busy}>
          <Icon name="refresh" />
          {t("action.regenerate")}
        </Button>
        <HoloButton onClick={onUse} disabled={generated === null || busy}>
          {busy ? t("credentialRotation.steps.new.saving") : t("credentialRotation.steps.new.use")}
        </HoloButton>
      </div>
    </>
  );
}

function SiteStep({
  url,
  onCopyLink,
  onCopyPassword,
  onNext,
}: {
  url: ChangeUrlView | null;
  onCopyLink: () => void;
  onCopyPassword: () => void;
  onNext: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <NumberedHeading step="site">{t("credentialRotation.steps.site.title")}</NumberedHeading>
      <p>{t("credentialRotation.steps.site.body")}</p>
      {url === null ? (
        <p className="caption">{t("credentialRotation.steps.site.noLink")}</p>
      ) : (
        <>
          <TextField
            label={t("credentialRotation.steps.site.pageLabel")}
            value={url.url}
            readOnly
            mono
            hint={t("credentialRotation.steps.site.linkHint")}
          />
          <div className="actions">
            <Button onClick={onCopyLink}>
              <Icon name="copy" />
              {t("credentialRotation.steps.site.copyLink")}
            </Button>
          </div>
        </>
      )}
      <div className="actions">
        <Button onClick={onCopyPassword}>
          <Icon name="copy" />
          {t("credentialRotation.steps.site.copyPassword")}
        </Button>
      </div>
      <div className="dialog__actions">
        <HoloButton onClick={onNext}>{t("credentialRotation.steps.site.next")}</HoloButton>
      </div>
    </>
  );
}

function ResultStep({
  busy,
  onChoose,
  onBack,
}: {
  busy: boolean;
  onChoose: (outcome: RotationOutcomeKey | "notSure") => void;
  onBack: () => void;
}) {
  const { t } = useI18n();
  return (
    <>
      <NumberedHeading step="result">{t("credentialRotation.steps.result.title")}</NumberedHeading>
      <p className="caption">{t("credentialRotation.steps.result.body")}</p>
      <div className="dialog__actions">
        <Button onClick={onBack} disabled={busy}>
          {t("action.back")}
        </Button>
        <Button
          onClick={() => {
            onChoose("notSure");
          }}
          disabled={busy}
        >
          {t("credentialRotation.result.notSure")}
        </Button>
        <Button
          onClick={() => {
            onChoose("failed");
          }}
          disabled={busy}
        >
          {t("credentialRotation.result.failed")}
        </Button>
        <HoloButton
          onClick={() => {
            onChoose("succeeded");
          }}
          disabled={busy}
        >
          {t("credentialRotation.result.worked")}
        </HoloButton>
      </div>
    </>
  );
}

function DoneStep({ finished, onClose }: { finished: Finished; onClose: () => void }) {
  const { t } = useI18n();
  return (
    <>
      <StepHeading>
        {finished === "worked"
          ? t("action.done")
          : t(finished === "failed" ? "credentialRotation.steps.done.failed" : "credentialRotation.steps.done.notSure")}
      </StepHeading>
      <p role="status">{t(`credentialRotation.result.${finished}Done`)}</p>
      <div className="dialog__actions">
        <HoloButton onClick={onClose} data-autofocus="">
          {t("action.done")}
        </HoloButton>
      </div>
    </>
  );
}

/** The panel's state: the step, the attempt that was started and the two commands. */
function usePanelFlow(entry: PanelTarget) {
  const ipc = useIpc();
  const continued = entry.attempts[entry.attempts.length - 1]?.attemptId ?? null;
  const [step, setStep] = useState<Step>(continued === null ? "new" : "site");
  const [attemptId, setAttemptId] = useState<string | null>(continued);
  const [begunUrl, setBegunUrl] = useState<ChangeUrlView | null>(null);
  const [finished, setFinished] = useState<Finished>("notSure");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const item = { vaultId: entry.vaultId, itemId: entry.itemId };

  async function begin(password: string) {
    setBusy(true);
    setError(null);
    const result = await ipc.call("rotation_begin", { ...item, newPassword: password, rulesSource: null });
    setBusy(false);
    if (result.ok) {
      setAttemptId(result.value.attemptId);
      setBegunUrl(result.value.changeUrl);
      setStep("site");
    } else {
      setError(result.error);
    }
  }

  async function choose(outcome: RotationOutcomeKey | "notSure") {
    if (outcome === "notSure") {
      setFinished("notSure");
      setStep("done");
      return;
    }
    if (attemptId === null) {
      return;
    }
    setBusy(true);
    setError(null);
    const result = await ipc.call("rotation_close", { ...item, attemptId, outcome });
    setBusy(false);
    if (result.ok) {
      setFinished(outcome === "succeeded" ? "worked" : "failed");
      setStep("done");
    } else {
      setError(result.error);
    }
  }

  return { step, setStep, attemptId, begunUrl, finished, busy, error, item, begin, choose };
}

export function RotationPanel({ entry, onClose }: { entry: PanelTarget; onClose: () => void }) {
  const { t } = useI18n();
  const copies = useCopyAttempt();
  const flow = usePanelFlow(entry);
  const { step, setStep, attemptId, busy, item } = flow;
  const [round, setRound] = useState(0);
  const continuedUrl = useContinuedUrl(entry);
  const { generated, error: generateError } = useGenerated(step === "new", entry.host, round);
  const title = entry.title ?? t("vault.untitled");
  const url = flow.begunUrl ?? continuedUrl;
  const toast = useToast();
  // Closing after the new password was stored leaves the change open: say so (the X and Esc).
  const closeWithNotice = () => {
    if (attemptId !== null && step !== "done") {
      toast(t("credentialRotation.panel.stays", { title }));
    }
    onClose();
  };

  return (
    <Dialog title={t("credentialRotation.panel.title", { title })} onClose={closeWithNotice} size="wide" closeButton>
      {step !== "done" && (
        <p className="caption" aria-hidden="true">
          {t("credentialRotation.panel.step", { current: STEP_NUMBER[step], total: 3 })}
        </p>
      )}
      <ErrorAlert error={flow.error ?? generateError} />
      {step === "new" && (
        <NewStep
          generated={generated}
          busy={busy}
          onRegenerate={() => {
            setRound((n) => n + 1);
          }}
          onUse={() => {
            if (generated !== null) {
              void flow.begin(generated.value);
            }
          }}
          onCancel={onClose}
        />
      )}
      {step === "site" && (
        <SiteStep
          url={url}
          onCopyLink={() => {
            if (url !== null) {
              void copies.link(url.url);
            }
          }}
          onCopyPassword={() => {
            if (attemptId !== null) {
              void copies.password({ ...item, attemptId });
            }
          }}
          onNext={() => {
            setStep("result");
          }}
        />
      )}
      {step === "result" && (
        <ResultStep
          busy={busy}
          onChoose={(outcome) => {
            void flow.choose(outcome);
          }}
          onBack={() => {
            setStep("site");
          }}
        />
      )}
      {step === "done" && <DoneStep finished={flow.finished} onClose={onClose} />}
    </Dialog>
  );
}
