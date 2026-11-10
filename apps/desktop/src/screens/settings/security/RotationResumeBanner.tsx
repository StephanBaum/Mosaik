/**
 * "Continue key rotation": after an unlock, an open journal row means a password or key change was
 * cut off (a lock, a crash, a closed window). Nothing is lost. What it asks depends on the job:
 * nothing (it continues), the passwords again (it was never sent: the wizard starts at that
 * step), or the new password and the last group of the new Secret Key (it may have reached the
 * account: typing them again proves the person holds the new kit).
 */
import { useEffect, useRef, useState, type SyntheticEvent } from "react";
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { PasswordField } from "../../../components/PasswordField.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useToast } from "../../../components/ToastProvider.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, RotationJobView } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import type { WizardKind } from "./CredentialsStep.tsx";
import {
  focusLost,
  focusMain,
  needsSecondFactor,
  useAnyWizardOpen,
  useBackendOverview,
  useRotationStatus,
} from "./hooks.ts";
import { RotationWizard } from "./RotationWizard.tsx";
import { AbandonDialog } from "./AbandonDialog.tsx";

/** How often a continuing job is read again (the count is not announced). */
const POLL_MS = 1500;

function wizardKindOf(job: RotationJobView): WizardKind | null {
  switch (job.kind) {
    case "secretKey":
      return "secretKey";
    case "rekey":
      return "rekey";
    case "profile":
      return "profile";
    case "vault":
      return null;
  }
}

// eslint-disable-next-line max-lines-per-function -- a form with optional fields.
function ResumeDialog({
  job,
  onDone,
  onClose,
}: {
  job: RotationJobView;
  onDone: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const needsCode = needsSecondFactor(useBackendOverview());
  const [password, setPassword] = useState("");
  const [group, setGroup] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);
  const askGroup = job.needs === "kitGroup";

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("rotation_resume", {
      password,
      kitGroup: askGroup ? group : null,
      code: needsCode && code !== "" ? code : null,
    });
    setBusy(false);
    setPassword("");
    setGroup("");
    setCode("");
    if (result.ok) {
      onDone();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("rotation.resume.dialog.title")} onClose={onClose}>
      <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
        <p className="body muted">{t("rotation.resume.dialog.body")}</p>
        <PasswordField
          toggleLabel="password"
          label={t("rotation.resume.dialog.password")}
          autoComplete="current-password"
          data-autofocus=""
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        {askGroup && (
          <TextField
            label={t("rotation.resume.dialog.group")}
            className="field__input--code"
            mono
            maxLength={4}
            autoCapitalize="characters"
            autoComplete="off"
            spellCheck={false}
            value={group}
            onChange={(event) => {
              setGroup(event.target.value);
            }}
          />
        )}
        {needsCode && (
          <TextField
            label={t("rotation.credentials.code")}
            inputMode="numeric"
            autoComplete="one-time-code"
            mono
            value={code}
            onChange={(event) => {
              setCode(event.target.value);
            }}
          />
        )}
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button onClick={onClose}>{t("action.cancel")}</Button>
          <HoloButton type="submit" disabled={busy || password === "" || (askGroup && group === "")}>
            {t("rotation.resume.dialog.submit")}
          </HoloButton>
        </div>
      </form>
    </Dialog>
  );
}

