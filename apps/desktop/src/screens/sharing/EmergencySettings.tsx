/**
 * Settings, "Emergency access": the people who can ask for access to this account's vaults (as
 * grantor) and the people who named this account (as grantee). It is not account recovery: it
 * covers vault keys only, read-only, after a waiting time the server keeps. A grantee asks,
 * waits and claims; a grantor sees the request in the banner and can refuse or approve early.
 */
import { useCallback, useEffect, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { Button } from "../../components/Button.tsx";
import { Dialog } from "../../components/Dialog.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { EmergencyOverview, GrantedView, IpcError, ReceivedView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate, useNow } from "../sync/hooks.ts";
import { SettingsSection } from "../SettingsSection.tsx";
import { GrantedSection, ReceivedSection } from "./EmergencyRows.tsx";
import { useEmergencyActions } from "./useEmergencyActions.ts";
import { GrantDialog } from "./GrantDialog.tsx";
import { openSyncSettings, useBackendKind, type BackendKind } from "./hooks.ts";
import { NotHostedNote } from "./NotHostedNote.tsx";
import { ConfirmDialog } from "./ConfirmDialog.tsx";
import { VerifyContactDialog } from "./VerifyContactDialog.tsx";

/** The grants given and received, read again on every `sharingChanged` and on `reload`. */
function useOverview(kind: BackendKind) {
  const ipc = useIpc();
  const { sharing } = useEvents();
  const [overview, setOverview] = useState<EmergencyOverview | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [local, setLocal] = useState(0);
  useEffect(() => {
    if (kind !== "hosted") {
      return undefined;
    }
    const live = { current: true };
    void ipc.call("emergency_overview", null).then((result) => {
      if (!live.current) {
        return;
      }
      setOverview(result.ok ? result.value : null);
      setError(result.ok ? null : result.error);
    });
    return () => {
      live.current = false;
    };
  }, [ipc, kind, sharing.version, local]);
  const reload = useCallback(() => {
    setLocal((n) => n + 1);
  }, []);
  return { overview, error, setError, reload };
}

function RemoveGrantDialog({
  grant,
  onRemove,
  onClose,
}: {
  grant: GrantedView;
  onRemove: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  return (
    <Dialog title={t("emergency.granted.remove.title", { email: isolate(grant.email) })} onClose={onClose}>
      <div className="stack">
        <p className="body muted">{t("emergency.granted.remove.body")}</p>
        <div className="dialog__actions">
          <Button data-autofocus="" onClick={onClose}>
            {t("action.cancel")}
          </Button>
          <Button
            variant="dangerSolid"
            onClick={() => {
              onRemove();
              onClose();
            }}
          >
            {t("emergency.granted.remove")}
          </Button>
        </div>
      </div>
    </Dialog>
  );
}

/** Emergency access is off: why, and a way to the sync settings. */
function Off({ kind }: { kind: "folder" | "none" }) {
  return (
    <NotHostedNote
      kind={kind}
      message={kind === "folder" ? "emergency.folderUnsupported" : "sharing.notHosted.none"}
      onOpenSync={openSyncSettings}
    />
  );
}

/** Asking tells the grantor, so it asks first. */
function RequestDialog({
  grant,
  onRequest,
  onClose,
}: {
  grant: ReceivedView;
  onRequest: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const email = isolate(grant.email);
  return (
    <ConfirmDialog
      title={t("emergency.received.request.title", { email })}
      body={t("emergency.received.request.body", { email, days: grant.waitDays })}
      confirmLabel={t("emergency.received.request")}
      variant="secondary"
      onConfirm={onRequest}
      onClose={onClose}
    />
  );
}

export function EmergencySettings() {
  const { t } = useI18n();
  const toast = useToast();
  const now = useNow();
  const { kind, totp } = useBackendKind();
  const { overview, error, setError, reload } = useOverview(kind);
  const [granting, setGranting] = useState(false);
  const [removing, setRemoving] = useState<GrantedView | null>(null);
  const [verifying, setVerifying] = useState<string | null>(null);
  const [asking, setAsking] = useState<ReceivedView | null>(null);
  const actions = useEmergencyActions(reload, setError);

  return (
    <SettingsSection title={t("emergency.title")} body={t("emergency.body")}>
      {(kind === "folder" || kind === "none") && <Off kind={kind} />}
      <ErrorAlert error={error} />
      {kind === "hosted" && overview !== null && (
        <>
          <GrantedSection
            granted={overview.granted}
            onAdd={() => {
              setGranting(true);
            }}
            onRemove={setRemoving}
          />
          <ReceivedSection
            received={overview.received}
            now={now}
            busy={actions.busy}
            unverified={actions.unverified}
            onVerify={setVerifying}
            onRequest={setAsking}
            onClaim={(grant) => void actions.claim(grant)}
          />
        </>
      )}
      {granting && (
        <GrantDialog
          totp={totp}
          onGranted={(email) => {
            toast(t("emergency.grant.done", { email: isolate(email) }));
            reload();
          }}
          onClose={() => {
            setGranting(false);
          }}
        />
      )}
      {asking !== null && (
        <RequestDialog
          grant={asking}
          onRequest={() => void actions.request(asking)}
          onClose={() => {
            setAsking(null);
          }}
        />
      )}
      {removing !== null && (
        <RemoveGrantDialog
          grant={removing}
          onRemove={() => void actions.revoke(removing)}
          onClose={() => {
            setRemoving(null);
          }}
        />
      )}
      {verifying !== null && (
        <VerifyContactDialog
          email={verifying}
          onVerified={actions.forgetUnverified}
          onClose={() => {
            setVerifying(null);
          }}
        />
      )}
    </SettingsSection>
  );
}
