/**
 * "Quick search shortcut" (M3-R10): the global accelerator that opens the palette from any
 * program. The field records the key press (nothing is typed); it shows the keys with the names
 * of this system and stores the accelerator as Rust writes it. Rust validates it
 * (`shortcut::parse_accelerator`) and registers it when it changes. A combination another program
 * holds is `shortcutUnavailable`; where the system has no global shortcuts the field is disabled
 * and says why.
 */
import { useState, type KeyboardEvent, type SyntheticEvent } from "react";
import type { MessageKey } from "@mosaic/i18n";
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

/** The key of an accelerator for `event`, or null while only modifiers are held. */
function keyOf(event: KeyboardEvent): string | null {
  const code = event.code;
  if (/^(Control|Alt|Shift|Meta|OS)(Left|Right)$/.test(code) || code === "") {
    return null;
  }
  const letter = /^Key([A-Z])$/.exec(code) ?? /^Digit(\d)$/.exec(code);
  return letter === null ? code : (letter[1] ?? code);
}

/** The accelerator the pressed keys make, as Rust writes it (`Ctrl+Shift+Space`). */
export function acceleratorOf(event: KeyboardEvent): string | null {
  const key = keyOf(event);
  if (key === null) {
    return null;
  }
  const mods = [
    event.ctrlKey ? "Ctrl" : null,
    event.altKey ? "Alt" : null,
    event.shiftKey ? "Shift" : null,
    event.metaKey ? "Super" : null,
  ].filter((mod): mod is string => mod !== null);
  return [...mods, key].join("+");
}

/** An accelerator with the key names of this system: Ctrl or Strg, and ⌘ ⌥ ⇧ ⌃ on macOS. */
export function describeAccelerator(
  accelerator: string,
  platform: Platform,
  t: (key: MessageKey) => string,
): string {
  const names: Record<string, string> =
    platform === "mac"
      ? { ctrl: "⌃", control: "⌃", alt: "⌥", option: "⌥", shift: "⇧", super: "⌘", cmd: "⌘", command: "⌘", meta: "⌘" }
      : {
          ctrl: t("keys.ctrl"),
          control: t("keys.ctrl"),
          alt: t("keys.alt"),
          option: t("keys.alt"),
          shift: t("keys.shift"),
          super: platform === "windows" ? "Win" : "Super",
          cmd: platform === "windows" ? "Win" : "Super",
          command: platform === "windows" ? "Win" : "Super",
          meta: platform === "windows" ? "Win" : "Super",
        };
  const parts = accelerator.split("+").map((part) => {
    const lower = part.toLowerCase();
    if (lower === "space") {
      return t("keys.space");
    }
    return names[lower] ?? part;
  });
  return platform === "mac" ? parts.join("") : parts.join("+");
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
      toast(t("settings.shortcut.saved", { shortcut: describeAccelerator(next, platform, t) }));
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

  // The field records the key press. Tab (and Esc) leave it: a keyboard user is never trapped.
  function record(event: KeyboardEvent<HTMLInputElement>) {
    if (unsupported || event.key === "Tab" || (event.key === "Escape" && !event.ctrlKey && !event.altKey && !event.metaKey)) {
      return;
    }
    event.preventDefault();
    const next = acceleratorOf(event);
    if (next !== null) {
      setDraft(next);
      setError(null);
    }
  }

  return (
    <SettingsSection title={t("settings.shortcut.title")} body={t("settings.shortcut.body")}>
      <div className="setting-row">
        <form className="stack setting-row__form" onSubmit={submit} noValidate>
          <TextField
            label={t("settings.shortcut.label")}
            hint={unsupported ? t("settings.shortcut.unsupported") : t("settings.shortcut.hint")}
            mono
            autoComplete="off"
            spellCheck={false}
            readOnly
            value={describeAccelerator(draft, platform, t)}
            disabled={unsupported}
            onKeyDown={record}
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
      </div>
    </SettingsSection>
  );
}
