/**
 * The one subscription to Rust events, fanned out to the app: lock (hide secrets, reload
 * the session), clipboard cleared (toast), SSH approvals (a queue) and the quick-search
 * shortcut (opens the palette).
 */
import { createContext, useContext, useMemo, useState, type ReactNode } from "react";
import { useToast } from "../components/ToastProvider.tsx";
import { useIpcEvent } from "../hooks/useIpcEvent.ts";
import { useI18n } from "../i18n/I18nProvider.tsx";
import type { AppEvent, LockReason, SshApprovalRequest } from "../ipc/contract/index.ts";
import { useSecrets } from "./SecretsContext.tsx";

interface EventsContextValue {
  /** Open SSH approval requests, oldest first. */
  ssh: readonly SshApprovalRequest[];
  removeSsh: (requestId: string) => void;
  paletteOpen: boolean;
  setPaletteOpen: (open: boolean) => void;
  /** Why the vault locked last, until the next unlock. */
  lockReason: LockReason | null;
  clearLockReason: () => void;
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
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [lockReason, setLockReason] = useState<LockReason | null>(null);

  useIpcEvent((event: AppEvent) => {
    switch (event.type) {
      case "locked":
        secrets.clearAll();
        setSsh([]);
        setPaletteOpen(false);
        setLockReason(event.reason);
        onLocked();
        break;
      case "clipboardCleared":
        toast(t("toast.clipboardCleared"));
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
        break;
    }
  });

  const value = useMemo<EventsContextValue>(
    () => ({
      ssh,
      removeSsh: (requestId) => {
        setSsh((queue) => queue.filter((request) => request.requestId !== requestId));
      },
      paletteOpen,
      setPaletteOpen,
      lockReason,
      clearLockReason: () => {
        setLockReason(null);
      },
    }),
    [ssh, paletteOpen, lockReason],
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
