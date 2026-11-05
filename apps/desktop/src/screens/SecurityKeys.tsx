/**
 * Security-key dialogs shared by Settings (sign-in keys) and hardware-protected vaults:
 * the key list with remove, and the add flow (password, PIN, touch). Only rendered where
 * the backend reports security keys as supported.
 */
import { useEffect, useRef, useState, type ReactNode, type RefObject, type SyntheticEvent } from "react";
import { Button } from "../components/Button.tsx";
import { Callout } from "../components/Callout.tsx";
import { Dialog } from "../components/Dialog.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { HoloButton } from "../components/HoloButton.tsx";
import { PasswordField } from "../components/PasswordField.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { HardwareKeySummary, IpcError } from "../ipc/contract/index.ts";

/** Errors that mean "the master password": shown at its field. */
const PASSWORD_CODES: readonly string[] = ["reauthRequired", "wrongCredentials"];

/** The first 8 hex characters of a key's method id, to tell registered keys apart. */
export function keyTag(key: HardwareKeySummary): string {
  return key.methodId.slice(0, 8);
}

type OnAdd = (password: string | null, pin: string | null) => Promise<IpcError | null>;

/** The add flow's state: password, PIN, waiting for the key, and the error to show. */
function useAddKey(withPassword: boolean, onAdd: OnAdd, onClose: () => void, form: RefObject<HTMLFormElement | null>) {
  const [password, setPassword] = useState("");
  const [pin, setPin] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  // The submit button is disabled while the key is waited for: focus Cancel instead.
  useEffect(() => {
    if (busy) {
      form.current?.querySelector<HTMLElement>("[data-cancel]")?.focus();
    }
  }, [busy, form]);

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
    } else if (failure.code !== "cancelled") {
      // (A "cancelled" failure is the user backing out of the master-password step: say nothing.)
      // A missing master password fails before the key is touched: keep the PIN for the retry.
      setPin(failure.code === "reauthRequired" ? pin : "");
      setError(failure);
      if (withPassword && PASSWORD_CODES.includes(failure.code)) {
        requestAnimationFrame(() => {
          form.current?.querySelector<HTMLElement>('input[autocomplete="current-password"]')?.focus();
        });
      }
    }
  }
  return { password, setPassword, pin, setPin, busy, error, submit };
}

export function AddKeyDialog({
  withPassword,
  onAdd,
  onClose,
}: {
  /** Ask for the master password (Settings) or not (keys of an open vault). */
  withPassword: boolean;
  /** Runs the registration; resolves to the error to show, or `null` when done. */
  onAdd: OnAdd;
  onClose: () => void;
}) {
  const { t, errorMessage } = useI18n();
  const ipc = useIpc();
  const form = useRef<HTMLFormElement>(null);
  const flow = useAddKey(withPassword, onAdd, onClose, form);
  const { error } = flow;
  // A password problem belongs to the password field, not to the alert under the form.
  const passwordError = withPassword && error !== null && PASSWORD_CODES.includes(error.code) ? error : null;

  function cancel() {
    if (flow.busy) {
      void ipc.call("hardware_key_cancel", null);
    }
    onClose();
  }

  return (
    <Dialog title={t("settings.quick.addKey")} onClose={cancel}>
      <form ref={form} className="stack" onSubmit={(event) => void flow.submit(event)} noValidate>
        <p className="body muted">{t("settings.quick.addKeyBody")}</p>
        {withPassword && (
          <PasswordField
            label={t("reauth.password")}
            autoComplete="current-password"
            data-autofocus=""
            error={passwordError === null ? undefined : errorMessage(passwordError)}
            value={flow.password}
            onChange={(event) => {
              flow.setPassword(event.target.value);
            }}
          />
        )}
        <PasswordField
          label={t("lock.quick.pin")}
          autoComplete="off"
          data-autofocus={withPassword ? undefined : ""}
          value={flow.pin}
          onChange={(event) => {
            flow.setPin(event.target.value);
          }}
        />
        <ErrorAlert error={passwordError === null ? error : null} />
        <p className="status" role="status">
          {flow.busy ? t("lock.quick.touchKey") : ""}
        </p>
        <div className="dialog__actions">
          <Button data-cancel="" onClick={cancel}>
            {t("action.cancel")}
          </Button>
          <HoloButton type="submit" disabled={flow.busy}>
            {t("settings.quick.addKey")}
          </HoloButton>
        </div>
      </form>
    </Dialog>
  );
}

