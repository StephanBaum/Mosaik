/**
 * The breach scan as the card shows it: what is running, what the last scan said, and the
 * consent flow. A scan runs in Rust; the webview only starts and stops it and listens to the
 * events (`breachScanProgress`, `breachScanFinished`).
 */
import { useState } from "react";
import { useSettings } from "../../app/SettingsProvider.tsx";
import { useIpcEvent } from "../../hooks/useIpcEvent.ts";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { BreachMode, BreachScanError, BreachStatus, IpcError } from "../../ipc/contract/index.ts";

export interface ScanSummary {
  checked: number;
  found: number;
  error: BreachScanError | null;
}

export interface BreachScan {
  running: boolean;
  progress: { done: number; total: number } | null;
  /** The scan that ended in this session (the status holds the older one). */
  finished: ScanSummary | null;
  error: IpcError | null;
  /** The mode the user picked while the consent dialog is open; `null`: no dialog. */
  consenting: BreachMode | null;
  /** "Check now". */
  checkNow: () => void;
  stop: () => void;
  /** The mode select: off and a mode that was agreed to apply at once, else the consent dialog opens. */
  setMode: (mode: BreachMode) => void;
  agree: () => void;
  decline: () => void;
}

/** What the scan events say: running, the step, and how the last scan of this session ended. */
function useScanEvents(status: BreachStatus) {
  const [running, setRunning] = useState(status.running);
  const [progress, setProgress] = useState<BreachScan["progress"]>(null);
  const [finished, setFinished] = useState<ScanSummary | null>(null);
  const [seen, setSeen] = useState(status.running);

  // The status is read from Rust: a scan that was already running (another start, a reload) shows too.
  if (status.running !== seen) {
    setSeen(status.running);
    setRunning(status.running);
  }

  useIpcEvent((event) => {
    if (event.type === "breachScanProgress") {
      setRunning(true);
      setProgress({ done: event.done, total: event.total });
    } else if (event.type === "breachScanFinished") {
      setRunning(false);
      setProgress(null);
      setFinished({ checked: event.checked, found: event.found, error: event.error });
    }
  });
  return { running, setRunning, progress, finished, setFinished };
}

export function useBreachScan(status: BreachStatus, onChanged: () => void): BreachScan {
  const ipc = useIpc();
  const { settings, update } = useSettings();
  const { running, setRunning, progress, finished, setFinished } = useScanEvents(status);
  const [error, setError] = useState<IpcError | null>(null);
  const [consenting, setConsenting] = useState<BreachMode | null>(null);

  async function start() {
    setError(null);
    setFinished(null);
    setRunning(true);
    const result = await ipc.call("breach_scan_start", null);
    if (!result.ok) {
      setRunning(false);
      setError(result.error);
    }
  }

  async function save(mode: BreachMode, consentAt: number | null): Promise<boolean> {
    if (settings === null) {
      return false;
    }
    const failure = await update({ ...settings, breachCheck: mode, breachConsentAt: consentAt }, null);
    setError(failure);
    onChanged();
    return failure === null;
  }

  const consented = status.mode !== "off" && status.consentAt !== null;
  return {
    running,
    progress,
    finished,
    error,
    consenting,
    checkNow: () => {
      if (consented) {
        void start();
      } else {
        setConsenting("manual");
      }
    },
    stop: () => {
      void ipc.call("breach_scan_cancel", null).then((result) => {
        if (!result.ok) {
          setError(result.error);
        }
      });
    },
    setMode: (mode) => {
      if (mode === "off") {
        void save("off", null);
      } else if (consented) {
        void save(mode, status.consentAt);
      } else {
        setConsenting(mode);
      }
    },
    agree: () => {
      const mode = consenting ?? "manual";
      setConsenting(null);
      void save(mode, Date.now()).then((saved) => {
        if (saved) {
          void start();
        }
      });
    },
    decline: () => {
      setConsenting(null);
    },
  };
}
