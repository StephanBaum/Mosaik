/**
 * Security-key dialogs shared by Settings (sign-in keys) and hardware-protected vaults:
 * the key list with remove, and the add flow (password, PIN, touch). Only rendered where
 * the backend reports security keys as supported.
 */
import { useState, type ReactNode, type SyntheticEvent } from "react";
import { Button } from "../components/Button.tsx";
import { Callout } from "../components/Callout.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { PasswordField } from "../components/PasswordField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { HardwareKeySummary, IpcError } from "../ipc/contract/index.ts";

/** The first 8 hex characters of a key's method id, to tell registered keys apart. */
export function keyTag(key: HardwareKeySummary): string {
  return key.methodId.slice(0, 8);
}

export function AddKeyDialog({
  withPassword,
  onAdd,
  onClose,
}: {
  /** Ask for the master password (Settings) or not (keys of an open vault). */
  withPassword: boolean;
  /** Runs the registration; resolves to the error to show, or `null` when done. */
  onAdd: (password: string | null, pin: string | null) => Promise<IpcError | null>;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [password, setPassword] = useState("");
  const [pin, setPin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const failure = await onAdd(password === "" ? null : password, pin === "" ? null : pin);
    setBusy(false);
    setPassword("");
    if (failure === null) {
      setPin("");
      onClose();
      return;
    }
    // A missing master password fails before the key is touched: keep the PIN for the retry.
    if (failure.code !== "reauthRequired") {
      setPin("");
    }
    setError(failure);
  }

  function cancel() {
    if (busy) {
      void ipc.call("hardware_key_cancel", null);
    }
    onClose();
  }

  return (
    <Dialog title={t("settings.quick.addKey")} onClose={cancel}>
      <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
        <p className="body muted">{t("settings.quick.addKeyBody")}</p>
        {withPassword && (
          <PasswordField
            label={t("reauth.password")}
            autoComplete="current-password"
            autoFocus
            value={password}
            onChange={(event) => {
              setPassword(event.target.value);
            }}
          />
        )}
        <PasswordField
          label={t("lock.quick.pin")}
          autoComplete="off"
          autoFocus={!withPassword}
          value={pin}
          onChange={(event) => {
            setPin(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <p className="status" role="status">
          {busy ? t("lock.quick.touchKey") : ""}
        </p>
        <div className="dialog__actions">
          <Button onClick={cancel}>{t("action.cancel")}</Button>
          <HoloButton type="submit" disabled={busy}>
            {t("settings.quick.addKey")}
          </HoloButton>
        </div>
      </form>
    </Dialog>
  );
}

export function KeysDialog({
  title,
  keys,
  note,
  withPassword,
  onAdd,
  onRemove,
  onClose,
}: {
  title: string;
  keys: readonly HardwareKeySummary[];
  /** The warning shown under the list. */
  note: ReactNode;
  withPassword: boolean;
  onAdd: (password: string | null, pin: string | null) => Promise<IpcError | null>;
  onRemove: (methodId: string) => Promise<IpcError | null>;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [adding, setAdding] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  return (
    <Dialog title={title} onClose={onClose} closeButton>
      <div className="stack">
        {keys.length === 0 ? (
          <p className="body muted">{t("settings.quick.keysCount", { count: 0 })}</p>
        ) : (
          <ul className="rows">
            {keys.map((key) => (
              <li key={key.methodId} className="row">
                <p className="row__main">
                  {t("settings.quick.keyRow", { tag: keyTag(key), addedAt: new Date(key.addedAt) })}
                </p>
                <Button
                  variant="ghost"
                  size="sm"
                  aria-label={t("settings.quick.removeKeyNamed", { tag: keyTag(key) })}
                  onClick={() => {
                    void onRemove(key.methodId).then(setError);
                  }}
                >
                  {t("settings.quick.removeKey")}
                </Button>
              </li>
            ))}
          </ul>
        )}
        <ErrorAlert error={error} />
        <Callout tone="warning">{note}</Callout>
        <div className="dialog__actions">
          <Button
            onClick={() => {
              setAdding(true);
            }}
          >
            {t("settings.quick.addKey")}
          </Button>
        </div>
      </div>
      {adding && (
        <AddKeyDialog
          withPassword={withPassword}
          onAdd={onAdd}
          onClose={() => {
            setAdding(false);
          }}
        />
      )}
    </Dialog>
  );
}
