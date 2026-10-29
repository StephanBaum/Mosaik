/** The settings (`get_settings` / `update_settings`), shared by the theme, the language and the settings screen. */
import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcError, Settings } from "../ipc/contract/index.ts";

interface SettingsContextValue {
  settings: Settings | null;
  /** Saves the settings; resolves to the error, or `null`. `password` re-authenticates. */
  update: (next: Settings, password: string | null) => Promise<IpcError | null>;
}

const SettingsContext = createContext<SettingsContextValue | null>(null);

export function SettingsProvider({ children }: { children: ReactNode }) {
  const ipc = useIpc();
  const [settings, setSettings] = useState<Settings | null>(null);

  useEffect(() => {
    void ipc.call("get_settings", null).then((result) => {
      if (result.ok) {
        setSettings(result.value);
      }
    });
  }, [ipc]);

  const update = useCallback(
    async (next: Settings, password: string | null) => {
      const result = await ipc.call("update_settings", { settings: next, password });
      if (result.ok) {
        setSettings(result.value);
        return null;
      }
      return result.error;
    },
    [ipc],
  );

  const value = useMemo(() => ({ settings, update }), [settings, update]);
  return <SettingsContext value={value}>{children}</SettingsContext>;
}

export function useSettings(): SettingsContextValue {
  const context = useContext(SettingsContext);
  if (context === null) {
    throw new Error("useSettings needs a SettingsProvider");
  }
  return context;
}
