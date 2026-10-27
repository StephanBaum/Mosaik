/**
 * Picks the IPC implementation by build mode: `vite --mode mock` uses the in-memory mock,
 * every other mode talks to Tauri. Non-mock builds replace `mock.ts` with a stub
 * (vite.config.ts), so the mock never ships.
 */
import type { MosaicIpc } from "./client.ts";

export type { EventListener, MosaicIpc } from "./client.ts";

export async function loadIpc(): Promise<MosaicIpc> {
  if (import.meta.env.MODE === "mock") {
    const { createMockIpc, mockOptionsFromUrl } = await import("./mock.ts");
    return createMockIpc(mockOptionsFromUrl(window.location.search));
  }
  const { createTauriIpc } = await import("./tauri.ts");
  return createTauriIpc();
}
