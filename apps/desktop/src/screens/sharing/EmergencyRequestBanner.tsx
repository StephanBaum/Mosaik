/**
 * A grantor's banner: someone asked for emergency access to this account's vaults and the wait
 * is running. There is no email, so this is how the grantor finds out: it stays at the top until
 * the request is refused, approved or released. "Refuse" needs no password; "Approve now" is a
 * fresh login with the master password.
 */
import { useEffect, useState } from "react";
import { useEvents } from "../../app/EventsProvider.tsx";
import { Button } from "../../components/Button.tsx";
import { Callout } from "../../components/Callout.tsx";
import { ErrorAlert } from "../../components/ErrorAlert.tsx";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { GrantedView, IpcError } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";
import { PasswordDialog } from "../sync/PasswordDialog.tsx";
import { useBackendKind, useFormatWhen } from "./hooks.ts";

/** The requests that wait for the grantor's answer, read again on `sharingChanged`. */
function useWaiting(): { waiting: GrantedView[]; reload: () => void } {
  const ipc = useIpc();
  const { sharing } = useEvents();
  const [loaded, setLoaded] = useState<GrantedView[]>([]);
  const [local, setLocal] = useState(0);
  useEffect(() => {
    const live = { current: true };
    void ipc.call("emergency_overview", null).then((result) => {
      if (live.current && result.ok) {
        setLoaded(result.value.granted.filter((g) => g.state === "waiting"));
      }
    });
    return () => {
      live.current = false;
    };
  }, [ipc, sharing.emergencyRequests, sharing.version, local]);
  // Nothing waits as far as the last event knows: no banner, whatever was loaded before.
  const waiting = sharing.emergencyRequests === 0 && local === 0 ? [] : loaded;
  return {
    waiting,
    reload: () => {
      setLocal((n) => n + 1);
    },
  };
}

function ApproveDialog({
  grant,
  totp,
  onDone,
  onClose,
}: {
  grant: GrantedView;
  totp: boolean;
  onDone: () => void;
  onClose: () => void;
}) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  return (
    <PasswordDialog
      title={t("emergency.banner.approve.title")}
      body={t("emergency.banner.approve.body", { email: isolate(grant.email) })}
      withCode={totp}
      submitLabel={t("emergency.banner.approve.confirm")}
      submitVariant="dangerSolid"
      onSubmit={async (password, code) => {
        const result = await ipc.call("emergency_respond", {
          granteeAccountId: grant.granteeAccountId,
          decision: "approve",
          password,
          code: code === "" ? null : code,
        });
        if (result.ok) {
          toast(t("emergency.banner.approved"));
          onDone();
          return null;
        }
        return result.error;
      }}
      onClose={onClose}
    />
  );
}

export function EmergencyRequestBanner() {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const formatWhen = useFormatWhen();
  const { kind, totp } = useBackendKind();
  const { waiting, reload } = useWaiting();
  const [approving, setApproving] = useState<GrantedView | null>(null);
  const [error, setError] = useState<IpcError | null>(null);

  if (kind !== "hosted" || waiting.length === 0) {
    return null;
  }

  async function refuse(grant: GrantedView) {
    setError(null);
    const result = await ipc.call("emergency_respond", {
      granteeAccountId: grant.granteeAccountId,
      decision: "decline",
      password: null,
      code: null,
    });
    if (result.ok) {
      toast(t("emergency.banner.refused"));
      reload();
    } else {
      setError(result.error);
    }
  }

  return (
    <div className="emergency-banner stack" role="region" aria-label={t("emergency.banner.region")}>
      {waiting.map((grant) => (
        <Callout key={grant.granteeAccountId} tone="warning">
          <p className="body">
            {t("emergency.banner.body", {
              email: isolate(grant.email),
              when: formatWhen(grant.releaseAt ?? 0),
            })}
          </p>
          <div className="actions">
            <Button
              size="sm"
              onClick={() => {
                void refuse(grant);
              }}
            >
              {t("emergency.banner.refuse")}
            </Button>
            <Button
              size="sm"
              variant="danger"
              onClick={() => {
                setApproving(grant);
              }}
            >
              {t("emergency.banner.approve")}
            </Button>
          </div>
        </Callout>
      ))}
      <ErrorAlert error={error} />
      {approving !== null && (
        <ApproveDialog
          grant={approving}
          totp={totp}
          onDone={reload}
          onClose={() => {
            setApproving(null);
          }}
        />
      )}
    </div>
  );
}
