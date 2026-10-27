/**
 * The real IPC: Tauri `invoke` for commands, one `Channel` for events (no `core:event`
 * permission needed, ADR-0019). Every command takes its request as the single `request`
 * argument, or no argument for `null` requests.
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import type { EventListener, MosaicIpc } from "./client.ts";
import {
  isErrorCode,
  type AppEvent,
  type CommandName,
  type IpcError,
  type IpcResult,
  type RequestOf,
  type ResponseOf,
} from "./contract/index.ts";

/** Rust rejects with `{ code }`; anything else (a Tauri or ACL error) is `internal`. */
export function toIpcError(error: unknown): IpcError {
  if (typeof error === "object" && error !== null && "code" in error && isErrorCode(error.code)) {
    return { code: error.code };
  }
  return { code: "internal" };
}

async function call<K extends CommandName>(
  command: K,
  request: RequestOf<K>,
): Promise<IpcResult<ResponseOf<K>>> {
  try {
    const value = await invoke<ResponseOf<K>>(command, request === null ? {} : { request });
    return { ok: true, value };
  } catch (error) {
    return { ok: false, error: toIpcError(error) };
  }
}

export function createTauriIpc(): MosaicIpc {
  const listeners = new Set<EventListener>();
  let subscription: Promise<void> | null = null;

  return {
    call,
    async subscribe(listener) {
      listeners.add(listener);
      subscription ??= (async () => {
        const channel = new Channel<AppEvent>();
        channel.onmessage = (event) => {
          for (const each of listeners) {
            each(event);
          }
        };
        await invoke("subscribe_events", { channel });
      })();
      await subscription;
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
