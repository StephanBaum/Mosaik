/**
 * The popup's side of the port to the background: requests by number, the replies, and the
 * events the background pushes (the link to the desktop changed, a pairing code arrived).
 * The port is a parameter, so tests drive it with a fake.
 */
import { ExtensionError, isErrorCode } from "../shared/errors.ts";
import type { PopupEvent, PopupRequest, PopupResults } from "../shared/messages.ts";

/** The part of `chrome.runtime.Port` the popup uses. */
export interface PortLike {
  postMessage: (message: unknown) => void;
  onMessage: { addListener: (listener: (message: unknown) => void) => void };
  onDisconnect: { addListener: (listener: () => void) => void };
}

export interface PopupApi {
  request<R extends PopupRequest>(request: R): Promise<PopupResults[R["op"]]>;
  /** Listens to pushed events; returns the function that stops listening. */
  subscribe(listener: (event: PopupEvent) => void): () => void;
}

interface Waiting {
  resolve: (result: never) => void;
  reject: (error: ExtensionError) => void;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null;

function asEvent(message: Record<string, unknown>): PopupEvent | null {
  if (message["event"] === "connection" && typeof message["connection"] === "string") {
    return message as unknown as PopupEvent;
  }
  if (
    message["event"] === "pairCode" &&
    typeof message["code"] === "string" &&
    typeof message["expiresInSeconds"] === "number"
  ) {
    return message as unknown as PopupEvent;
  }
  return null;
}

export function createPopupClient(port: PortLike): PopupApi {
  const waiting = new Map<number, Waiting>();
  const listeners = new Set<(event: PopupEvent) => void>();
  let nextId = 1;
  let closed = false;

  const settle = (message: Record<string, unknown>): void => {
    const id = message["id"];
    const entry = typeof id === "number" ? waiting.get(id) : undefined;
    if (typeof id !== "number" || entry === undefined) {
      return;
    }
    waiting.delete(id);
    if (message["ok"] === true) {
      entry.resolve(message["result"] as never);
    } else {
      const code = message["error"];
      entry.reject(new ExtensionError(isErrorCode(code) ? code : "internal"));
    }
  };

  port.onMessage.addListener((message) => {
    if (!isRecord(message)) {
      return;
    }
    const event = asEvent(message);
    if (event === null) {
      settle(message);
      return;
    }
    for (const listener of listeners) {
      listener(event);
    }
  });
  port.onDisconnect.addListener(() => {
    closed = true;
    for (const entry of waiting.values()) {
      entry.reject(new ExtensionError("hostExited"));
    }
    waiting.clear();
  });

  return {
    request: <R extends PopupRequest>(request: R): Promise<PopupResults[R["op"]]> => {
      if (closed) {
        return Promise.reject(new ExtensionError("hostExited"));
      }
      const id = nextId++;
      return new Promise((resolve, reject) => {
        waiting.set(id, { resolve, reject });
        port.postMessage({ id, request });
      });
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}
