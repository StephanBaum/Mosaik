/**
 * "Quick search shortcut" (M3-R10): the global accelerator that opens the palette from any
 * program. Rust validates it (`shortcut::parse_accelerator`) and registers it when it changes.
 * A combination another program holds is `shortcutUnavailable`; where the system has no global
 * shortcuts (Wayland) the field is disabled and says why.
 */
import { useState, type SyntheticEvent } from "react";
import { useSettings } from "../app/SettingsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { TextField } from "../components/TextField.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { AppStatus, IpcError, Settings } from "../ipc/contract/index.ts";
import { usePlatform, type Platform } from "../platform/platform.ts";
import { SettingsSection } from "./SettingsSection.tsx";

/** The default accelerator of this system, as Rust writes it. */
function defaultShortcut(platform: Platform): string {
  return platform === "mac" ? "Alt+Super+Space" : "Ctrl+Shift+Space";
}

export function ShortcutSettings({ settings, status }: { settings: Settings; status: AppStatus | null }) {
  const { t, errorMessage } = useI18n();
  const platform = usePlatform();
  const toast = useToast();
  const { update } = useSettings();
  const [draft, setDraft] = useState(settings.globalShortcut);
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);
  const unsupported = status?.shortcut === "unsupported";
  const changed = draft.trim() !== settings.globalShortcut;

  async function save(next: string) {
    setBusy(true);
    setError(null);
    const failure = await update({ ...settings, globalShortcut: next }, null);
    setBusy(false);
    if (failure === null) {
      setDraft(next);
      toast(t("settings.shortcut.saved", { shortcut: next }));
    } else {
      setError(failure);
    }
  }

  function submit(event: SyntheticEvent) {
    event.preventDefault();
    if (!unsupported && changed) {
      void save(draft.trim());
    }
  }

  return (
    <SettingsSection title={t("settings.shortcut.title")} body={t("settings.shortcut.body")}>
      <form className="setting-row stack" onSubmit={submit} noValidate>
        <TextField
          label={t("settings.shortcut.label")}
          hint={unsupported ? t("settings.shortcut.unsupported") : t("settings.shortcut.hint")}
          mono
          autoComplete="off"
          spellCheck={false}
          maxLength={64}
          value={draft}
          disabled={unsupported}
          onChange={(event) => {
            setDraft(event.target.value);
          }}
        />
        {error !== null && (
          <ErrorAlert
            error={error}
            message={error.code === "invalidInput" ? t("settings.shortcut.invalid") : errorMessage(error)}
          />
        )}
        <div className="actions">
          <Button type="submit" disabled={unsupported || busy || !changed}>
            {t("settings.shortcut.save")}
          </Button>
          <Button
            variant="ghost"
            disabled={unsupported || busy || settings.globalShortcut === defaultShortcut(platform)}
            onClick={() => {
              void save(defaultShortcut(platform));
            }}
          >
            {t("settings.shortcut.reset")}
          </Button>
        </div>
      </form>
    </SettingsSection>
  );
}
