/** Whether the user switched agent access on (`agent_access_status`), read when the screen opens. */
import { useEffect, useState } from "react";
import { useIpc } from "../ipc/IpcProvider.tsx";

export function useAgentEnabled(): boolean {
  const ipc = useIpc();
  const [enabled, setEnabled] = useState(false);
  useEffect(() => {
    let current = true;
    void ipc.call("agent_access_status", null).then((result) => {
      if (current && result.ok) {
        setEnabled(result.value.enabled);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  return enabled;
}
