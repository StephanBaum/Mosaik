/**
 * The browser extension's prompts and pairings as the events report them: one pairing request
 * and one save request at a time (Rust keeps one slot of each), a counter that changes
 * whenever the paired browsers do, and the clean-up when the vault locks.
 */
import { useState } from "react";
import type {
  AppEvent,
  BrowserPairingRequest,
  BrowserSaveRequest,
} from "../ipc/contract/index.ts";

export interface BrowserPrompts {
  pairing: BrowserPairingRequest | null;
  removePairing: (requestId: string) => void;
  save: BrowserSaveRequest | null;
  removeSave: (requestId: string) => void;
  /** Changes whenever a browser was paired, removed, connected or disconnected. */
  version: number;
  /** Applies a browser event; `false` for any other event. */
  apply: (event: AppEvent) => boolean;
  /** Drops both prompts (the vault locked). */
  clear: () => void;
}

export function useBrowserPrompts(): BrowserPrompts {
  const [pairing, setPairing] = useState<BrowserPairingRequest | null>(null);
  const [save, setSave] = useState<BrowserSaveRequest | null>(null);
  const [version, setVersion] = useState(0);

  const removePairing = (requestId: string) => {
    setPairing((open) => (open?.requestId === requestId ? null : open));
  };
  const removeSave = (requestId: string) => {
    setSave((open) => (open?.requestId === requestId ? null : open));
  };

  function apply(event: AppEvent): boolean {
    switch (event.type) {
      case "browserPairingRequested":
        setPairing(event.request);
        return true;
      case "browserPairingClosed":
        removePairing(event.requestId);
        return true;
      case "browserSaveRequested":
        setSave(event.request);
        return true;
      case "browserSaveClosed":
        removeSave(event.requestId);
        return true;
      case "browserPairingsChanged":
        setVersion((v) => v + 1);
        return true;
      default:
        return false;
    }
  }

  return {
    pairing,
    removePairing,
    save,
    removeSave,
    version,
    apply,
    clear: () => {
      setPairing(null);
      setSave(null);
    },
  };
}
