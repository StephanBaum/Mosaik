/** Hooks of the sync settings: the overview (read again on every state event) and the device name. */
import { createContext, useCallback, useContext, useEffect, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { SyncOverview } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { usePlatform } from "../../platform/platform.ts";

const FocusSection = createContext<() => void>(() => {
  // No section to focus (a component rendered on its own).
});

/** Provided by the sync section: moves focus to its heading after its content was replaced. */
export const FocusSectionProvider = FocusSection.Provider;

/** Moves focus to the section heading: for a change that removes the control that had it. */
export function useFocusSection(): () => void {
  return useContext(FocusSection);
}

/** `sync_status`, again whenever Rust reports `syncStateChanged` or `refresh` is called. */
export function useSyncOverview() {
  const ipc = useIpc();
  const { sync } = useEvents();
  const [overview, setOverview] = useState<SyncOverview | null>(null);
  const [version, setVersion] = useState(0);
  useEffect(() => {
    let current = true;
    void ipc.call("sync_status", null).then((result) => {
      if (current && result.ok) {
        setOverview(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, sync, version]);
  const refresh = useCallback(() => {
    setVersion((n) => n + 1);
  }, []);
  return { overview, setOverview, refresh };
}

/** The default name of this device: "Mosaic on Windows", "...macOS" or "...Linux". */
export function useDefaultDeviceName(): string {
  const { t } = useI18n();
  const platform = usePlatform();
  const name =
    platform === "mac"
      ? t("sync.platform.macos")
      : platform === "windows"
        ? t("sync.platform.windows")
        : t("sync.platform.linux");
  return t("settings.sync.deviceName.default", { platform: name });
}

/** A time that moves on every 30 s, for "synced 5 minutes ago". */
export function useNow(): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 30_000);
    return () => {
      clearInterval(timer);
    };
  }, []);
  return now;
}

/** Wraps a user-supplied name so right-to-left text in it cannot reorder the sentence around it. */
export function isolate(name: string): string {
  return `⁨${name}⁩`;
}
