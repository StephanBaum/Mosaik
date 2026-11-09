/**
 * Another device renewed the account's keys (a Secret Key rotation or a re-key). There is no
 * email: this notice is how the user learns of it. The way back is the new Secret Key from the
 * new Emergency Kit and the master password. A Secret Key this device used before is refused
 * with its own message; nothing is adopted without the new key.
 */
import { useState, type SyntheticEvent } from "react";
import { Button } from "../../../components/Button.tsx";
import { Callout } from "../../../components/Callout.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { HoloButton } from "../../../components/HoloButton.tsx";
import { PasswordField } from "../../../components/PasswordField.tsx";
import { TextField } from "../../../components/TextField.tsx";
import { useToast } from "../../../components/ToastProvider.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, SyncOverview } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { useSyncOverview } from "../../sync/hooks.ts";
import { needsSecondFactor } from "./hooks.ts";

/** The form: the master password, the new Secret Key and, when needed, a code. */
// eslint-disable-next-line max-lines-per-function -- a three-field form.
export function RekeyRecoveryForm({
  needsCode,
  onRecovered,
  onCancel,
}: {
  needsCode: boolean;
  onRecovered: (overview: SyncOverview) => void;
  onCancel: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [password, setPassword] = useState("");
  const [secretKey, setSecretKey] = useState("");
  const [code, setCode] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_recover_password", {
      newPassword: password,
      secretKey,
      code: needsCode && code !== "" ? code : null,
    });
    setBusy(false);
    setPassword("");
    setSecretKey("");
    setCode("");
    if (result.ok) {
      onRecovered(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <form className="stack" onSubmit={(event) => void submit(event)} noValidate>
      <p className="body muted">{t("rotation.recovery.body")}</p>
      <PasswordField
        toggleLabel="password"
        label={t("rotation.recovery.password")}
        autoComplete="current-password"
        data-autofocus=""
        value={password}
        onChange={(event) => {
          setPassword(event.target.value);
        }}
      />
      <TextField
        label={t("rotation.recovery.secretKey")}
        mono
        autoCapitalize="characters"
        autoComplete="off"
        spellCheck={false}
        value={secretKey}
        onChange={(event) => {
          setSecretKey(event.target.value);
        }}
      />
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
      <Callout tone="warning">{t("rotation.recovery.warning")}</Callout>
      <ErrorAlert error={error} />
      <div className="dialog__actions">
        <Button onClick={onCancel}>{t("action.cancel")}</Button>
        <HoloButton type="submit" disabled={busy || password === "" || secretKey === ""}>
          {t("rotation.recovery.submit")}
        </HoloButton>
      </div>
    </form>
  );
}

/** The dialog around the form. */
export function RekeyRecoveryDialog({
  needsCode,
  onRecovered,
  onClose,
}: {
  needsCode: boolean;
  onRecovered: (overview: SyncOverview) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={t("rotation.recovery.title")} onClose={onClose}>
      <RekeyRecoveryForm needsCode={needsCode} onRecovered={onRecovered} onCancel={onClose} />
    </Dialog>
  );
}

/** The in-app notice: sync is paused because a verified marker says the keys were renewed elsewhere. */
export function RekeyNotice() {
  const { t } = useI18n();
  const toast = useToast();
  const { overview, setOverview, refresh } = useSyncOverview();
  const [open, setOpen] = useState(false);
  if (overview === null || !overview.rekeyPending) {
    return null;
  }
  return (
    <div role="region" aria-label={t("rotation.recovery.title")}>
      <Callout tone="warning">
        <p className="body">
          <strong>{t("rotation.recovery.title")}</strong>
        </p>
        <p className="body">{t("rotation.recovery.notice")}</p>
        <div className="actions">
          <Button
            onClick={() => {
              setOpen(true);
            }}
          >
            {t("rotation.recovery.open")}
          </Button>
        </div>
      </Callout>
      {open && (
        <RekeyRecoveryDialog
          needsCode={needsSecondFactor(overview)}
          onRecovered={(next) => {
            setOverview(next);
            refresh();
            setOpen(false);
            toast(t("rotation.recovery.done"));
          }}
          onClose={() => {
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}
