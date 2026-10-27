/**
 * The IPC client the UI talks to. Two implementations: `tauri.ts` (the app) and
 * `mock.ts` (browser development and tests); `index.ts` picks one by build mode.
 */
import type { AppEvent, CommandName, IpcResult, RequestOf, ResponseOf } from "./contract/index.ts";

export type EventListener = (event: AppEvent) => void;

export interface MosaicIpc {
  /** Runs a command. Never throws: failures come back as `{ ok: false, error }`. */
  call<K extends CommandName>(command: K, request: RequestOf<K>): Promise<IpcResult<ResponseOf<K>>>;
  /** Listens for events from Rust; resolves to an unsubscribe function. */
  subscribe(listener: EventListener): Promise<() => void>;
}
