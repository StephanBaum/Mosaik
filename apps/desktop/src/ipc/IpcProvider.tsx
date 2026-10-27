/** Makes the IPC client available to every component. */
import { createContext, useContext, type ReactNode } from "react";
import type { MosaicIpc } from "./client.ts";

const IpcContext = createContext<MosaicIpc | null>(null);

export function IpcProvider({ ipc, children }: { ipc: MosaicIpc; children: ReactNode }) {
  return <IpcContext value={ipc}>{children}</IpcContext>;
}

export function useIpc(): MosaicIpc {
  const ipc = useContext(IpcContext);
  if (ipc === null) {
    throw new Error("useIpc needs an IpcProvider");
  }
  return ipc;
}
