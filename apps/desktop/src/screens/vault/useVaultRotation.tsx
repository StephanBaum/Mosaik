/**
 * "Rotate vault key…" of the open vault (M7-R2): the vault menu entry and the dialog it opens.
 * Offered for the one account-key vault that is open, on a device with sync (a vault key is
 * offered to a backend), and only to the owner of a shared vault.
 */
import { useEffect, useState, type ReactNode } from "react";
import type { VaultId, VaultSummary } from "../../ipc/contract/index.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import { RotateVaultKeyDialog } from "./RotateVaultKeyDialog.tsx";

/** Whether this device may rotate `vaultId`: it has a backend, and is the owner when the vault is shared. */
function useMayRotate(vaultId: VaultId | null, enabled: boolean, reloadKey: number): boolean {
  const ipc = useIpc();
  const key = enabled && vaultId !== null ? `${vaultId}:${String(reloadKey)}` : null;
  const [answer, setAnswer] = useState<{ key: string; may: boolean } | null>(null);
  useEffect(() => {
    if (key === null || vaultId === null) {
      return undefined;
    }
    const live = { current: true };
    void (async () => {
      const sync = await ipc.call("sync_status", null);
      const hasBackend = sync.ok && sync.value.config !== null;
      // A folder backend has no member list: the vault is then the user's own.
      const members = hasBackend ? await ipc.call("vault_members", { vaultId }) : null;
      const may = hasBackend && (members === null || !members.ok || members.value.ownRole === "owner");
      if (live.current) {
        setAnswer({ key, may });
      }
    })();
    return () => {
      live.current = false;
    };
  }, [ipc, key, vaultId]);
  return key !== null && answer?.key === key && answer.may;
}

export function useVaultRotation(options: {
  vaults: readonly VaultSummary[];
  vaultId: VaultId | null;
  listing: boolean;
  reloadKey: number;
  onChanged: () => void;
}): { open: (() => void) | undefined; dialog: ReactNode } {
  const { vaults, vaultId, listing, reloadKey, onChanged } = options;
  const [shown, setShown] = useState<VaultSummary | null>(null);
  const rotatable = listing
    ? vaults.find((v) => v.vaultId === vaultId && v.protection === "accountKey" && v.available)
    : undefined;
  const may = useMayRotate(vaultId, rotatable !== undefined, reloadKey);
  return {
    open:
      rotatable === undefined || !may
        ? undefined
        : () => {
            setShown(rotatable);
          },
    dialog:
      shown === null ? null : (
        <RotateVaultKeyDialog
          vault={shown}
          onDone={onChanged}
          onClose={() => {
            setShown(null);
          }}
        />
      ),
  };
}
