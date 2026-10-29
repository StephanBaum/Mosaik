/** Unlock with the master password (and the Secret Key on a device that lacks it). */
import { useState, type SyntheticEvent } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { TextField } from "../components/TextField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, SessionState } from "../ipc/contract/index.ts";

type Locked = Extract<SessionState, { state: "locked" }>;

export function UnlockScreen({
  session,
  onUnlocked,
}: {
  session: Locked;
  onUnlocked: (session: SessionState) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const { lockReason, clearLockReason } = useEvents();
  const [password, setPassword] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("unlock", {
      password,
      secretKey: session.secretKeyStored ? null : secretKey,
    });
    setBusy(false);
    setPassword("");
    if (result.ok) {
      setSecretKey("");
      clearLockReason();
      onUnlocked(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <main id="main" className="screen screen--centered">
      <form className="panel" onSubmit={(event) => void submit(event)} noValidate>
        <h1 className="title">{t("lock.unlockTitle")}</h1>
        {session.email !== null && <p className="body muted">{session.email}</p>}
        {lockReason !== null && (
          <p className="body" role="status">
            {t(`lock.reason.${lockReason}`)}
          </p>
        )}
        <TextField
          label={t("lock.masterPassword")}
          type="password"
          autoComplete="current-password"
          // The one field on this screen: focus it like a native unlock prompt.
          autoFocus
          required
          value={password}
          onChange={(event) => {
            setPassword(event.target.value);
          }}
        />
        {!session.secretKeyStored && (
          <TextField
            label={t("lock.secretKey")}
            hint={t("lock.secretKeyHint")}
            mono
            autoComplete="off"
            spellCheck={false}
            required
            value={secretKey}
            onChange={(event) => {
              setSecretKey(event.target.value);
            }}
          />
        )}
        <ErrorAlert error={error} />
        <p className="status" aria-live="polite">
          {busy ? t("lock.working") : ""}
        </p>
        <Button variant="primary" type="submit" disabled={busy}>
          {t("lock.submit")}
        </Button>
      </form>
    </main>
  );
}
