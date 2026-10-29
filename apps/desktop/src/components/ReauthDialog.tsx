/** Asks for the master password again before a change that weakens protection. */
import { useState, type SyntheticEvent } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError } from "../ipc/contract/index.ts";
import { Button } from "./Button.tsx";
import { Dialog } from "./Dialog.tsx";
import { ErrorAlert } from "./ErrorAlert.tsx";
import { TextField } from "./TextField.tsx";

export function ReauthDialog({
  kit = false,
  onSubmit,
  onClose,
}: {
  /** The password protects the Emergency Kit, not a setting. */
  kit?: boolean;
  /** Runs the protected action; resolves to the error to show, or `null` when done. */
  onSubmit: (password: string) => Promise<IpcError | null>;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [password, setPassword] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    const failure = await onSubmit(password);
    setBusy(false);
    setPassword("");
    setError(failure);
    if (failure === null) {
      onClose();
    }
  }

  return (
    <Dialog title={t("reauth.title")} onClose={onClose}>
      <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
        <p className="body muted">{t(kit ? "reauth.bodyKit" : "reauth.body")}</p>
        <TextField
          label={t("reauth.password")}
          type="password"
          autoComplete="current-password"
          autoFocus
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <div className="actions">
          <Button onClick={onClose}>{t("action.cancel")}</Button>
          <Button variant="primary" type="submit" disabled={busy}>
            {t("reauth.submit")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
