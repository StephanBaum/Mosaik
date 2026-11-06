/**
 * The passkey bridge (ADR-0040): the isolated-world half of the shim, at `document_start` in
 * the top frame of https pages. It is the only script of the extension that listens to page
 * messages, and it listens only to its own window (`event.source === window`) and answers only
 * messages of exactly the shim's shape and size. It forwards a request to the background with
 * the origin it reads from `window.origin` (a view of the real document the page cannot
 * redefine); the background compares that with what the browser says about the sender and takes
 * no value of the origin from here. It holds no state beyond the requests in flight.
 */
import { PAGE_MAX_IN_FLIGHT } from "../shared/passkey-limits.ts";
import {
  PASSKEY_ACK_TYPE,
  PASSKEY_RESPONSE_TYPE,
  isPasskeyPageResult,
  isShimAbort,
  isShimRequest,
  type PasskeyAbortMessage,
  type PasskeyMessage,
  type PasskeyPageResult,
  type ShimRequest,
} from "../shared/passkey-messages.ts";

export interface BridgeEnv {
  win: Window & typeof globalThis;
  /** `chrome.runtime.sendMessage`. */
  send: (message: PasskeyMessage | PasskeyAbortMessage) => Promise<unknown>;
}

const MARKER = "mosaicPasskeyBridge";
const FALLBACK: PasskeyPageResult = { kind: "fallback" };

function messageOf(request: ShimRequest, origin: string): PasskeyMessage {
  return request.op === "create"
    ? { kind: "passkey", op: "create", pageRequestId: request.id, origin, options: request.options }
    : { kind: "passkey", op: "get", pageRequestId: request.id, origin, options: request.options };
}

/** Starts listening; `false` when this world already has a bridge (the browser can inject twice). */
export function installBridge(env: BridgeEnv): boolean {
  const { win } = env;
  if (Object.hasOwn(win, MARKER)) {
    return false;
  }
  Object.defineProperty(win, MARKER, { value: true, enumerable: false });
  const inFlight = new Set<number>();

  const post = (message: { type: string; id: number; result?: PasskeyPageResult }): void => {
    win.postMessage(message, win.origin);
  };
  const reply = (id: number, result: PasskeyPageResult): void => {
    post({ type: PASSKEY_RESPONSE_TYPE, id, result });
  };

  /** A tab the user is not looking at gets no dialog; the desktop is asked only for a page in use. */
  const inUse = (): boolean =>
    win.origin.startsWith("https:") && win.parent === win && win.document.hasFocus() && win.document.visibilityState === "visible";

  const onRequest = (request: ShimRequest): void => {
    if (inFlight.size >= PAGE_MAX_IN_FLIGHT || inFlight.has(request.id) || !inUse()) {
      reply(request.id, FALLBACK);
      return;
    }
    inFlight.add(request.id);
    post({ type: PASSKEY_ACK_TYPE, id: request.id });
    let sent: Promise<unknown>;
    try {
      sent = env.send(messageOf(request, win.origin));
    } catch {
      sent = Promise.reject(new Error("send"));
    }
    void sent
      .then((answer) => (isPasskeyPageResult(answer) ? answer : FALLBACK))
      .catch(() => FALLBACK)
      .then((result) => {
        // An aborted request was dropped from the set: the page has its AbortError already.
        if (inFlight.delete(request.id)) {
          reply(request.id, result);
        }
      });
  };

  win.addEventListener("message", (event: MessageEvent) => {
    if (event.source !== win) {
      return;
    }
    const data: unknown = event.data;
    if (isShimRequest(data)) {
      onRequest(data);
    } else if (isShimAbort(data) && inFlight.delete(data.id)) {
      env.send({ kind: "passkeyAbort", pageRequestId: data.id }).catch(() => undefined);
    }
  });
  return true;
}
