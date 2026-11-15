/**
 * The agent endpoint's prompts as the events report them: one pairing request and one access
 * request at a time (Rust shows one of each and queues the rest), the number of requests being
 * sent (the in-use indicator), a counter that changes whenever pairings, grants or the endpoint
 * do, and the clean-up when the vault locks. An access request with the same ID arrives again
 * when the number of waiting requests changes: it replaces the open one without restarting the
 * dialog (the dialog is keyed by the ID), while a new ID is a new dialog with its own guard.
 */
import { useState } from "react";
import type { AgentAccessRequest, AgentPairingRequest, AppEvent } from "../ipc/contract/index.ts";

export interface AgentPrompts {
  pairing: AgentPairingRequest | null;
  removePairing: (requestId: string) => void;
  access: AgentAccessRequest | null;
  removeAccess: (requestId: string) => void;
  /** Requests being sent now. */
  inFlight: number;
  /** Changes whenever agent pairings, grants or the endpoint did. */
  version: number;
  /** Applies an agent event; `false` for any other event. */
  apply: (event: AppEvent) => boolean;
  /** Drops every prompt (the vault locked). */
  clear: () => void;
}

export function useAgentPrompts(): AgentPrompts {
  const [pairing, setPairing] = useState<AgentPairingRequest | null>(null);
  const [access, setAccess] = useState<AgentAccessRequest | null>(null);
  const [inFlight, setInFlight] = useState(0);
  const [version, setVersion] = useState(0);

  const removePairing = (requestId: string) => {
    setPairing((open) => (open?.requestId === requestId ? null : open));
  };
  const removeAccess = (requestId: string) => {
    setAccess((open) => (open?.requestId === requestId ? null : open));
  };

  function apply(event: AppEvent): boolean {
    switch (event.type) {
      case "agentPairingRequested":
        setPairing(event.request);
        return true;
      case "agentPairingClosed":
        removePairing(event.requestId);
        return true;
      case "agentAccessRequested":
        setAccess(event.request);
        return true;
      case "agentAccessClosed":
        removeAccess(event.requestId);
        return true;
      case "agentActivity":
        setInFlight(event.inFlight);
        return true;
      case "agentStateChanged":
        setVersion((v) => v + 1);
        return true;
      default:
        return false;
    }
  }

  return {
    pairing,
    removePairing,
    access,
    removeAccess,
    inFlight,
    version,
    apply,
    clear: () => {
      setPairing(null);
      setAccess(null);
      setInFlight(0);
    },
  };
}
