/** What the members panel reads and the changes it makes (members of one vault, its pending invites). */
import { useCallback, useEffect, useState } from "react";
import type {
  IpcError,
  IpcResult,
  OutgoingInviteView,
  VaultId,
  VaultMembersView,
} from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";

export interface LoadedMembers {
  view: VaultMembersView;
  /** The invites this account sent for the vault that still wait. */
  invites: OutgoingInviteView[];
}

/** The members of a vault and the invites that wait, read again whenever `version` changes. */
export function useMembersData(vaultId: VaultId, version: number) {
  const ipc = useIpc();
  const [loaded, setLoaded] = useState<LoadedMembers | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [local, setLocal] = useState(0);
  useEffect(() => {
    const live = { current: true };
    void (async () => {
      const members = await ipc.call("vault_members", { vaultId });
      const invites = members.ok ? await ipc.call("invites_list", null) : null;
      if (!live.current) {
        return;
      }
      if (members.ok) {
        setError(null);
        setLoaded({
          view: members.value,
          invites: invites?.ok ? invites.value.outgoing.filter((i) => i.vaultId === vaultId) : [],
        });
      } else {
        setError(members.error);
      }
    })();
    return () => {
      live.current = false;
    };
  }, [ipc, vaultId, version, local]);
  const reload = useCallback(() => {
    setLocal((n) => n + 1);
  }, []);
  return { loaded, error, reload };
}

/**
 * Runs one change at a time. A native question the user answers "no" to (`cancelled`) is no
 * error. Resolves to whether the change went through.
 */
export function useChange(onDone: () => void) {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<IpcError | null>(null);
  const run = async <T>(change: () => Promise<IpcResult<T>>): Promise<boolean> => {
    setBusy(true);
    setError(null);
    const result = await change();
    setBusy(false);
    if (result.ok) {
      onDone();
      return true;
    }
    if (result.error.code !== "cancelled") {
      setError(result.error);
    }
    return false;
  };
  return { busy, error, run };
}
