/** The paired browsers, read again whenever Rust reports a change (`browserPairingsChanged`). */
import { useEffect, useState } from "react";
import { useEvents } from "../app/EventsProvider.tsx";
import type { BrowserStatus } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";

export function useBrowserStatus() {
  const ipc = useIpc();
  const { browser } = useEvents();
  const { version } = browser;
  const [status, setStatus] = useState<BrowserStatus | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("browser_status", null).then((result) => {
      if (current && result.ok) {
        setStatus(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, version]);
  return [status, setStatus] as const;
}
