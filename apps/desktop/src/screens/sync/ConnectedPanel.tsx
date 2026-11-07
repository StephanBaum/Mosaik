/** Sync is set up: status, Sync now, warnings, devices, two-step verification and Disconnect. */
import { useState } from "react";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { UserText } from "../../components/UserText.tsx";
import { formatRelative } from "../../i18n/format.ts";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { IpcError, SyncConfigView, SyncOverview } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { DevicesCard } from "./DevicesCard.tsx";
import { HostedForm } from "./HostedForm.tsx";
import { useNow } from "./hooks.ts";
import { TotpCard } from "./TotpCard.tsx";
import { WarningList } from "./WarningList.tsx";

function Backend({ config }: { config: SyncConfigView }) {
  const { t } = useI18n();
  const rows =
    config.kind === "hosted"
      ? [
          [t("settings.sync.backend.server"), config.serverUrl],
          [t("settings.sync.backend.account"), config.email],
          [t("settings.sync.backend.device"), config.deviceName],
        ]
      : [
          [t("settings.sync.backend.folder"), config.displayPath],
          [t("settings.sync.backend.device"), config.deviceName],
        ];
  return (
    <dl className="stack">
      {rows.map(([label, value]) => (
        <div key={label}>
          <dt className="row__label">{label}</dt>
          <dd className="row__value">
            <UserText>{value}</UserText>
          </dd>
        </div>
      ))}
    </dl>
  );
}

function DisconnectDialog({
  onDone,
  onClose,
}: {
  onDone: (overview: SyncOverview) => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const [error, setError] = useState<IpcError | null>(null);

  async function disconnect() {
    const result = await ipc.call("sync_disconnect", null);
    if (result.ok) {
      onDone(result.value);
      onClose();
    } else {
      setError(result.error);
    }
  }

  return (
    <Dialog title={t("settings.sync.disconnect.title")} onClose={onClose}>
      <div className="stack">
        <p className="body muted">{t("settings.sync.disconnect.body")}</p>
        <ErrorAlert error={error} />
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("action.cancel")}
          </Button>
          <Button
            variant="dangerSolid"
            onClick={() => {
              void disconnect();
            }}
          >
            {t("settings.sync.disconnect.confirm")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

/** The status in a live region, the error of the last run and Sync now. */
function StatusBlock({
  state,
  onChange,
  onRefresh,
}: {
  state: SyncOverview["state"];
  onChange: (overview: SyncOverview) => void;
  onRefresh: () => void;
}) {
  const { t, locale } = useI18n();
  const ipc = useIpc();
  const now = useNow();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const { status, lastSyncedAt, error: stateError } = state;
  const signedOut = status === "signedOut";

  async function syncNow() {
    setBusy(true);
    setError(null);
    const result = await ipc.call("sync_now", null);
    setBusy(false);
    if (result.ok) {
      onChange(result.value);
    } else {
      setError(result.error);
      onRefresh();
    }
  }

  const statusLabel = t(status === "notConfigured" ? "sync.status.local" : `sync.status.${status}`);
  return (
    <div className="stack">
      <p className="row__label">{t("settings.sync.status.label")}</p>
      <p className="body" role="status" aria-live="polite">
        {statusLabel}
        {" · "}
        {lastSyncedAt === null
          ? t("settings.sync.neverSynced")
          : t("settings.sync.lastSynced", {
              when: formatRelative(locale, lastSyncedAt, Math.max(now, lastSyncedAt)),
            })}
      </p>
      {stateError !== null && !signedOut && <ErrorAlert error={{ code: stateError }} />}
      <ErrorAlert error={error} />
      {!signedOut && (
        <div className="actions">
          <Button
            disabled={busy}
            onClick={() => {
              void syncNow();
            }}
          >
            {t("settings.sync.now")}
          </Button>
        </div>
      )}
    </div>
  );
}

export function ConnectedPanel({
  overview,
  config,
  onChange,
  onRefresh,
}: {
  overview: SyncOverview;
  config: SyncConfigView;
  onChange: (overview: SyncOverview) => void;
  onRefresh: () => void;
}) {
  const { t } = useI18n();
  const [disconnecting, setDisconnecting] = useState(false);
  return (
    <div className="stack">
      <Backend config={config} />
      <StatusBlock state={overview.state} onChange={onChange} onRefresh={onRefresh} />
      {overview.state.status === "signedOut" && config.kind === "hosted" && (
        <div className="stack">
          <h3 className="subtitle">{t("settings.sync.signedOut.title")}</h3>
          <p className="caption">{t("settings.sync.signedOut.body")}</p>
          <HostedForm
            serverUrl={config.serverUrl}
            registration="closed"
            signInOnly
            deviceName={config.deviceName}
            onConnected={onChange}
          />
        </div>
      )}
      <WarningList warnings={overview.warnings} onChange={onChange} />
      <DevicesCard config={config} refreshKey={overview.state.lastSyncedAt} onRenamed={onRefresh} />
      {config.kind === "hosted" && <TotpCard enabled={config.totpEnabled} onChanged={onRefresh} />}
      <div className="actions">
        <Button
          variant="danger"
          onClick={() => {
            setDisconnecting(true);
          }}
        >
          {t("settings.sync.disconnect")}
        </Button>
      </div>
      {disconnecting && (
        <DisconnectDialog
          onDone={onChange}
          onClose={() => {
            setDisconnecting(false);
          }}
        />
      )}
    </div>
  );
}
