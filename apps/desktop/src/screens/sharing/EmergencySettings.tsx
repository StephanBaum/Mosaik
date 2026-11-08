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
import type { EmergencyOverview, GrantedView, IpcError } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate, useNow } from "../sync/hooks.ts";
import { SettingsSection } from "../SettingsSection.tsx";
import { GrantedSection, ReceivedSection } from "./EmergencyRows.tsx";
import { useEmergencyActions } from "./useEmergencyActions.ts";
import { GrantDialog } from "./GrantDialog.tsx";
import { useBackendKind, type BackendKind } from "./hooks.ts";
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

export function EmergencySettings() {
  const { t } = useI18n();
  const toast = useToast();
  const now = useNow();
  const { kind, totp } = useBackendKind();
  const { overview, error, setError, reload } = useOverview(kind);
  const [granting, setGranting] = useState(false);
  const [removing, setRemoving] = useState<GrantedView | null>(null);
  const [verifying, setVerifying] = useState<string | null>(null);
  const actions = useEmergencyActions(reload, setError);

  return (
    <SettingsSection title={t("emergency.title")} body={t("emergency.body")}>
      {kind === "folder" && <p className="body muted">{t("emergency.folderUnsupported")}</p>}
      {kind === "none" && <p className="body muted">{t("sharing.notHosted.none")}</p>}
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
            onRequest={(grant) => void actions.request(grant)}
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
