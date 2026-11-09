/**
 * "Ignored re-key notices" (Settings > Password and keys): notices the person chose to ignore stay
 * listed, each with a way back. Restoring is safe (the notice only pauses sync again so the
 * re-key it names can be adopted), so it asks nothing. Shown only when something is ignored.
 */
import { useEffect, useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, RekeyMarkerView } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { SettingsSection } from "../../SettingsSection.tsx";
import { isolate } from "../../sync/hooks.ts";

export function IgnoredNotices() {
  const { t } = useI18n();
  const ipc = useIpc();
  const [notices, setNotices] = useState<RekeyMarkerView[]>([]);
  const [error, setError] = useState<IpcError | null>(null);

  useEffect(() => {
    let current = true;
    void ipc.call("sync_ignored_rekey_markers", null).then((result) => {
      if (current && result.ok) {
        setNotices(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);

  async function restore(notice: RekeyMarkerView) {
    setError(null);
    const result = await ipc.call("sync_restore_rekey_marker", { name: notice.name, sha256: notice.sha256 });
    if (result.ok) {
      // The status event refreshes the re-key notice: rekey-pending is set again.
      setNotices((all) => all.filter((n) => n.sha256 !== notice.sha256));
    } else {
      setError(result.error);
    }
  }

  if (notices.length === 0) {
    return null;
  }
  return (
    <SettingsSection title={t("rotation.ignored.title")} body={t("rotation.ignored.body")}>
      {notices.map((notice) => (
        <div key={notice.sha256} className="setting-row">
          <div className="setting-row__label">
            <p className="body">{t("rotation.ignored.row")}</p>
            <p className="caption">{isolate(notice.name)}</p>
          </div>
          <Button
            aria-label={t("rotation.ignored.restore.label", { name: isolate(notice.name) })}
            onClick={() => {
              void restore(notice);
            }}
          >
            {t("rotation.ignored.restore")}
          </Button>
        </div>
      ))}
      <ErrorAlert error={error} />
    </SettingsSection>
  );
}
