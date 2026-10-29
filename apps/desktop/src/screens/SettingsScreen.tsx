/**
 * Settings (D17): auto-lock, clipboard, appearance, the SSH agent and the Emergency Kit.
 * A change that weakens protection asks for the master password (`reauthRequired`).
 */
import { useEffect, useState } from "react";
import { useSettings } from "../app/SettingsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { ReauthDialog } from "../components/ReauthDialog.tsx";
import { Select } from "../components/Select.tsx";
import { Switch } from "../components/Switch.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import {
  languages,
  themes,
  type AppStatus,
  type IpcError,
  type Settings,
  type SshAgentStatus,
} from "../ipc/contract/index.ts";
import { usePlatform } from "../platform/platform.ts";
import { format, GLOBAL_PALETTE } from "../platform/shortcuts.ts";
import { kitLabels } from "./onboarding/kit.ts";

const AUTO_LOCK = [1, 5, 10, 30, 60];
const CLIPBOARD = [10, 30, 60, 120];

/**
 * Whether a lock trigger that can fire is on (mirrors Rust `has_lock_trigger`, CRYPTO_SPEC
 * §11): screen lock counts only where the system can detect it.
 */
function hasLockTrigger(s: Settings, screenLockDetection: boolean): boolean {
  return s.autoLockMinutes !== null || s.lockOnSleep || (s.lockOnScreenLock && screenLockDetection);
}

/** A number or "never" (`null`) from a select value. */
const optional = (value: string): number | null => (value === "never" ? null : Number(value));

function SecuritySettings({
  settings,
  status,
  lockRequired,
  onChange,
}: {
  settings: Settings;
  status: AppStatus | null;
  lockRequired: boolean;
  onChange: (next: Settings) => void;
}) {
  const { t } = useI18n();
  const platform = usePlatform();
  return (
    <>
      <h2 className="subtitle">{t("settings.security")}</h2>
      <Select
        label={t("settings.autoLock")}
        value={settings.autoLockMinutes === null ? "never" : String(settings.autoLockMinutes)}
        options={[
          ...AUTO_LOCK.map((minutes) => ({
            value: String(minutes),
            label: t("settings.autoLock.minutes", { minutes }),
          })),
          { value: "never", label: t("settings.autoLock.never") },
        ]}
        onChange={(event) => {
          onChange({ ...settings, autoLockMinutes: optional(event.target.value) });
        }}
      />
      <Switch
        label={t("settings.lockOnSleep")}
        checked={settings.lockOnSleep}
        onChange={(event) => {
          onChange({ ...settings, lockOnSleep: event.target.checked });
        }}
      />
      <Switch
        label={t("settings.lockOnScreenLock")}
        checked={settings.lockOnScreenLock}
        onChange={(event) => {
          onChange({ ...settings, lockOnScreenLock: event.target.checked });
        }}
      />
      {status?.screenLockDetection === false && (
        <p className="field__hint">{t("settings.screenLock.unsupported")}</p>
      )}
      {lockRequired && (
        <p className="field__hint" role="status">
          {t("settings.lockTrigger.required")}
        </p>
      )}
      {status?.shortcut === "conflict" && (
        <p className="field__hint">
          {t("shell.shortcut.conflict", { shortcut: format(GLOBAL_PALETTE[platform], platform, t) })}
        </p>
      )}
      <Select
        label={t("settings.clipboard")}
        value={settings.clipboardClearSeconds === null ? "never" : String(settings.clipboardClearSeconds)}
        options={[
          ...CLIPBOARD.map((seconds) => ({
            value: String(seconds),
            label: t("settings.clipboard.seconds", { seconds }),
          })),
          { value: "never", label: t("settings.clipboard.never") },
        ]}
        onChange={(event) => {
          onChange({ ...settings, clipboardClearSeconds: optional(event.target.value) });
        }}
      />
    </>
  );
}

function Appearance({ settings, onChange }: { settings: Settings; onChange: (next: Settings) => void }) {
  const { t } = useI18n();
  return (
    <>
      <h2 className="subtitle">{t("settings.appearance")}</h2>
      <Select
        label={t("settings.theme")}
        value={settings.theme}
        options={themes.map((theme) => ({ value: theme, label: t(`settings.theme.${theme}`) }))}
        onChange={(event) => {
          onChange({ ...settings, theme: themes.find((x) => x === event.target.value) ?? "system" });
        }}
      />
      <Select
        label={t("settings.language")}
        value={settings.language}
        options={languages.map((language) => ({
          value: language,
          label: t(`settings.language.${language}`),
        }))}
        onChange={(event) => {
          onChange({
            ...settings,
            language: languages.find((x) => x === event.target.value) ?? "system",
          });
        }}
      />
    </>
  );
}

