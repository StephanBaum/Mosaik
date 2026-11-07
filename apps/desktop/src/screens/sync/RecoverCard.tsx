/**
 * "Your master password was changed": the server (or the folder) holds a newer master password
 * than this device, for example after the app was closed in the middle of a change. The new
 * password brings the device up to date; Rust checks that it opens the same account key
 * (CRYPTO_SPEC §9.4). Wrong passwords count towards the five-failure lock.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { PasswordField } from "../../components/PasswordField.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SyncConfigView, SyncOverview } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";

export function RecoverCard({
  config,
  onRecovered,
}: {
  config: SyncConfigView;
  onRecovered: (overview: SyncOverview) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");
  const [needsCode, setNeedsCode] = useState(config.kind === "hosted" && config.totpEnabled);
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_recover_password", {
      newPassword: password,
      code: needsCode && code !== "" ? code : null,
    });
    setBusy(false);
    setPassword("");
    setCode("");
    if (result.ok) {
      onRecovered(result.value);
    } else if (result.error.code === "syncTwoFactorRequired") {
      setNeedsCode(true);
    } else {
      setError(result.error);
    }
  }

  return (
    <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
      <h3 className="subtitle">{t("settings.sync.recover.title")}</h3>
      <p className="caption">{t("settings.sync.recover.body")}</p>
      <PasswordField
        toggleLabel="password"
        label={t("settings.sync.recover.password")}
        autoComplete="new-password"
        value={password}
        onChange={(event) => {
          setPassword(event.target.value);
        }}
      />
      {needsCode && (
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
      <div className="actions">
        <Button type="submit" disabled={busy || password === ""}>
          {t("settings.sync.recover.submit")}
        </Button>
      </div>
    </form>
  );
}
