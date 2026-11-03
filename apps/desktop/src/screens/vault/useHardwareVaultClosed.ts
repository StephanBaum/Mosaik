/** A hardware vault closed itself (timer or lock): drop revealed codes and show the panel. */
import { useSecrets } from "../../app/SecretsContext.tsx";
import { useIpcEvent } from "../../hooks/useIpcEvent.ts";
import type { Shell } from "./useShell.ts";

export function useHardwareVaultClosed(shell: Shell): void {
  const secrets = useSecrets();
  useIpcEvent((event) => {
    if (event.type === "hardwareVaultClosed") {
      secrets.clearAll();
      if (shell.selected?.vaultId === event.vaultId) {
        shell.setSelected(null);
      }
      shell.changed();
    }
  });
}
