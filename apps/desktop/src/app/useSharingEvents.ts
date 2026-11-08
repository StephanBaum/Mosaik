/**
 * What the sharing events report: how many invites and emergency requests wait for this
 * account, and a counter that changes whenever Rust sends `sharingChanged` (the inbox and the
 * request banner read again on it). The counts reset when the vault locks.
 */
import { useState } from "react";
import { useIpcEvent } from "../hooks/useIpcEvent.ts";

export interface SharingEvents {
  /** Invites that wait for this account. */
  invites: number;
  /** Emergency-access requests that wait for this account's answer. */
  emergencyRequests: number;
  /** Changes whenever the invites or the emergency requests changed. */
  version: number;
}

const NONE: SharingEvents = { invites: 0, emergencyRequests: 0, version: 0 };

export function useSharingEvents(): SharingEvents {
  const [state, setState] = useState(NONE);
  useIpcEvent((event) => {
    if (event.type === "sharingChanged") {
      setState((old) => ({
        invites: event.invites,
        emergencyRequests: event.emergencyRequests,
        version: old.version + 1,
      }));
    } else if (event.type === "locked") {
      setState((old) => ({ ...NONE, version: old.version + 1 }));
    }
  });
  return state;
}
