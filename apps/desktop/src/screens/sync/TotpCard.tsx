/**
 * Two-step verification of the sync account (Mosaic server only). It protects the server
 * account, not the vaults. Turning it on takes the master password, then the setup key (as
 * text: there is no QR component) and a code from the authenticator app; turning it off takes
 * the password and a code.
 */
import { useRef, useState, type SyntheticEvent } from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { TextField } from "../../components/TextField.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, TotpSetupInfo } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { PasswordDialog } from "./PasswordDialog.tsx";

/** What a hidden secret looks like to the snapshot painter (the same text as `SecretField`). */
const MASK = "\u2022".repeat(16);

function SetupDialog({
  setup,
  onDone,
  onClose,
  returnFocus,
}: {
  setup: TotpSetupInfo;
  onDone: () => void;
  onClose: () => void;
  returnFocus: () => HTMLElement | null;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [code, setCode] = useState("");
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function confirm(event: SyntheticEvent) {
    event.preventDefault();
    setBusy(true);
    const result = await ipc.call("sync_totp_confirm", { code });
    setBusy(false);
    setCode("");
    if (result.ok) {
      onDone();
      onClose();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("settings.sync.totp.beginTitle")} onClose={onClose} returnFocus={returnFocus}>
      <form className="stack" onSubmit={(event) => void confirm(event)} noValidate>
        <p className="body muted">{t("settings.sync.totp.setupBody")}</p>
        <TextField
          label={t("settings.sync.totp.secret")}
          mono
          readOnly
          autoComplete="off"
          data-secret=""
          data-mask={MASK}
          value={setup.secret}
          onFocus={(event) => {
            event.target.select();
          }}
        />
        <TextField
          label={t("settings.sync.code")}
          inputMode="numeric"
          autoComplete="one-time-code"
          mono
          data-autofocus=""
          value={code}
          onChange={(event) => {
            setCode(event.target.value);
          }}
        />
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button onClick={onClose}>{t("action.cancel")}</Button>
          <Button type="submit" disabled={busy}>
            {t("settings.sync.totp.confirm")}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}

export function TotpCard({ enabled, onChanged }: { enabled: boolean; onChanged: () => void }) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [asking, setAsking] = useState<"begin" | "disable" | null>(null);
  const [setup, setSetup] = useState<TotpSetupInfo | null>(null);
  const toggle = useRef<HTMLButtonElement>(null);

  return (
    <div className="stack">
      <h3 className="subtitle">{t("settings.sync.totp.title")}</h3>
      <p className="caption">{t("settings.sync.totp.body")}</p>
      <div className="actions">
        <p className="body" role="status">
          {t(enabled ? "settings.sync.totp.on" : "settings.sync.totp.off")}
        </p>
        <Button
          ref={toggle}
          onClick={() => {
            setAsking(enabled ? "disable" : "begin");
          }}
        >
          {t(enabled ? "settings.sync.totp.disable" : "settings.sync.totp.enable")}
        </Button>
      </div>
      {asking === "begin" && (
        <PasswordDialog
          title={t("settings.sync.totp.beginTitle")}
          body={t("settings.sync.totp.beginBody")}
          withCode={false}
          submitLabel={t("settings.sync.totp.begin")}
          onSubmit={async (password) => {
            const result = await ipc.call("sync_totp_begin", { password });
            if (result.ok) {
              setSetup(result.value);
            }
            return result.ok ? null : result.error;
          }}
          onClose={() => {
            setAsking(null);
          }}
        />
      )}
      {asking === "disable" && (
        <PasswordDialog
          title={t("settings.sync.totp.disableTitle")}
          body={t("settings.sync.totp.disableBody")}
          withCode
          submitLabel={t("settings.sync.totp.disableConfirm")}
          submitVariant="dangerSolid"
          onSubmit={async (password, code) => {
            const result = await ipc.call("sync_totp_disable", { password, code: code ?? "" });
            if (result.ok) {
              toast(t("settings.sync.totp.disabledToast"));
              onChanged();
            }
            return result.ok ? null : result.error;
          }}
          onClose={() => {
            setAsking(null);
          }}
        />
      )}
      {setup !== null && (
        <SetupDialog
          setup={setup}
          onDone={() => {
            toast(t("settings.sync.totp.enabledToast"));
            onChanged();
          }}
          onClose={() => {
            setSetup(null);
          }}
          returnFocus={() => toggle.current}
        />
      )}
    </div>
  );
}
