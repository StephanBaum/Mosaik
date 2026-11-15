/**
 * "Fill in programs" (Windows program fill, ADR-0164): the switch, the Enter switch, the hotkey
 * that opens the list, and the programs Mosaic was told not to offer logins for. Turning it on
 * needs no password. Where program fill cannot run (another system, Mosaic started as
 * administrator, the watcher failed) the section says why and offers nothing.
 */
import { useCallback, useEffect, useState, type KeyboardEvent } from "react";
import { useSettings } from "../app/SettingsProvider.tsx";
import { Button } from "../components/Button.tsx";
import { ErrorAlert } from "../components/ErrorAlert.tsx";
import { Switch } from "../components/Switch.tsx";
import { TextField } from "../components/TextField.tsx";
import { useToast } from "../components/ToastProvider.tsx";
import { UserText } from "../components/UserText.tsx";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { IpcError, ProgramFillStatus, Settings } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { usePlatform } from "../platform/platform.ts";
import { SettingsSection } from "./SettingsSection.tsx";
import { acceleratorOf, describeAccelerator } from "./ShortcutSettings.tsx";

const REASON_KEYS = {
  notWindows: "programFill.settings.notWindows",
  mosaicElevated: "programFill.settings.mosaicElevated",
  hostFailed: "programFill.settings.hostFailed",
} as const;

/** The status of program fill, read again whenever the settings change. */
export function useProgramFillStatus(settings: Settings | null): {
  status: ProgramFillStatus | null;
  setStatus: (status: ProgramFillStatus) => void;
} {
  const ipc = useIpc();
  const [status, setStatus] = useState<ProgramFillStatus | null>(null);
  const on = settings?.programFill;
  useEffect(() => {
    let current = true;
    void ipc.call("program_fill_status", null).then((result) => {
      if (current && result.ok) {
        setStatus(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, on]);
  return { status, setStatus };
}

function HotkeyField({ settings, onError }: { settings: Settings; onError: (error: IpcError | null) => void }) {
  const { t } = useI18n();
  const platform = usePlatform();
  const { update } = useSettings();
  const toast = useToast();
  const [draft, setDraft] = useState<string | null>(settings.programFillHotkey);
  const [busy, setBusy] = useState(false);
  const changed = draft !== settings.programFillHotkey;

  const save = async (next: string | null): Promise<void> => {
    setBusy(true);
    onError(null);
    const failure = await update({ ...settings, programFillHotkey: next }, null);
    setBusy(false);
    if (failure === null) {
      setDraft(next);
      toast(t("settings.saved"));
    } else {
      onError(failure);
    }
  };

  // The field records the key press, like the palette shortcut; Tab and Esc leave it.
  const record = (event: KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === "Tab" || (event.key === "Escape" && !event.ctrlKey && !event.altKey && !event.metaKey)) {
      return;
    }
    event.preventDefault();
    const next = acceleratorOf(event);
    if (next !== null) {
      setDraft(next);
      onError(null);
    }
  };

  return (
    <form
      className="stack setting-row__form"
      noValidate
      onSubmit={(event) => {
        event.preventDefault();
        if (changed) {
          void save(draft);
        }
      }}
    >
      <TextField
        label={t("programFill.settings.hotkey")}
        mono
        autoComplete="off"
        spellCheck={false}
        readOnly
        value={draft === null ? t("programFill.settings.hotkeyNone") : describeAccelerator(draft, platform, t)}
        onKeyDown={record}
      />
      <div className="actions">
        <Button type="submit" disabled={busy || !changed}>
          {t("programFill.settings.hotkeySave")}
        </Button>
        <Button
          variant="ghost"
          disabled={busy || (settings.programFillHotkey === null && draft === null)}
          onClick={() => {
            void save(null);
          }}
        >
          {t("programFill.settings.hotkeyNone")}
        </Button>
      </div>
    </form>
  );
}

function BlockedPrograms({
  status,
  onStatus,
  onError,
}: {
  status: ProgramFillStatus;
  onStatus: (status: ProgramFillStatus) => void;
  onError: (error: IpcError | null) => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const unblock = async (key: string): Promise<void> => {
    const result = await ipc.call("program_fill_unblock", { key });
    if (result.ok) {
      onError(null);
      onStatus(result.value);
    } else {
      onError(result.error);
    }
  };
  return (
    <div className="stack">
      <h3 className="editor__group-heading">{t("programFill.settings.blocked")}</h3>
      {status.blocked.length === 0 ? (
        <p className="caption">{t("programFill.settings.blockedNone")}</p>
      ) : (
        <ul className="rows">
          {status.blocked.map((entry) => (
            <li key={entry.key} className="row">
              <div className="row__main">
                <UserText>{entry.label}</UserText>
              </div>
              <Button
                variant="ghost"
                size="sm"
                aria-label={`${t("programFill.settings.unblock")}: ${entry.label}`}
                onClick={() => {
                  void unblock(entry.key);
                }}
              >
                {t("programFill.settings.unblock")}
              </Button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export function ProgramFillSettings() {
  const { t } = useI18n();
  const { settings, update } = useSettings();
  const toast = useToast();
  const { status, setStatus } = useProgramFillStatus(settings);
  const [error, setError] = useState<IpcError | null>(null);

  const change = useCallback(
    async (next: Settings): Promise<void> => {
      const failure = await update(next, null);
      setError(failure);
      if (failure === null) {
        toast(t("settings.saved"));
      }
    },
    [update, toast, t],
  );

  if (settings === null || status === null) {
    return null;
  }
  const reason = status.reason === null ? null : REASON_KEYS[status.reason];
  return (
    <SettingsSection title={t("programFill.settings.title")} body={t("programFill.settings.description")}>
      {!status.available ? (
        <p className="caption" role="status">
          {reason === null ? "" : t(reason)}
        </p>
      ) : (
        <div className="stack">
          <Switch
            label={t("programFill.settings.enable")}
            checked={settings.programFill}
            onChange={(event) => {
              void change({ ...settings, programFill: event.target.checked });
            }}
          />
          <Switch
            label={t("programFill.settings.submit")}
            checked={settings.programFillSubmit}
            disabled={!settings.programFill}
            onChange={(event) => {
              void change({ ...settings, programFillSubmit: event.target.checked });
            }}
          />
          <HotkeyField key={settings.programFillHotkey ?? ""} settings={settings} onError={setError} />
          <BlockedPrograms status={status} onStatus={setStatus} onError={setError} />
          <ErrorAlert
            error={error}
            message={error?.code === "invalidInput" ? t("settings.shortcut.invalid") : undefined}
          />
        </div>
      )}
    </SettingsSection>
  );
}
