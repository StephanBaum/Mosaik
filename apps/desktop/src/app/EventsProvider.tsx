/**
 * The one subscription to Rust events, fanned out to the app: lock (hide secrets, reload
 * the session), clipboard cleared and settings reset (toasts), SSH approvals (a queue) and the quick-search
 * shortcut (opens the palette). Sync: the last state, a counter that changes when a sync run
 * changed vaults (the vault screens reload on it) and a toast when it merged or repaired items.
 * Sharing: how many invites and emergency requests wait (`useSharingEvents`). The browser extension's prompts live in `useBrowserPrompts`.
 */
import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { useToast } from "../components/ToastProvider.tsx";
import { useIpcEvent } from "../hooks/useIpcEvent.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { AppEvent, LockReason, SshApprovalRequest, SyncState } from "../ipc/contract/index.ts";
import { useBrowserPrompts, type BrowserPrompts } from "./useBrowserPrompts.ts";
import { useSharingEvents, type SharingEvents } from "./useSharingEvents.ts";
import { useSecrets } from "./SecretsContext.tsx";

interface EventsContextValue {
  /** Open SSH approval requests, oldest first. */
  ssh: readonly SshApprovalRequest[];
  removeSsh: (requestId: string) => void;
  /** The browser extension's pairing, save and passkey prompts, and the paired-browsers counter. */
  browser: BrowserPrompts;
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
  /** Why the vault locked last, until the next unlock. */
  lockReason: LockReason | null;
  clearLockReason: () => void;
  /** The last sync state Rust reported; `null` until the first event. */
  sync: SyncState | null;
  /** Changes whenever a sync run changed vault data (`syncDataChanged`). */
  syncDataVersion: number;
  /** Invites and emergency-access requests that wait for this account. */
  sharing: SharingEvents;
}

const EventsContext = createContext<EventsContextValue | null>(null);

export function EventsProvider({
  onLocked,
  children,
}: {
  onLocked: () => void;
  children: ReactNode;
}) {
  const { t } = useI18n();
  const toast = useToast();
  const secrets = useSecrets();
  const [ssh, setSsh] = useState<SshApprovalRequest[]>([]);
  const browser = useBrowserPrompts();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [lockReason, setLockReason] = useState<LockReason | null>(null);
  const [sync, setSync] = useState<SyncState | null>(null);
  const [syncDataVersion, setSyncDataVersion] = useState(0);
  const sharing = useSharingEvents();

  useIpcEvent((event: AppEvent) => {
    if (browser.apply(event)) {
      return;
    }
    switch (event.type) {
      case "locked":
        secrets.clearAll();
        setSsh([]);
        browser.clear();
        setPaletteOpen(false);
        setLockReason(event.reason);
        onLocked();
        break;
      case "clipboardCleared":
        toast(t("toast.clipboardCleared"));
        break;
      case "settingsReset":
        toast(t("toast.settingsReset"));
        break;
      case "sshApprovalRequested":
        setSsh((queue) => [...queue, event.request]);
        break;
      case "sshApprovalClosed":
        setSsh((queue) => queue.filter((request) => request.requestId !== event.requestId));
        break;
      case "sshSignFailed":
        toast(t(`ssh.signFailed.${event.reason}`));
        break;
      case "quickSearchRequested":
        setPaletteOpen(true);
        break;
      case "syncStateChanged":
        setSync(event.state);
        break;
      case "syncDataChanged":
        setSyncDataVersion((n) => n + 1);
        break;
      case "syncAttention":
        toast(t("toast.syncAttention"));
        break;
    }
  });

  const value = useMemo<EventsContextValue>(
    () => ({
      ssh,
      browser,
      removeSsh: (requestId) => {
        setSsh((queue) => queue.filter((request) => request.requestId !== requestId));
      },
      paletteOpen,
      setPaletteOpen,
      lockReason,
      clearLockReason: () => {
        setLockReason(null);
      },
      sync,
      syncDataVersion,
      sharing,
    }),
    [ssh, browser, paletteOpen, lockReason, sync, syncDataVersion, sharing],
  );
  return <EventsContext value={value}>{children}</EventsContext>;
}

export function useEvents(): EventsContextValue {
  const context = useContext(EventsContext);
  if (context === null) {
    throw new Error("useEvents needs an EventsProvider");
  }
  return context;
}
