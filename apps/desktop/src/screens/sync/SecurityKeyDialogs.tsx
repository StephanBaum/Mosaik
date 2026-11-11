/**
 * The dialogs of the security key list (M9): add a key (name, master password, a code when the
 * account has one, then the browser step) and remove one (ask, then, if the server wants a
 * recent second factor, the master password and a confirm in the browser).
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, WebauthnKeyView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { SecurityKeyStep, type StepView } from "./SecurityKeyStep.tsx";

interface AddFields {
  label: string;
  password: string;
  code: string;
}

function AddForm({
  withCode,
  warn,
  busy,
  error,
  onSubmit,
  onClose,
}: {
  /** Ask for an authenticator-app code. */
  withCode: boolean;
  /** Without a code backup this would be the only second step. */
  warn: boolean;
  busy: boolean;
  error: IpcError | null;
  onSubmit: (fields: AddFields) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [fields, setFields] = useState<AddFields>({ label: "", password: "", code: "" });
  const set = (key: keyof AddFields) => (event: { target: { value: string } }) => {
    setFields((current) => ({ ...current, [key]: event.target.value }));
  };
  function submit(event: SyntheticEvent) {
    event.preventDefault();
    onSubmit(fields);
    setFields((current) => ({ ...current, password: "", code: "" }));
  }
  return (
    <form className="stack" onSubmit={submit} noValidate>
      <p className="body muted">{t("settings.sync.key.addBody")}</p>
      {warn && (
        <Callout tone="warning">
          <p className="body">{t("settings.sync.key.noBackup")}</p>
        </Callout>
      )}
      <TextField
        label={t("settings.sync.key.label")}
        hint={t("settings.sync.key.labelHint")}
        autoComplete="off"
        maxLength={64}
        data-autofocus=""
        value={fields.label}
        onChange={set("label")}
      />
      <PasswordField
        toggleLabel="password"
        label={t("settings.sync.password")}
        autoComplete="current-password"
        value={fields.password}
        onChange={set("password")}
      />
      {withCode && (
        <TextField
          label={t("settings.sync.code")}
          inputMode="numeric"
          autoComplete="one-time-code"
          mono
          value={fields.code}
          onChange={set("code")}
        />
      )}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onClose}>{t("action.cancel")}</Button>
        <Button type="submit" disabled={busy || fields.label.trim() === ""}>
          {t("settings.sync.key.addConfirm")}
        </Button>
      </div>
    </form>
  );
}

export function AddDialog({
  totp,
  firstKey,
  onDone,
  onClose,
  returnFocus,
}: {
  /** The account has an authenticator-app code (asked for the fresh login if needed). */
  totp: boolean;
  /** No key yet: without a code as a backup, this would be the only second step. */
  firstKey: boolean;
  onDone: () => void;
  onClose: () => void;
  returnFocus: () => HTMLElement | null;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [needCode, setNeedCode] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);
  const [view, setView] = useState<StepView | null>(null);

  async function begin(fields: AddFields) {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_webauthn_add_begin", {
      password: fields.password,
      code: fields.code === "" ? null : fields.code,
      label: fields.label.trim(),
    });
    setBusy(false);
    if (result.ok) {
      setView(result.value);
      return;
    }
    setNeedCode(result.error.code === "syncTwoFactorRequired");
    setError(result.error);
  }

  return (
    <Dialog title={t("settings.sync.key.addTitle")} onClose={onClose} returnFocus={returnFocus}>
      {view === null ? (
        <AddForm
          withCode={totp || needCode}
          warn={firstKey && !totp}
          busy={busy}
          error={error}
          onSubmit={(fields) => {
            void begin(fields);
          }}
          onClose={onClose}
        />
      ) : (
        <SecurityKeyStep
          view={view}
          poll={() => ipc.call("sync_webauthn_add_status", null)}
          onView={setView}
          onDone={() => {
            onDone();
            onClose();
          }}
          onCancel={onClose}
          onRetry={() => {
            setView(null);
          }}
        />
      )}
    </Dialog>
  );
}

function PasswordForm({
  busy,
  error,
  onSubmit,
  onClose,
}: {
  busy: boolean;
  error: IpcError | null;
  onSubmit: (password: string) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [password, setPassword] = useState("");
  return (
    <form
      className="stack"
      onSubmit={(event) => {
        event.preventDefault();
        onSubmit(password);
        setPassword("");
      }}
      noValidate
    >
      <p className="body muted">{t("settings.sync.key.confirmBody")}</p>
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
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onClose}>{t("action.cancel")}</Button>
        <Button type="submit" disabled={busy}>
          {t("settings.sync.key.addConfirm")}
        </Button>
      </div>
    </form>
  );
}