function DiscardDialog({ onDone, onClose }: { onDone: () => void; onClose: () => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [error, setError] = useState<IpcError | null>(null);

  async function discard() {
    const result = await ipc.call("rotation_abandon", null);
    if (result.ok) {
      onDone();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("rotation.resume.discard.title")} onClose={onClose}>
      <div className="stack">
        <p className="body muted">{t("rotation.resume.discard.body")}</p>
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("action.cancel")}
          </Button>
          <Button
            variant="dangerSolid"
            onClick={() => {
              void discard();
            }}
          >
            {t("rotation.resume.discard.confirm")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

// eslint-disable-next-line max-lines-per-function, complexity -- the banner and its three ways to continue.
export function RotationResumeBanner() {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const wizardOpen = useAnyWizardOpen();
  const { status, refresh } = useRotationStatus();
  const [dialog, setDialog] = useState<"resume" | "discard" | "abandon" | null>(null);
  const [wizard, setWizard] = useState<WizardKind | null>(null);
  const [running, setRunning] = useState(false);
  // A continue that ended without finishing: a committed job may then be given up.
  const [stalled, setStalled] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const hadJob = useRef(false);
  const job = status?.job ?? null;

  // When the banner goes away (finished, discarded, given up) the control that had focus is gone
  // with it: focus goes to the page, not to <body>.
  useEffect(() => {
    if (job !== null) {
      hadJob.current = true;
    } else if (hadJob.current) {
      hadJob.current = false;
      if (focusLost()) {
        focusMain();
      }
    }
  }, [job]);

  // While a job continues, the count is read again (plain text, not a live region).
  useEffect(() => {
    if (!running) {
      return undefined;
    }
    const timer = setInterval(refresh, POLL_MS);
    return () => {
      clearInterval(timer);
    };
  }, [running, refresh]);

  async function continueNow() {
    setRunning(true);
    setError(null);
    const result = await ipc.call("rotation_resume", { password: null, kitGroup: null, code: null });
    setRunning(false);
    refresh();
    if (result.ok) {
      setStalled(!result.value.finished);
      if (result.value.finished) {
        toast(t("rotation.resume.done"));
      }
    } else {
      setStalled(true);
      setError(result.error);
    }
  }

  function continueJob(current: RotationJobView) {
    if (current.needs === "none") {
      void continueNow();
    } else if (current.needs === "kitGroup") {
      setDialog("resume");
    } else {
      setWizard(wizardKindOf(current));
    }
  }

  // The title is announced from a status region that stays mounted (one that appears with its
  // text is often not read).
  const statusLine = (
    <p className="visually-hidden" role="status">
      {job !== null && wizard === null && !wizardOpen && wizardKindOf(job) !== null ? t("rotation.resume.title") : ""}
    </p>
  );
  if (wizard !== null) {
    return (
      <RotationWizard
        kind={wizard}
        startAt="credentials"
        onClose={() => {
          setWizard(null);
          refresh();
        }}
      />
    );
  }
  if (job === null || wizardOpen || wizardKindOf(job) === null) {
    return statusLine;
  }
  const committed = job.phase === "localCommitted" || job.phase === "attachments" || job.phase === "uploading";
  // A committed job cannot be discarded; once a continue failed it can be given up.
  const canAbandon = committed && stalled;
  const body = job.needs === "password" ? "password" : (job.kind as "secretKey" | "rekey" | "profile");
  return (
    <div className="stack" role="region" aria-label={t("rotation.resume.title")}>
      {statusLine}
      <Callout tone="warning">
        <p className="body" aria-hidden="true">
          <strong>{t("rotation.resume.title")}</strong>
        </p>
        <p className="body">{t(`rotation.resume.body.${body}`)}</p>
        {running && job.total > 0 && (
          <p className="caption">{t("rotation.resume.progress", { done: job.done, total: job.total })}</p>
        )}
        {canAbandon && <p className="caption">{t("rotation.resume.stalled")}</p>}
        <ErrorAlert error={error} />
        <div className="actions">
          <Button
            aria-disabled={running}
            onClick={() => {
              // Not `disabled`: a disabled button drops focus to the page.
              if (!running) {
                continueJob(job);
              }
            }}
          >
            {running ? t("rotation.resume.running") : t("rotation.resume.continue")}
          </Button>
          {(canAbandon || !committed) && (
            <Button
              variant="ghost"
              disabled={running}
              onClick={() => {
                setDialog(canAbandon ? "abandon" : "discard");
              }}
            >
              {canAbandon ? t("rotation.resume.abandon") : t("rotation.resume.discard")}
            </Button>
          )}
        </div>
      </Callout>
      {dialog === "resume" && (
        <ResumeDialog
          job={job}
          onDone={() => {
            setDialog(null);
            refresh();
            toast(t("rotation.resume.done"));
          }}
          onClose={() => {
            setDialog(null);
          }}
        />
      )}
      {dialog === "abandon" && (
        <AbandonDialog
          onDone={() => {
            setDialog(null);
            setStalled(false);
            refresh();
          }}
          onClose={() => {
            setDialog(null);
          }}
        />
      )}
      {dialog === "discard" && (
        <DiscardDialog
          onDone={() => {
            setDialog(null);
            refresh();
          }}
          onClose={() => {
            setDialog(null);
          }}
        />
      )}
    </div>
  );
}
