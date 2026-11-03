/** Unlock with the master password (and the Secret Key on a device that lacks it). */
import { useRef, useState, type SyntheticEvent } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import { useSettings } from "../app/SettingsProvider.tsx";
import { AuthShell } from "../components/AuthShell.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { PasswordField } from "../components/PasswordField.tsx";
import { TextField } from "../components/TextField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, IpcResult, QuickUnlockKind, SessionState } from "../ipc/contract/index.ts";
import { QuickUnlockPanel } from "./QuickUnlockPanel.tsx";

type Locked = Extract<SessionState, { state: "locked" }>;

/** After these the buttons are gone: read the session again and ask for the password. */
const REREAD_CODES: readonly string[] = ["quickUnlockExpired", "quickUnlockInvalidated", "passwordRequired"];

function useUnlock(
  session: Locked,
  onUnlocked: (session: SessionState) => void,
  onReread: () => void,
) {
  const ipc = useIpc();
  const { clearLockReason } = useEvents();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function run(call: () => Promise<IpcResult<SessionState>>): Promise<boolean> {
    setBusy(true);
    setError(null);
    const result = await call();
    setBusy(false);
    if (result.ok) {
      clearLockReason();
      onUnlocked(result.value);
    } else {
      setError(result.error);
    }
    return result.ok;
  }

  const unlock = (password: string, secretKey: string) =>
    run(() => ipc.call("unlock", { password, secretKey: session.secretKeyStored ? null : secretKey }));
  async function quick(method: QuickUnlockKind, pin: string | null): Promise<IpcError | null> {
    setError(null);
    const result = await ipc.call("unlock_quick", { method, pin });
    if (result.ok) {
      clearLockReason();
      onUnlocked(result.value);
      return null;
    }
    setError(result.error);
    if (REREAD_CODES.includes(result.error.code)) {
      const state = await ipc.call("session_state", null);
      if (state.ok) {
        onUnlocked(state.value);
      }
      onReread();
    }
    return result.error;
  }
  return { busy, error, unlock, quick };
}

function UnlockIntro({ email }: { email: string | null }) {
  const { t } = useI18n();
  const { lockReason } = useEvents();
  return (
    <div className="auth__intro">
      <h1 className="display">{t("lock.unlockTitle")}</h1>
      {email !== null && <p className="lede">{email}</p>}
      {lockReason !== null && (
        <p className="caption" role="status">
          {t(`lock.reason.${lockReason}`)}
        </p>
      )}
    </div>
  );
}

export function UnlockScreen({
  session,
  onUnlocked,
}: {
  session: Locked;
  onUnlocked: (session: SessionState) => void;
}) {
  const { t } = useI18n();
  const { settings } = useSettings();
  const [password, setPassword] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const form = useRef<HTMLFormElement>(null);
  const { busy, error, unlock, quick } = useUnlock(session, onUnlocked, () => {
    form.current?.querySelector<HTMLInputElement>('input[autocomplete="current-password"]')?.focus();
  });
  const minutes = settings?.autoLockMinutes ?? null;

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    const submitted = password;
    setPassword("");
    if (await unlock(submitted, secretKey)) {
      setSecretKey("");
    }
  }

  return (
    <AuthShell center>
      <div className="auth__main">
        <UnlockIntro email={session.email} />
        <form ref={form} className="form-stack" onSubmit={(event) => void submit(event)} noValidate>
          <PasswordField
            size="lg"
            label={t("lock.masterPassword")}
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
              size="lg"
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
          <HoloButton type="submit" size="lg" block ambient disabled={busy}>
            {t("lock.submit")}
          </HoloButton>
        </form>
        <QuickUnlockPanel methods={session.quickUnlock} quick={quick} />
        {minutes !== null && <p className="caption auth__note">{t("lock.autoLockInfo", { minutes })}</p>}
      </div>
      <p className="auth__footnote">
        <strong>{t("lock.forgot.title")}</strong> {t("lock.forgot.body")}
      </p>
    </AuthShell>
  );
}