function AskForm({
  lastFactor,
  withCode,
  busy,
  error,
  onRemove,
  onClose,
}: {
  lastFactor: boolean;
  /** The account has an authenticator-app code: it can stand in for the confirm in the browser. */
  withCode: boolean;
  busy: boolean;
  error: IpcError | null;
  onRemove: (code: string) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const [code, setCode] = useState("");
  return (
    <form
      className="stack"
      onSubmit={(event) => {
        event.preventDefault();
        onRemove(code);
        setCode("");
      }}
      noValidate
    >
      {!withCode && <p className="body muted">{t("settings.sync.key.removeBody")}</p>}
      {lastFactor && (
        <Callout tone="warning">
          <p className="body">{t("settings.sync.key.removeLast")}</p>
        </Callout>
      )}
      {withCode && (
        <TextField
          label={t("settings.sync.key.removeCode")}
          hint={t("settings.sync.key.removeCodeHint")}
          inputMode="numeric"
          autoComplete="one-time-code"
          mono
          data-autofocus=""
          value={code}
          onChange={(event) => {
            setCode(event.target.value);
          }}
        />
      )}
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onClose} data-autofocus={withCode ? undefined : ""}>
          {t("action.cancel")}
        </Button>
        <Button type="submit" variant="dangerSolid" disabled={busy}>
          {t("settings.sync.key.removeConfirm")}
        </Button>
      </div>
    </form>
  );
}

type Phase = "ask" | "password" | "key";

/** The state and the two calls of removing a key. */
function useRemoveFlow(target: WebauthnKeyView, onDone: () => void, onClose: () => void) {
  const ipc = useIpc();
  const [phase, setPhase] = useState<Phase>("ask");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);
  const [view, setView] = useState<StepView | null>(null);

  /** Removes the key; a missing recent confirm sends the user through the confirm step. */
  async function remove(code: string) {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_webauthn_remove", {
      credentialRef: target.credentialRef,
      code: code.trim() === "" ? null : code.trim(),
    });
    setBusy(false);
    if (result.ok) {
      onDone();
      onClose();
    } else if (
      result.error.code === "syncTwoFactorRequired" ||
      result.error.code === "syncFreshLoginExpired"
    ) {
      setPhase("password");
    } else {
      setError(result.error);
    }
  }

  async function confirm(password: string) {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_webauthn_confirm_begin", { password });
    setBusy(false);
    if (result.ok) {
      setView(result.value);
      setPhase("key");
    } else {
      setError(result.error);
    }
  }
  return { phase, setPhase, view, setView, busy, error, remove, confirm };
}

export function RemoveDialog({
  target,
  lastFactor,
  totp,
  onDone,
  onClose,
  returnFocus,
}: {
  target: WebauthnKeyView;
  /** Removing it leaves no second step: two-step verification turns off. */
  lastFactor: boolean;
  /** The account has an authenticator-app code (asked for in the dialog). */
  totp: boolean;
  onDone: () => void;
  onClose: () => void;
  returnFocus: () => HTMLElement | null;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const flow = useRemoveFlow(target, onDone, onClose);
  const { phase, view, setView, setPhase, busy, error } = flow;
  return (
    <Dialog
      title={t("settings.sync.key.removeTitle", { name: target.label })}
      onClose={onClose}
      returnFocus={returnFocus}
    >
      {phase === "key" && view !== null ? (
        <SecurityKeyStep
          view={view}
          poll={() => ipc.call("sync_webauthn_add_status", null)}
          onView={setView}
          onDone={() => {
            setPhase("ask");
            void flow.remove("");
          }}
          onCancel={onClose}
          onRetry={() => {
            setView(null);
            setPhase("password");
          }}
        />
      ) : phase === "password" ? (
        <PasswordForm
          busy={busy}
          error={error}
          onSubmit={(password) => {
            void flow.confirm(password);
          }}
          onClose={onClose}
        />
      ) : (
        <AskForm
          lastFactor={lastFactor}
          withCode={totp}
          busy={busy}
          error={error}
          onRemove={(code) => {
            void flow.remove(code);
          }}
          onClose={onClose}
        />
      )}
    </Dialog>
  );
}
