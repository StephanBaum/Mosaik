/**
 * Settings (D17): auto-lock, clipboard, appearance, sync, the SSH agent and the Emergency Kit.
 * A change that weakens protection asks for the master password (`reauthRequired`).
 */
import { useEffect, useState } from "react";
import { IS_PREVIEW } from "../app/preview.ts";
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
} from "../ipc/contract/index.ts";
import { usePlatform } from "../platform/platform.ts";
import { format, GLOBAL_PALETTE } from "../platform/shortcuts.ts";
import { QuickUnlockSettings } from "./QuickUnlockSettings.tsx";
import { AccountKeysSection } from "./settings/security/AccountKeysSection.tsx";
import { kitLabels } from "./onboarding/kit.ts";
import { SettingsSection } from "./SettingsSection.tsx";
import { BrowserSettings } from "./BrowserSettings.tsx";
import { PasskeyExportSettings } from "./PasskeyExportSettings.tsx";
import { EmergencySettings } from "./sharing/EmergencySettings.tsx";
import { SharingSettings } from "./sharing/SharingSettings.tsx";
import { ShortcutSettings } from "./ShortcutSettings.tsx";
import { SshSettings } from "./SshSettings.tsx";
import { SyncSettings } from "./SyncSettings.tsx";

const AUTO_LOCK = [1, 5, 10, 30, 60];

/** Whether `text` is the default global shortcut of a system, which has its own notation per platform. */
const isDefaultShortcut = (text: string): boolean => text === "Alt+Super+Space" || text === "Ctrl+Shift+Space";
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

function AutoLockSettings({
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
    <SettingsSection title={t("settings.section.autoLock")} body={t("settings.section.autoLock.body")}>
      <Select
        inline
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
      <div className="setting-row">
        <Switch
          label={t("settings.lockOnSleep")}
          checked={settings.lockOnSleep}
          onChange={(event) => {
            onChange({ ...settings, lockOnSleep: event.target.checked });
          }}
        />
      </div>
      <div className="setting-row">
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
            {t("shell.shortcut.conflict", {
              shortcut: isDefaultShortcut(settings.globalShortcut) ? format(GLOBAL_PALETTE[platform], platform, t) : settings.globalShortcut,
            })}
          </p>
        )}
      </div>
    </SettingsSection>
  );
}

function ClipboardSettings({ settings, onChange }: { settings: Settings; onChange: (next: Settings) => void }) {
  const { t } = useI18n();
  return (
    <SettingsSection title={t("settings.section.clipboard")} body={t("settings.section.clipboard.body")}>
      <Select
        inline
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
    </SettingsSection>
  );
}

function ReminderSettings({ settings, onChange }: { settings: Settings; onChange: (next: Settings) => void }) {
  const { t } = useI18n();
  return (
    <SettingsSection title={t("credentialRotation.settings.title")} body={t("credentialRotation.settings.body")}>
      <div className="setting-row">
        <Switch
          label={t("credentialRotation.settings.reminders")}
          hint={t("credentialRotation.settings.remindersHint")}
          checked={settings.rotationReminders}
          onChange={(event) => {
            onChange({ ...settings, rotationReminders: event.target.checked });
          }}
        />
      </div>
    </SettingsSection>
  );
}

function Appearance({ settings, onChange }: { settings: Settings; onChange: (next: Settings) => void }) {
  const { t } = useI18n();
  return (
    <SettingsSection title={t("settings.appearance")} body={t("settings.appearance.body")}>
      <Select
        inline
        label={t("settings.theme")}
        value={settings.theme}
        options={themes.map((theme) => ({ value: theme, label: t(`settings.theme.${theme}`) }))}
        onChange={(event) => {
          onChange({ ...settings, theme: themes.find((x) => x === event.target.value) ?? "system" });
        }}
      />
      <Select
        inline
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
    </SettingsSection>
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
    <SettingsSection title={t("settings.kit")} body={t("settings.kit.body")}>
      <div className="setting-row">
        <Button
          onClick={() => {
            setAsk(true);
          }}
        >
          {t("settings.kit.save")}
        </Button>
      </div>
      {ask && (
        <ReauthDialog
          kit
          onSubmit={async (password) => {
            const result = await ipc.call("save_emergency_kit", { labels: kitLabels(t), pending: false, password });
            return result.ok ? null : result.error;
          }}
          onClose={() => {
            setAsk(false);
          }}
        />
      )}
    </SettingsSection>
  );
}

/** Sync, the groups for programs that use the vault (the SSH agent, browser extensions) and the passkey export. */
function Integrations({ onError }: { onError: (error: IpcError | null) => void }) {
  return (
    <>
      <AccountKeysSection />
      <SyncSettings />
      <SharingSettings />
      <EmergencySettings />
      <SshSettings onError={onError} />
      <BrowserSettings onError={onError} />
      <PasskeyExportSettings />
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

  const apply = (next: Settings) => {
    void change(next);
  };

  if (settings === null) {
    return null;
  }
  return (
    <section className="settings" aria-label={t("settings.title")}>
      <div className="settings__intro">
        <h1 className="display">{t("settings.title")}</h1>
        <p className="lede">{IS_PREVIEW ? `${t("settings.intro")} ${t("app.previewBuild")}` : t("settings.intro")}</p>
      </div>
      <ErrorAlert error={error} />
      <AutoLockSettings
        settings={settings}
        status={status}
        lockRequired={lockRequired}
        onChange={apply}
      />
      <ClipboardSettings
        settings={settings}
        onChange={apply}
      />
      <ReminderSettings
        settings={settings}
        onChange={apply}
      />
      <ShortcutSettings settings={settings} status={status} />
      <Appearance
        settings={settings}
        onChange={apply}
      />
      <QuickUnlockSettings
        settings={settings}
        onChange={apply}
        onError={setError}
      />
      <Integrations onError={setError} />
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
