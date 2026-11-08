/** The sharing dialog of the open vault: the top bar's "Share…" button and the dialog it opens. */
import { useState, type ReactNode } from "react";
import type { VaultId, VaultSummary } from "../../ipc/contract/index.ts";
import { VaultSharingDialog } from "./VaultSharingDialog.tsx";

export function useVaultSharing(
  vaults: readonly VaultSummary[],
  vaultId: VaultId | null,
  listing: boolean,
  onChanged: () => void,
): { open: (() => void) | undefined; dialog: ReactNode } {
  const [shown, setShown] = useState<VaultSummary | null>(null);
  // Sharing is offered for the one account-key vault that is open.
  const shareable = listing
    ? vaults.find((v) => v.vaultId === vaultId && v.protection === "accountKey" && v.available)
    : undefined;
  return {
    open:
      shareable === undefined
        ? undefined
        : () => {
            setShown(shareable);
          },
    dialog:
      shown === null ? null : (
        <VaultSharingDialog
          vault={shown}
          onChanged={onChanged}
          onClose={() => {
            setShown(null);
          }}
        />
      ),
  };
}
