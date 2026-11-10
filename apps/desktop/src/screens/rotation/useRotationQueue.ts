/** The rotate-now queue (`rotation_queue`): loaded when the page opens and again after every change. */
import { useCallback, useEffect, useState } from "react";
import { useIpcEvent } from "../../hooks/useIpcEvent.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { IpcError, RotationQueue } from "../../ipc/contract/index.ts";

export interface QueueState {
  queue: RotationQueue | null;
  error: IpcError | null;
  /** Reads the queue again. */
  reload: () => void;
}

/** The entries to count and badge: not snoozed. */
export function dueCount(queue: RotationQueue | null): number {
  return queue === null ? 0 : queue.entries.filter((entry) => entry.snoozedUntil === null).length;
}

export function useRotationQueue(): QueueState {
  const ipc = useIpc();
  const [queue, setQueue] = useState<RotationQueue | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [round, setRound] = useState(0);

  useEffect(() => {
    let current = true;
    void ipc.call("rotation_queue", null).then((result) => {
      if (!current) {
        return;
      }
      setError(result.ok ? null : result.error);
      if (result.ok) {
        setQueue(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, round]);

  const reload = useCallback(() => {
    setRound((n) => n + 1);
  }, []);
  // A finished scan changes the findings; a running one changes `breach.running`.
  useIpcEvent((event) => {
    if (event.type === "breachScanFinished" || event.type === "syncDataChanged") {
      reload();
    }
  });
  return { queue, error, reload };
}
