/** Hooks of the password and key screens (M7): the rotation journal and the sync account's second factor. */
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";
import { useIpcEvent } from "../../../hooks/useIpcEvent.ts";
import type { RotationStatus, SyncOverview } from "../../../ipc/contract/index.ts";
import { useIpc } from "../../../ipc/IpcProvider.tsx";

/** The id of the quick unlock heading: "Set up quick unlock again" moves focus there. */
export const QUICK_UNLOCK_HEADING_ID = "settings-quick-unlock-heading";

/** Moves focus to the quick unlock section (after the wizard dialog closed). */
export function focusQuickUnlock(): void {
  setTimeout(() => {
    const heading = document.getElementById(QUICK_UNLOCK_HEADING_ID);
    if (heading !== null) {
      heading.tabIndex = -1;
      heading.focus();
      heading.scrollIntoView({ block: "start" });
    }
  }, 0);
}

/** `rotation_status`, again on every `rotationProgress` event and when `refresh` is called. */
export function useRotationStatus(): { status: RotationStatus | null; refresh: () => void } {
  const ipc = useIpc();
  const [status, setStatus] = useState<RotationStatus | null>(null);
  const [version, setVersion] = useState(0);
  useEffect(() => {
    let current = true;
    void ipc.call("rotation_status", null).then((result) => {
      if (current && result.ok) {
        setStatus(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, version]);
  useIpcEvent((event) => {
    if (event.type === "rotationProgress") {
      setVersion((n) => n + 1);
    }
  });
  const refresh = useCallback(() => {
    setVersion((n) => n + 1);
  }, []);
  return { status, refresh };
}

/** The sync overview, read once (the backend decides what the wizards can do). */
export function useBackendOverview(): SyncOverview | null {
  const ipc = useIpc();
  const [overview, setOverview] = useState<SyncOverview | null>(null);
  useEffect(() => {
    let current = true;
    void ipc.call("sync_status", null).then((result) => {
      if (current && result.ok) {
        setOverview(result.value);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  return overview;
}

/** Whether a change needs a second-factor code: a hosted sync account with two-step verification. */
/** Moves focus to the page's main region: for a notice that went away with the control that had focus. */
export function focusMain(): void {
  const main = document.getElementById("main");
  if (main !== null) {
    if (!main.hasAttribute("tabindex")) {
      main.tabIndex = -1;
    }
    main.focus();
  }
}

/** Whether focus fell to the page (the control that had it was removed). */
export function focusLost(): boolean {
  return document.activeElement === null || document.activeElement === document.body;
}

export function needsSecondFactor(overview: SyncOverview | null): boolean {
  return overview?.config?.kind === "hosted" && overview.config.totpEnabled;
}

// ---- which wizard is open --------------------------------------------------------------------

let openWizards = 0;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function setOpen(delta: number): void {
  openWizards += delta;
  for (const listener of listeners) {
    listener();
  }
}

/** Marks a wizard dialog as open for as long as the component lives. */
export function useWizardOpenMarker(): void {
  useEffect(() => {
    setOpen(1);
    return () => {
      setOpen(-1);
    };
  }, []);
}

/** Whether any wizard dialog is open (the "Continue key rotation" banner steps aside). */
export function useAnyWizardOpen(): boolean {
  return useSyncExternalStore(
    subscribe,
    () => openWizards > 0,
    () => false,
  );
}
