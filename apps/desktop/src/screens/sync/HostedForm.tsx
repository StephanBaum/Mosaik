/**
 * Creating or signing in to the sync account on a Mosaic server, including the second-factor
 * step. `signInOnly` is the "Sign in again" form of a device the server signed out.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type {
  HostedMode,
  IpcError,
  IpcResult,
  RegistrationMode,
  SyncOverview,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { useDefaultDeviceName } from "./hooks.ts";

interface Credentials {
  password: string;
  invite: string;
  deviceName: string;
}

function TwoFactorForm({
  busy,
  error,
  onVerify,
  onCancel,
}: {
  busy: boolean;
  error: IpcError | null;
  onVerify: (code: string) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const [code, setCode] = useState("");
  function verify(event: SyntheticEvent) {
    event.preventDefault();
    onVerify(code);
  }
  return (
    <form className="stack" onSubmit={verify} noValidate>
      <h4 className="body">{t("settings.sync.twoFactor.title")}</h4>
      <p className="field__hint">{t("settings.sync.twoFactor.body")}</p>
      <TextField
        label={t("settings.sync.code")}
        inputMode="numeric"
        autoComplete="one-time-code"
        mono
        autoFocus
        value={code}
        onChange={(event) => {
          setCode(event.target.value);
        }}
      />
      <ErrorAlert error={error} />
      <div className="actions">
        <Button type="submit" disabled={busy}>
          {t("settings.sync.twoFactor.submit")}
        </Button>
        <Button variant="ghost" onClick={onCancel}>
          {t("action.cancel")}
        </Button>
      </div>
    </form>
  );
}

function CredentialsForm({
  registration,
  signInOnly,
  deviceName,
  busy,
  error,
  onSubmit,
}: {
  registration: RegistrationMode;
  signInOnly: boolean;
  deviceName: string;
  busy: boolean;
  error: IpcError | null;
  onSubmit: (mode: HostedMode, credentials: Credentials) => void;
}) {
  const { t } = useI18n();
  const [fields, setFields] = useState<Credentials>({ password: "", invite: "", deviceName });
  const canCreate = !signInOnly && registration !== "closed";
  const set = (key: keyof Credentials) => (event: { target: { value: string } }) => {
    setFields((current) => ({ ...current, [key]: event.target.value }));
  };
  return (
    <form
      className="stack"
      onSubmit={(event) => {
        event.preventDefault();
      }}
      noValidate
    >
      {canCreate && registration === "invite" && (
        <TextField
          label={t("settings.sync.server.invite")}
          autoComplete="off"
          value={fields.invite}
          onChange={set("invite")}
        />
      )}
      <PasswordField
        toggleLabel="password"
        label={t("settings.sync.password")}
        autoComplete="current-password"
        value={fields.password}
        onChange={set("password")}
      />
      <TextField
        label={t("settings.sync.deviceName")}
        autoComplete="off"
        value={fields.deviceName}
        onChange={set("deviceName")}
      />
      <ErrorAlert error={error} />
      <p className="status caption" role="status">
        {busy ? t("settings.sync.server.working") : ""}
      </p>
      <div className="actions">
        {canCreate && (
          <Button
            disabled={busy}
            onClick={() => {
              onSubmit("register", fields);
            }}
          >
            {t("settings.sync.server.create")}
          </Button>
        )}
        <Button
          disabled={busy}
          onClick={() => {
            onSubmit("signIn", fields);
          }}
        >
          {t(signInOnly ? "settings.sync.server.signInAgain" : "settings.sync.server.signIn")}
        </Button>
      </div>
    </form>
  );
}

export function HostedForm({
  serverUrl,
  registration,
  signInOnly = false,
  deviceName,
  onConnected,
}: {
  serverUrl: string;
  registration: RegistrationMode;
  signInOnly?: boolean;
  /** The name to start with; this device's default when omitted. */
  deviceName?: string;
  onConnected: (overview: SyncOverview) => void;
}) {
  const ipc = useIpc();
  const defaultName = useDefaultDeviceName();
  const [needsCode, setNeedsCode] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);

  async function finish(call: () => Promise<IpcResult<SyncOverview>>) {
    setBusy(true);
    setError(null);
    const result = await call();
    setBusy(false);
    if (result.ok) {
      onConnected(result.value);
    } else if (result.error.code === "syncTwoFactorRequired") {
      setNeedsCode(true);
    } else {
      setError(result.error);
    }
  }

  if (needsCode) {
    return (
      <TwoFactorForm
        busy={busy}
        error={error}
        onVerify={(code) => {
          void finish(() => ipc.call("sync_hosted_two_factor", { code }));
        }}
        onCancel={() => {
          setNeedsCode(false);
          setError(null);
        }}
      />
    );
  }
  return (
    <CredentialsForm
      registration={registration}
      signInOnly={signInOnly}
      deviceName={deviceName ?? defaultName}
      busy={busy}
      error={error}
      onSubmit={(mode, fields) => {
        void finish(() =>
          ipc.call("sync_hosted_connect", {
            serverUrl,
            mode,
            password: fields.password,
            inviteCode: mode === "register" && registration === "invite" ? fields.invite : null,
            deviceName: fields.deviceName.trim(),
          }),
        );
      }}
    />
  );
}
