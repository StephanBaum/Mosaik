/**
 * Re-key notices in the sync folder that this device could not confirm (M7 review H1). A forged
 * or stray notice would keep sync paused, so each can be ignored, after a confirmation: a genuine
 * notice means the account was re-keyed elsewhere and this device stays on the old keys until it
 * adopts the re-key.
 */
import { useState } from "react";
import { Button } from "../../../components/Button.tsx";
import { Dialog } from "../../../components/Dialog.tsx";
import { ErrorAlert } from "../../../components/ErrorAlert.tsx";
import { useI18n } from "../../../i18n/I18nProvider.tsx";
import type { IpcError, RekeyMarkerView, SyncOverview } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";
import { isolate } from "../../sync/hooks.ts";

function IgnoreDialog({
  marker,
  onDismissed,
  onClose,
}: {
  marker: RekeyMarkerView;
  onDismissed: (overview: SyncOverview) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [error, setError] = useState<IpcError | null>(null);
  const [busy, setBusy] = useState(false);

  async function ignore() {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_dismiss_rekey_marker", { name: marker.name, sha256: marker.sha256 });
    setBusy(false);
    if (result.ok) {
      onDismissed(result.value);
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("rotation.marker.title")} onClose={onClose}>
      <div className="stack">
        <p className="body">{t("rotation.marker.confirm")}</p>
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("rotation.marker.keep")}
          </Button>
          <Button
            variant="dangerSolid"
            disabled={busy}
            onClick={() => {
              void ignore();
            }}
          >
            {t("rotation.marker.ignore")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

export function RekeyMarkerList({
  markers,
  onDismissed,
}: {
  markers: readonly RekeyMarkerView[];
  onDismissed: (marker: RekeyMarkerView, overview: SyncOverview) => void;
}) {
  const { t } = useI18n();
  const [asking, setAsking] = useState<RekeyMarkerView | null>(null);
  if (markers.length === 0) {
    return null;
  }
  return (
    <>
      <p className="body muted">{t("rotation.marker.hint")}</p>
      <div className="actions">
        {markers.map((marker) => (
          <Button
            key={marker.sha256}
            variant="ghost"
            aria-label={t("rotation.marker.open.label", { name: isolate(marker.name) })}
            onClick={() => {
              setAsking(marker);
            }}
          >
            {t("rotation.marker.open")}
          </Button>
        ))}
      </div>
      {asking !== null && (
        <IgnoreDialog
          marker={asking}
          onDismissed={(overview) => {
            onDismissed(asking, overview);
            setAsking(null);
          }}
          onClose={() => {
            setAsking(null);
          }}
        />
      )}
    </>
  );
}
