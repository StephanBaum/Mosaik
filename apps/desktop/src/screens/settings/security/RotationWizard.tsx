/**
 * The guided wizard of a password or key change (M7): change the master password, rotate the
 * Secret Key, upgrade the protection, or re-key the account. One component with a step list per
 * kind:
 *
 * 1. Review: what changes and what happens to other devices, shared vaults, emergency contacts,
 *    quick unlock and security keys. A re-key asks first whether something was compromised.
 * 2. The passwords.
 * 3. The new Emergency Kit (Secret Key rotation and re-key): saved, then its last group typed.
 *    The backend learns nothing before this is done.
 * 4. Confirmation with a verb button (a re-key shows what it touches here).
 * 5. The run, with the phases of the job and a count for the attachments.
 * 6. Done: what was turned off and what to do next.
 *
 * Focus moves to the step's heading on every step change (M6b lesson). Closing before the run
 * discards a job the wizard started; a wizard opened from "Continue key rotation" keeps it.
 */
import { useEffect, useRef, useState, type SyntheticEvent } from "react";
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type {
  IpcError,
  RekeyBackend,
  RekeyReviewView,
  RotationDone,
  RotationJobView,
} from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { StepHeading } from "../../sharing/StepHeading.tsx";
import {
  CredentialsStep,
  emptyCredentials,
  type Credentials,
  type WizardKind,
} from "./CredentialsStep.tsx";
import { needsSecondFactor, useBackendOverview, useWizardOpenMarker } from "./hooks.ts";
import { NewKitStep } from "./NewKitStep.tsx";
import { RunProgress } from "./RunProgress.tsx";
import { ConfirmStep, DoneStep, ReviewStep } from "./WizardSteps.tsx";

export type StepId = "review" | "credentials" | "kit" | "confirm" | "run" | "done";

const STEPS: Record<WizardKind, StepId[]> = {
  password: ["review", "credentials", "done"],
  profile: ["review", "credentials", "done"],
  secretKey: ["review", "credentials", "kit", "confirm", "run", "done"],
  rekey: ["review", "credentials", "kit", "confirm", "run", "done"],
};

/** How often the run asks the journal where it stands. */
const POLL_MS = 1500;

