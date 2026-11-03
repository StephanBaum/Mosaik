/**
 * Settings, "Quick unlock" (D17): Windows Hello or Touch ID, how often the master password
 * is asked for again, and the registered security keys. Security keys show only where the
 * backend reports them as supported.
 */
import { useEffect, useState } from "react";
import { Button } from "../components/Button.tsx";
import { ReauthDialog } from "../components/ReauthDialog.tsx";
import { Select } from "../components/Select.tsx";
import { Switch } from "../components/Switch.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import {
  quickUnlockMaxAgeChoices,
  type IpcError,
  type QuickUnlockStatus,
  type Settings,
} from "../ipc/contract/index.ts";
import { KeysDialog } from "./SecurityKeys.tsx";
import { SettingsSection } from "./SettingsSection.tsx";

function useQuickStatus(reloadKey: unknown) {
  const ipc = useIpc();
  const [status, setStatus] = useState<QuickUnlockStatus | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("quick_unlock_status", null).then((result) => {
      if (current && result.ok) {
        setStatus(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, reloadKey]);
  return [status, setStatus] as const;
}

function BiometricRow({
  status,
  onStatus,
  onError,
}: {
  status: QuickUnlockStatus["biometric"];
  onStatus: (status: QuickUnlockStatus) => void;
  onError: (error: IpcError | null) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [ask, setAsk] = useState(false);
  const { platform } = status;

  if (platform === null) {
    return (
      <div className="setting-row">
        <p className="field__hint">{t("settings.quick.biometricLinux")}</p>
      </div>
    );
  }

  async function enroll(password: string | null): Promise<IpcError | null> {
    const result = await ipc.call("enroll_biometric", { password });
    if (result.ok) {
      onStatus(result.value);
      toast(t("settings.saved"));
    }
    return result.ok ? null : result.error;
  }

  async function toggle(on: boolean) {
    onError(null);
    if (!on) {
      const result = await ipc.call("remove_biometric", null);
      if (result.ok) {
        onStatus(result.value);
      }
      onError(result.ok ? null : result.error);
      return;
    }
    // Try without the password first: the backend accepts a recent re-authentication.
    const failure = await enroll(null);
    if (failure?.code === "reauthRequired") {
      setAsk(true);
    } else {
      onError(failure);
    }
  }

  const mac = platform === "touchId";
  return (
    <div className="setting-row">
      <Switch
        label={t(mac ? "settings.quick.touchId" : "settings.quick.windowsHello")}
        hint={t(mac ? "settings.quick.biometric.bodyMac" : "settings.quick.biometric.body")}
        checked={status.enrolled}
        disabled={!status.available && !status.enrolled}
        onChange={(event) => {
          void toggle(event.target.checked);
        }}
      />
      {!status.available && !status.enrolled && (
        <p className="field__hint">{t("settings.quick.biometricUnavailable")}</p>
      )}
      {status.enrolled && status.expiresAt !== null && (
        <p className="field__hint">{t("settings.quick.expires", { date: new Date(status.expiresAt) })}</p>
      )}
      {ask && (
        <ReauthDialog
          body="reauth.bodyQuick"
          onSubmit={enroll}
          onClose={() => {
            setAsk(false);
          }}
        />
      )}
    </div>
  );
}

function KeysRow({
  status,
  onStatus,
}: {
  status: QuickUnlockStatus["hardwareKeys"];
  onStatus: (status: QuickUnlockStatus) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [managing, setManaging] = useState(false);

  if (!status.supported) {
    return (
      <div className="setting-row">
        <p className="field__hint">{t("settings.quick.keysUnsupported")}</p>
      </div>
    );
  }
  return (
    <div className="setting-row">
      <span className="setting-row__label">{t("settings.quick.keys")}</span>
      <span className="caption">{t("settings.quick.keysCount", { count: status.keys.length })}</span>
      <Button
        onClick={() => {
          setManaging(true);
        }}
      >
        {t("settings.quick.manageKeys")}
      </Button>
      {managing && (
        <KeysDialog
          title={t("settings.quick.keys")}
          keys={status.keys}
          note={t("settings.quick.keyRisk")}
          withPassword
          onAdd={async (password, pin) => {
            const result = await ipc.call("enroll_hardware_key", { password, pin });
            if (result.ok) {
              onStatus(result.value);
            }
            return result.ok ? null : result.error;
          }}
          onRemove={async (methodId) => {
            const result = await ipc.call("remove_hardware_key", { methodId });
            if (result.ok) {
              onStatus(result.value);
            }
            return result.ok ? null : result.error;
          }}
          onClose={() => {
            setManaging(false);
          }}
        />
      )}
    </div>
  );
}

export function QuickUnlockSettings({
  settings,
  onChange,
  onError,
}: {
  settings: Settings;
  onChange: (next: Settings) => void;
  onError: (error: IpcError | null) => void;
}) {
  const { t } = useI18n();
  const [status, setStatus] = useQuickStatus(settings.quickUnlockMaxAgeDays);
  return (
    <SettingsSection title={t("settings.quick")} body={t("settings.quick.body")}>
      {status !== null && <BiometricRow status={status.biometric} onStatus={setStatus} onError={onError} />}
      <Select
        inline
        label={t("settings.quick.maxAge")}
        value={String(settings.quickUnlockMaxAgeDays)}
        options={quickUnlockMaxAgeChoices.map((days) => ({
          value: String(days),
          label: t("settings.quick.maxAgeDays", { days }),
        }))}
        onChange={(event) => {
          onChange({ ...settings, quickUnlockMaxAgeDays: Number(event.target.value) });
        }}
      />
      {status !== null && <KeysRow status={status.hardwareKeys} onStatus={setStatus} />}
    </SettingsSection>
  );
}