/** Removing a key: ask first, then continue at the row that took its place, or at "Add". */
function useRemoveKey(
  list: RefObject<HTMLUListElement | null>,
  onRemove: (methodId: string) => Promise<IpcError | null>,
) {
  const [confirming, setConfirming] = useState<string | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const rows = () => list.current?.querySelectorAll<HTMLElement>("[data-remove]") ?? [];
  async function remove(methodId: string, index: number) {
    const failure = await onRemove(methodId);
    setError(failure);
    setConfirming(null);
    requestAnimationFrame(() => {
      const left = rows();
      (failure === null ? left[Math.min(index, left.length - 1)] : left[index])?.focus();
      if (left.length === 0) {
        document.querySelector<HTMLElement>("[data-add-key]")?.focus();
      }
    });
  }
  return { confirming, setConfirming, error, remove, rows };
}

/** One registered key; "Remove" asks "Remove security key {tag}?" first. */
function KeyRow({
  keyItem,
  confirming,
  onAsk,
  onCancel,
  onConfirm,
}: {
  keyItem: HardwareKeySummary;
  confirming: boolean;
  onAsk: () => void;
  onCancel: () => void;
  onConfirm: () => void;
}) {
  const { t } = useI18n();
  const tag = keyTag(keyItem);
  if (confirming) {
    return (
      <li className="row">
        <p className="row__main" role="alert">
          {t("settings.quick.removeKeyConfirm", { tag })}
        </p>
        <Button variant="danger" size="sm" autoFocus onClick={onConfirm}>
          {t("settings.quick.removeKey")}
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          {t("action.cancel")}
        </Button>
      </li>
    );
  }
  return (
    <li className="row">
      <p className="row__main">
        {t("settings.quick.keyRow", { tag, addedAt: new Date(keyItem.addedAt) })}
      </p>
      <Button
        data-remove=""
        variant="ghost"
        size="sm"
        aria-label={t("settings.quick.removeKeyNamed", { tag })}
        onClick={onAsk}
      >
        {t("settings.quick.removeKey")}
      </Button>
    </li>
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
  onAdd: OnAdd;
  onRemove: (methodId: string) => Promise<IpcError | null>;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const list = useRef<HTMLUListElement>(null);
  const [adding, setAdding] = useState(false);
  const { confirming, setConfirming, error, remove, rows } = useRemoveKey(list, onRemove);

  return (
    <Dialog title={title} onClose={onClose} closeButton>
      <div className="stack">
        {keys.length === 0 ? (
          <p className="body muted">{t("settings.quick.keysCount", { count: 0 })}</p>
        ) : (
          <ul ref={list} className="rows">
            {keys.map((key, index) => (
              <KeyRow
                key={key.methodId}
                keyItem={key}
                confirming={confirming === key.methodId}
                onAsk={() => {
                  setConfirming(key.methodId);
                }}
                onCancel={() => {
                  setConfirming(null);
                  requestAnimationFrame(() => {
                    rows()[index]?.focus();
                  });
                }}
                onConfirm={() => {
                  void remove(key.methodId, index);
                }}
              />
            ))}
          </ul>
        )}
        <ErrorAlert error={error} />
        <Callout tone="warning">{note}</Callout>
        <div className="dialog__actions">
          <Button
            data-add-key=""
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
