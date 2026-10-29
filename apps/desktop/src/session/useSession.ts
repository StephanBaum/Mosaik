/**
 * The session state machine as the UI sees it: loading, then the Rust session state
 * (no account → locked ⇄ unlocked). `reload` is called when a `locked` event arrives
 * (`app/EventsProvider.tsx` owns the one event subscription).
 */
import { useCallback, useEffect, useState } from "react";
import type { MosaicIpc } from "../ipc/client.ts";
import type { IpcError, SessionState } from "../ipc/contract/index.ts";

export type SessionView =
  | { status: "loading" }
  | { status: "ready"; session: SessionState }
  | { status: "failed"; error: IpcError };

export function useSession(ipc: MosaicIpc): {
  view: SessionView;
  setSession: (session: SessionState) => void;
  reload: () => void;
} {
  const [view, setView] = useState<SessionView>({ status: "loading" });

  const reload = useCallback(() => {
    void ipc.call("session_state", null).then((result) => {
      setView(
        result.ok
          ? { status: "ready", session: result.value }
          : { status: "failed", error: result.error },
      );
    });
  }, [ipc]);

  const setSession = useCallback((session: SessionState) => {
    setView({ status: "ready", session });
  }, []);

  useEffect(() => {
    reload();
  }, [reload]);

  return { view, setSession, reload };
}
