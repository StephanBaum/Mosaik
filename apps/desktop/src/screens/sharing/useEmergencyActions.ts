/** What the emergency settings do: one call at a time, a toast when it worked, a refresh after. */
import { useState } from "react";
import { useToast } from "../../components/ToastProvider.tsx";
import { useI18n } from "../../i18n/I18nProvider.tsx";
import type { GrantedView, IpcError, IpcResult, ReceivedView } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { isolate } from "../sync/hooks.ts";

export function useEmergencyActions(reload: () => void, setError: (error: IpcError | null) => void) {
  const { t } = useI18n();
  const ipc = useIpc();
  const toast = useToast();
  const [busy, setBusy] = useState(false);
  /** The grantors whose claim needs their safety code compared first. */
  const [unverified, setUnverified] = useState<ReadonlySet<string>>(new Set());

  /**
   * Runs `call`; `done` words the toast. A claim that fails with `sharingContactUnverified`
   * offers the comparison (for `grantor`) instead of an error.
   */
  async function act<T>(call: () => Promise<IpcResult<T>>, done: (value: T) => string, grantor?: string) {
    setBusy(true);
    setError(null);
    const result = await call();
    setBusy(false);
    if (result.ok) {
      toast(done(result.value));
      reload();
    } else if (grantor !== undefined && result.error.code === "sharingContactUnverified") {
      setUnverified((set) => new Set(set).add(grantor));
    } else {
      setError(result.error);
    }
  }

  return {
    busy,
    unverified,
    forgetUnverified: () => {
      setUnverified(new Set());
    },
    request: (grant: ReceivedView) =>
      act(
        () => ipc.call("emergency_request", { grantorAccountId: grant.grantorAccountId }),
        () => t("emergency.received.requested", { email: isolate(grant.email) }),
      ),
    claim: (grant: ReceivedView) =>
      act(
        () => ipc.call("emergency_claim", { grantorAccountId: grant.grantorAccountId }),
        (claimed) => t("emergency.received.claimed", { count: claimed.vaults }),
        grant.grantorAccountId,
      ),
    revoke: (grant: GrantedView) =>
      act(
        () => ipc.call("emergency_revoke", { accountId: grant.granteeAccountId }),
        () => t("emergency.granted.removed"),
      ),
  };
}