function SshSettings({ onError }: { onError: (error: IpcError | null) => void }) {
  const { t, errorMessage } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [ssh, setSsh] = useState<SshAgentStatus | null>(null);

  useEffect(() => {
    void ipc.call("ssh_agent_status", null).then((result) => {
      if (result.ok) {
        setSsh(result.value);
      }
    });
  }, [ipc]);

  async function setAgent(enabled: boolean) {
    const result = await ipc.call("ssh_agent_set_enabled", { enabled });
    onError(result.ok ? null : result.error);
    const status = result.ok ? result : await ipc.call("ssh_agent_status", null);
    if (status.ok) {
      setSsh(status.value);
    }
  }

  async function copyEndpoint(endpoint: string) {
    try {
      await navigator.clipboard.writeText(endpoint);
    } catch {
      toast(errorMessage({ code: "clipboardUnavailable" }));
    }
  }

  const endpoint = ssh?.endpoint ?? null;
  return (
    <>
      <h2 className="subtitle">{t("settings.ssh")}</h2>
      <Switch
        label={t("settings.ssh.enable")}
        hint={t("settings.ssh.hint")}
        checked={ssh?.enabled ?? false}
        onChange={(event) => {
          void setAgent(event.target.checked);
        }}
      />
      {ssh?.running === true && <p className="body">{t("settings.ssh.status", { count: ssh.keyCount })}</p>}
      {endpoint !== null && (
        <div className="secret__row">
          <span>{t("settings.ssh.endpoint")}</span>
          <code className="mono">{endpoint}</code>
          <Button
            variant="ghost"
            onClick={() => {
              void copyEndpoint(endpoint);
            }}
          >
            {t("action.copy")}
          </Button>
        </div>
      )}
      {ssh?.error != null && (
        <p className="alert" role="alert">
          {errorMessage({ code: ssh.error })}
        </p>
      )}
      {ssh !== null && ssh.skipped.length > 0 && (
        <>
          <h3 className="subtitle">{t("settings.ssh.skipped")}</h3>
          <ul>
            {ssh.skipped.map((key) => (
              <li key={key.itemId}>{t(`ssh.skip.${key.reason}`)}</li>
            ))}
          </ul>
        </>
      )}
    </>
  );
}

function useAppStatus(): AppStatus | null {
  const ipc = useIpc();
  const [status, setStatus] = useState<AppStatus | null>(null);
  useEffect(() => {
    void ipc.call("app_status", null).then((result) => {
      if (result.ok) {
        setStatus(result.value);
      }
    });
  }, [ipc]);
  return status;
}

/** Saving the Emergency Kit again: always behind the master password. */
function KitSettings() {
  const { t } = useI18n();
  const ipc = useIpc();
  const [ask, setAsk] = useState(false);
  return (
    <>
      <h2 className="subtitle">{t("settings.kit")}</h2>
      <Button
        onClick={() => {
          setAsk(true);
        }}
      >
        {t("settings.kit.save")}
      </Button>
      {ask && (
        <ReauthDialog
          kit
          onSubmit={async (password) => {
            const result = await ipc.call("save_emergency_kit", { labels: kitLabels(t), password });
            return result.ok ? null : result.error;
          }}
          onClose={() => {
            setAsk(false);
          }}
        />
      )}
    </>
  );
}

export function SettingsScreen() {
  const { t } = useI18n();
  const toast = useToast();
  const { settings, update } = useSettings();
  const status = useAppStatus();
  const [error, setError] = useState<IpcError | null>(null);
  const [pending, setPending] = useState<Settings | null>(null);
  const [lockRequired, setLockRequired] = useState(false);

  async function change(next: Settings) {
    // The last effective lock trigger cannot be turned off (CRYPTO_SPEC §11).
    const allowed = hasLockTrigger(next, status?.screenLockDetection ?? false);
    setLockRequired(!allowed);
    if (!allowed) {
      return;
    }
    const failure = await update(next, null);
    if (failure?.code === "reauthRequired") {
      setPending(next);
      return;
    }
    setError(failure);
    if (failure === null) {
      toast(t("settings.saved"));
    }
  }

  if (settings === null) {
    return null;
  }
  return (
    <section className="stack" aria-label={t("settings.title")}>
      <h1 className="title">{t("settings.title")}</h1>
      <ErrorAlert error={error} />
      <SecuritySettings
        settings={settings}
        status={status}
        lockRequired={lockRequired}
        onChange={(next) => {
          void change(next);
        }}
      />
      <Appearance
        settings={settings}
        onChange={(next) => {
          void change(next);
        }}
      />
      <SshSettings onError={setError} />
      <KitSettings />
      {pending !== null && (
        <ReauthDialog
          onSubmit={async (password) => {
            const failure = await update(pending, password);
            if (failure === null) {
              toast(t("settings.saved"));
            }
            return failure;
          }}
          onClose={() => {
            setPending(null);
          }}
        />
      )}
    </section>
  );
}