// eslint-disable-next-line max-lines-per-function, complexity -- the wizard's state machine (steps, run, errors) is one unit.
export function RotationWizard({
  kind: startKind,
  startAt,
  onClose,
}: {
  kind: WizardKind;
  /** A wizard opened from "Continue key rotation" starts here and keeps the job when closed. */
  startAt?: StepId;
  /** `focusQuick`: the user chose to set quick unlock up again. */
  onClose: (focusQuick: boolean) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  useWizardOpenMarker();
  const overview = useBackendOverview();
  const needsCode = needsSecondFactor(overview);
  const backend: RekeyBackend = overview?.config?.kind ?? "local";
  const [kind, setKind] = useState(startKind);
  const [step, setStep] = useState<StepId>(startAt ?? "review");
  const [values, setValues] = useState<Credentials>(emptyCredentials);
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);
  const [slow, setSlow] = useState(false);
  const [done, setDone] = useState<RotationDone | null>(null);
  const [review, setReview] = useState<RekeyReviewView | null>(null);
  const [reviewError, setReviewError] = useState<IpcError | null>(null);
  const [job, setJob] = useState<RotationJobView | null>(null);
  const [quickWas, setQuickWas] = useState(false);
  const staged = useRef(false);
  const running = useRef(false);
  const steps = STEPS[kind];
  const position = Math.max(steps.indexOf(step), 0) + 1;
  const resumed = startAt !== undefined;

  // Quick unlock before the change: the password and Secret Key changes delete it without saying so.
  useEffect(() => {
    let current = true;
    void ipc.call("quick_unlock_status", null).then((result) => {
      if (current && result.ok) {
        setQuickWas(result.value.biometric.enrolled);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);

  // The re-key review, read when its confirmation step opens.
  useEffect(() => {
    if (kind !== "rekey" || step !== "confirm") {
      return undefined;
    }
    let current = true;
    void ipc.call("rekey_review", null).then((result) => {
      if (!current) {
        return;
      }
      if (result.ok) {
        setReviewError(null);
        setReview(result.value);
      } else {
        setReviewError(result.error);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, kind, step]);

  // The run: the command works, the journal is read every moment for the phases.
  useEffect(() => {
    if (step !== "run" || running.current) {
      return undefined;
    }
    running.current = true;
    let current = true;
    const poll = setInterval(() => {
      void ipc.call("rotation_status", null).then((result) => {
        if (current && result.ok) {
          setJob(result.value.job);
        }
      });
    }, POLL_MS);
    const request =
      kind === "rekey"
        ? ipc.call("rekey_commit", {
            password: values.password,
            newPassword: values.newPassword,
            code: needsCode ? values.code : null,
          })
        : ipc.call("secret_key_rotate_commit", {
            password: values.password,
            code: needsCode ? values.code : null,
          });
    void request.then((result) => {
      clearInterval(poll);
      running.current = false;
      if (!current) {
        return;
      }
      // Whatever the outcome, the typed secrets are not needed any more.
      setValues(emptyCredentials);
      if (result.ok) {
        setDone(result.value);
        setStep("done");
      } else {
        setError(result.error);
      }
      setBusy(false);
    });
    return () => {
      current = false;
      clearInterval(poll);
    };
    // The run starts once, from the values the user confirmed.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step]);

  async function submitCredentials(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    if (kind === "password") {
      const result = await ipc.call("change_master_password", {
        currentPassword: values.password,
        newPassword: values.newPassword,
        code: needsCode ? values.code : null,
      });
      setBusy(false);
      setValues(emptyCredentials);
      if (result.ok) {
        setDone({ quickUnlockRemoved: false, hardwareKeysRemoved: 0, unreachableAttachments: 0, finished: true });
        setStep("done");
      } else {
        setError(result.error);
      }
      return;
    }
    if (kind === "profile") {
      const result = await ipc.call("profile_upgrade", {
        password: values.password,
        profile: values.profile,
        code: needsCode ? values.code : null,
        force: values.force,
      });
      setBusy(false);
      if (result.ok) {
        setValues(emptyCredentials);
        setDone(result.value);
        setStep("done");
      } else {
        setSlow(result.error.code === "profileTooSlow");
        setError(result.error);
      }
      return;
    }
    const result =
      kind === "rekey"
        ? await ipc.call("rekey_prepare", { password: values.password, newPassword: values.newPassword })
        : await ipc.call("secret_key_rotate_prepare", { password: values.password });
    setBusy(false);
    if (result.ok) {
      staged.current = true;
      setStep("kit");
    } else {
      setError(result.error);
    }
  }

  function close(focusQuick: boolean) {
    setValues(emptyCredentials);
    // A wizard that staged a job and never ran it discards it; a resumed one keeps it.
    if (staged.current && !running.current && done === null && !resumed) {
      void ipc.call("rotation_abandon", null).then(() => {
        onClose(focusQuick);
      });
      return;
    }
    onClose(focusQuick);
  }

  function switchKind(next: WizardKind) {
    setKind(next);
    setStep("review");
    setError(null);
  }

  function run() {
    setBusy(true);
    setError(null);
    setStep("run");
  }

  const locked = step === "run" && busy;
  return (
    <Dialog
      title={t(`rotation.title.${kind}`)}
      size="wide"
      onClose={() => {
        if (!locked && step !== "run") {
          close(false);
        }
      }}
    >
      <div className="stack">
        {steps.length > 2 && (
          <p className="caption">{t("rotation.step", { current: position, total: steps.length })}</p>
        )}
        {step === "review" && (
          <ReviewStep
            key={kind}
            kind={kind}
            hasBackend={backend !== "local"}
            onCancel={() => {
              close(false);
            }}
            onNext={() => {
              setStep("credentials");
            }}
            onSwitch={switchKind}
          />
        )}
        {step === "credentials" && (
          <CredentialsStep
            kind={kind}
            values={values}
            needsCode={needsCode}
            slow={slow}
            busy={busy}
            error={error}
            submit={t(
              kind === "password" || kind === "profile" ? `rotation.confirm.verb.${kind}` : "rotation.next",
            )}
            danger={kind === "password"}
            onChange={(next) => {
              setValues(next);
            }}
            onSubmit={(event) => void submitCredentials(event)}
            onBack={() => {
              setStep("review");
            }}
          />
        )}
        {step === "kit" && (
          <NewKitStep
            onBack={() => {
              setStep("credentials");
            }}
            onVerified={() => {
              setStep("confirm");
            }}
          />
        )}
        {step === "confirm" && (kind === "secretKey" || kind === "rekey") && (
          <ConfirmStep
            kind={kind}
            review={review}
            reviewError={reviewError}
            busy={busy}
            onBack={() => {
              setStep("kit");
            }}
            onRun={run}
          />
        )}
        {step === "run" && (
          <div className="stack">
            <StepHeading>{t("rotation.run.title")}</StepHeading>
            <RunProgress job={job} backend={backend} />
            {error === null ? (
              <p className="caption">{t("rotation.run.keepOpen")}</p>
            ) : (
              <>
                <ErrorAlert error={error} />
                <div className="dialog__actions">
                  <Button
                    onClick={() => {
                      setError(null);
                      setStep("confirm");
                    }}
                  >
                    {t("rotation.back")}
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => {
                      onClose(false);
                    }}
                  >
                    {t("rotation.close")}
                  </Button>
                </div>
              </>
            )}
          </div>
        )}
        {step === "done" && done !== null && (
          <DoneStep kind={kind} done={done} quickWas={quickWas} onClose={close} />
        )}
      </div>
    </Dialog>
  );
}
