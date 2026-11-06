/** The vaults a browser request can write to: open ones that are not hardware-protected. */
import { useEffect, useState } from "react";
import type { VaultSummary } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";

export function useWritableVaults(): VaultSummary[] {
  const ipc = useIpc();
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  useEffect(() => {
    void ipc.call("list_vaults", null).then((result) => {
      if (result.ok) {
        setVaults(result.value.filter((v) => v.available && v.protection === "accountKey"));
      }
    });
  }, [ipc]);
  return vaults;
}
