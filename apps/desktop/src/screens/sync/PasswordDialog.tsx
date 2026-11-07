/**
 * A dialog that asks for the master password (and, if wanted, a second-factor code) before
 * an action on the sync account: removing a device, turning two-step verification on or off.
 */
import { useState, type ReactNode, type SyntheticEvent } from "react";
import { Button, type ButtonVariant } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError } from "../../ipc/contract/index.ts";

export function PasswordDialog({
  title,
  body,
  extra,
  withCode,
  submitLabel,
  submitVariant = "secondary",
  onSubmit,
  onClose,
  returnFocus,
}: {
  title: string;
  body: string;
  /** More text under the body (a note on what the action does not do). */
  extra?: ReactNode;
  withCode: boolean;
  submitLabel: string;
  submitVariant?: ButtonVariant;
  /** Runs the action; resolves to the error to show, or `null` when done (the dialog closes). */
  onSubmit: (password: string, code: string | null) => Promise<IpcError | null>;
  onClose: () => void;
  /** Where focus goes on close when the control that opened the dialog is gone. */
  returnFocus?: () => HTMLElement | null;
}) {
  const { t } = useI18n();
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    const failure = await onSubmit(password, withCode ? code : null);
    setBusy(false);
    setPassword("");
    setCode("");
    setError(failure);
    if (failure === null) {
      onClose();
    }
  }

  return (
    <Dialog title={title} onClose={onClose} {...(returnFocus === undefined ? {} : { returnFocus })}>
      <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
        <p className="body muted">{body}</p>
        {extra}
        <PasswordField
          toggleLabel="password"
          label={t("settings.sync.password")}
          autoComplete="current-password"
          data-autofocus=""
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        {withCode && (
          <TextField
            label={t("settings.sync.code")}
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
          <Button type="submit" variant={submitVariant} disabled={busy}>
            {submitLabel}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
