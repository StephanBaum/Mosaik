import { useEffect } from "react";
import type { AppEvent } from "../ipc/contract/index.ts";
import { useIpc } from "../ipc/IpcProvider.tsx";
import { useLatest } from "./useLatest.ts";

/** Subscribes to Rust events for the lifetime of the component. */
export function useIpcEvent(handler: (e: AppEvent) => void): void {
  const ipc = useIpc();
  const latest = useLatest(handler);
  useEffect(() => {
    let cancelled = false;
    let unsubscribe: (() => void) | undefined;
    void ipc
      .subscribe((event) => {
        latest.current(event);
      })
      .then((stop) => {
        if (cancelled) {
          stop();
        } else {
          unsubscribe = stop;
        }
      });
    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, [ipc, latest]);
}
