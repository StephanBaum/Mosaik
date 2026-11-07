/**
 * The "Sync" group of the settings (M6): set up a Mosaic server or an own cloud folder, then
 * status, warnings, devices and two-step verification (docs/DESKTOP_IPC.md §3 "Sync"). The
 * overview is read again whenever Rust reports `syncStateChanged`.
 */
import { useI18n } from "../i18n/I18nProvider.tsx";
import { ConnectedPanel } from "./sync/ConnectedPanel.tsx";
import { useSyncOverview } from "./sync/hooks.ts";
import { SetupCards } from "./sync/SetupCards.tsx";
import { SettingsSection } from "./SettingsSection.tsx";

export function SyncSettings() {
  const { t } = useI18n();
  const { overview, setOverview, refresh } = useSyncOverview();
  return (
    <SettingsSection title={t("settings.sync.title")} body={t("settings.sync.body")}>
      {overview === null ? null : overview.config === null ? (
        <SetupCards onConnected={setOverview} />
      ) : (
        <ConnectedPanel
          overview={overview}
          config={overview.config}
          onChange={setOverview}
          onRefresh={refresh}
        />
      )}
    </SettingsSection>
  );
}
