/**
 * The "Sync" group of the settings (M6): set up a Mosaic server or an own cloud folder, then
 * status, warnings, devices and two-step verification (docs/DESKTOP_IPC.md section 3 "Sync").
 * The overview is read again whenever Rust reports `syncStateChanged`. The section's content is
 * replaced when sync is connected or disconnected; focus then moves to the section heading
 * (the control that had it is gone).
 */
import { useCallback, useEffect, useRef } from "react";
import { useI18n } from "../i18n/I18nProvider.tsx";
import { ConnectedPanel } from "./sync/ConnectedPanel.tsx";
import { FocusSectionProvider, useSyncOverview } from "./sync/hooks.ts";
import { SetupCards } from "./sync/SetupCards.tsx";
import { SYNC_HEADING_ID } from "./sharing/hooks.ts";
import { SettingsSection } from "./SettingsSection.tsx";

export function SyncSettings() {
  const { t } = useI18n();
  const { overview, setOverview, refresh } = useSyncOverview();
  const heading = useRef<HTMLHeadingElement>(null);
  const focusWanted = useRef(false);
  const focusSection = useCallback(() => {
    focusWanted.current = true;
  }, []);
  // After the render that replaced the content (every caller changes the overview too).
  useEffect(() => {
    if (focusWanted.current) {
      focusWanted.current = false;
      heading.current?.focus();
    }
  }, [overview]);
  return (
    <SettingsSection
      title={t("settings.sync.title")}
      body={t("settings.sync.body")}
      headingRef={heading}
      headingId={SYNC_HEADING_ID}
    >
      <FocusSectionProvider value={focusSection}>
        {overview === null ? null : overview.config === null ? (
          <SetupCards
            onConnected={(next) => {
              setOverview(next);
              focusSection();
            }}
          />
        ) : (
          <ConnectedPanel
            overview={overview}
            config={overview.config}
            onChange={setOverview}
            onRefresh={refresh}
          />
        )}
      </FocusSectionProvider>
    </SettingsSection>
  );
}
