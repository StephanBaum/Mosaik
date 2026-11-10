/** Sync is set up: status, Sync now, warnings, devices, two-step verification and Disconnect. */
import { useState, type ReactNode } from "react";
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
import { useFocusSection, useNow } from "./hooks.ts";
import { RecoverCard } from "./RecoverCard.tsx";
import { RekeyRecoveryForm } from "../settings/security/RekeyRecovery.tsx";
import { SecurityKeysCard } from "./SecurityKeysCard.tsx";
import { TotpCard } from "./TotpCard.tsx";
import { WarningList } from "./WarningList.tsx";

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="row">
      <dt className="row__label">{label}</dt>
      <dd className="row__value">{children}</dd>
    </div>
  );
}

/** The backend, this device and the status, as one list of rows like the other sections. */
function Details({
  config,
  state,
  attention,
}: {
  config: SyncConfigView;
  state: SyncOverview["state"];
  /** A finding that needs a decision (two devices re-keyed at once): never "Synced". */
  attention: boolean;
}) {
  const { t } = useI18n();
  const { status } = state;
  // Only the status word is announced: "last synced" changes with the clock.
  const statusLabel =
    attention && status === "upToDate"
      ? t("sync.status.attention")
      : t(status === "notConfigured" ? "sync.status.local" : `sync.status.${status}`);
  return (
    <dl className="rows rows--closed">
      {config.kind === "hosted" ? (
        <>
          <Row label={t("settings.sync.backend.server")}>
            <UserText>{config.serverUrl}</UserText>
          </Row>
          <Row label={t("settings.sync.backend.account")}>
            <UserText>{config.email}</UserText>
          </Row>
        </>
      ) : (
        <Row label={t("settings.sync.backend.folder")}>
          <UserText>{config.displayPath}</UserText>
        </Row>
      )}
      <Row label={t("settings.sync.backend.device")}>
        <UserText>{config.deviceName}</UserText>
      </Row>
      <Row label={t("settings.sync.status.label")}>
        <span role="status" aria-live="polite">
          {statusLabel}
        </span>
      </Row>
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
  const focusSection = useFocusSection();
  const [error, setError] = useState<IpcError | null>(null);

  async function disconnect() {
    const result = await ipc.call("sync_disconnect", null);
    if (result.ok) {
      onDone(result.value);
      onClose();
      // The panel (and the Disconnect button that had focus) is replaced by the setup cards.
      focusSection();
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

/** When it last synced, what is wrong (offline is not an error) and Sync now. */
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

  return (
    <div className="stack">
      <p className="caption">
        {lastSyncedAt === null
          ? t("settings.sync.neverSynced")
          : t("settings.sync.lastSynced", {
              when: formatRelative(locale, lastSyncedAt, Math.max(now, lastSyncedAt)),
            })}
      </p>
      {status === "offline" ? (
        <p className="body" role="status">
          {t("settings.sync.offline")}
        </p>
      ) : (
        stateError !== null && !signedOut && <ErrorAlert error={{ code: stateError }} />
      )}
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

/** A hosted account re-keyed elsewhere: the Secret Key from the new kit is required. */
function RekeyedCard({
  needsCode,
  onRecovered,
}: {
  needsCode: boolean;
  onRecovered: (overview: SyncOverview) => void;
}) {
  const { t } = useI18n();
  return (
    <div className="stack">
      <h3 className="subtitle">{t("rotation.recovery.title")}</h3>
      <RekeyRecoveryForm needsCode={needsCode} onRecovered={onRecovered} />
    </div>
  );
}

/** Two-step verification (a code) and the security keys of the sync account. */
function SecondFactorCards({
  config,
  signedOut,
  onChanged,
}: {
  config: Extract<SyncConfigView, { kind: "hosted" }>;
  signedOut: boolean;
  onChanged: () => void;
}) {
  return (
    <>
      <TotpCard enabled={config.totpEnabled} onChanged={onChanged} />
      {!signedOut && <SecurityKeysCard refreshKey={config.webauthnKeys} onChanged={onChanged} />}
    </>
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
  const signedOut = overview.state.status === "signedOut";
  const attention = overview.warnings.some((w) => w.kind === "unverifiedAccountFile" && w.conflict);
  // A hosted account that was re-keyed on another device: the new Secret Key is needed, not
  // just a password, so the recovery form replaces the password card and the sign-in form.
  const rekeyed = config.kind === "hosted" && overview.state.error === "syncAccountRekeyed";
  const recovered = (next: SyncOverview) => {
    onChange(next);
    onRefresh();
  };
  return (
    <div className="stack">
      <Details config={config} state={overview.state} attention={attention} />
      <StatusBlock state={overview.state} onChange={onChange} onRefresh={onRefresh} />
      {rekeyed && (
        <RekeyedCard needsCode={config.totpEnabled} onRecovered={recovered} />
      )}
      {overview.passwordChanged && !rekeyed && (
        <RecoverCard config={config} onRecovered={recovered} />
      )}
      {signedOut && config.kind === "hosted" && !rekeyed && (
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
      <DevicesCard
        config={config}
        signedOut={signedOut}
        refreshKey={overview.state.lastSyncedAt}
        onRenamed={onRefresh}
      />
      {config.kind === "hosted" && (
        <SecondFactorCards config={config} signedOut={signedOut} onChanged={onRefresh} />
      )}
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
