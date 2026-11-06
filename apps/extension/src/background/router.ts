/**
 * Who may talk to the background. Popup operations come only over a port named `popup` from
 * the extension's own popup page; the messages a content script may send are `captured` (the
 * save capture) and the two passkey messages of the bridge (ADR-0040). Anything else is
 * dropped without a reply, so a page or another extension learns nothing.
 * The worker listens to nothing from other extensions or web pages (no external-message listeners).
 */
import { errorCodeOf } from "../shared/errors.ts";
import {
  isCapturedMessage,
  type CapturedMessage,
  isPopupEnvelope,
  POPUP_PORT,
  type PopupEvent,
  type PopupReply,
  type PopupRequest,
} from "../shared/messages.ts";
import {
  isPasskeyAbortMessage,
  isPasskeyKind,
  isPasskeyMessage,
  type PasskeyAbortMessage,
  type PasskeyMessage,
  type PasskeyPageResult,
} from "../shared/passkey-messages.ts";
import { RateLimiter } from "./ratelimit.ts";

/** The part of `MessageSender` the router reads. */
export interface SenderInfo {
  id?: string | undefined;
  url?: string | undefined;
  /** The sending document's origin (browser-supplied; "null" when opaque). */
  origin?: string | undefined;
  tab?: { id?: number | undefined; url?: string | undefined } | undefined;
  frameId?: number | undefined;
  /** The sending document (Chromium); with the tab and frame it names one page load. */
  documentId?: string | undefined;
}

/** The part of `chrome.runtime.Port` the router uses. */
export interface PortInfo {
  name: string;
  sender?: SenderInfo | undefined;
  postMessage: (message: unknown) => void;
  disconnect: () => void;
  onMessage: { addListener: (listener: (message: unknown) => void) => void };
  onDisconnect: { addListener: (listener: () => void) => void };
}

export interface RouterDeps {
  extensionId: string;
  /** `chrome.runtime.getURL("popup.html")`. */
  popupUrl: string;
  /** E2E builds only: the popup page may carry a query string and live in a tab. */
  testPopup: boolean;
  now: () => number;
  handlePopup: (request: PopupRequest, sender: SenderInfo) => Promise<unknown>;
  /** The opt-in capture script's message; without a handler it is dropped. */
  onCaptured?: (message: CapturedMessage, sender: SenderInfo) => void;
  /** The bridge's passkey messages; without a handler they are dropped. */
  passkey?: {
    handle: (message: PasskeyMessage, sender: SenderInfo) => Promise<PasskeyPageResult>;
    abort: (message: PasskeyAbortMessage, sender: SenderInfo) => void;
  };
}

const CONTENT_PER_SECOND = 10;
const CAPTURED_PER_MINUTE = 5;

export function isPopupSender(sender: SenderInfo | undefined, deps: RouterDeps): boolean {
  if (sender?.id !== deps.extensionId || sender.url === undefined) {
    return false;
  }
  if (sender.tab !== undefined && !deps.testPopup) {
    return false;
  }
  return sender.url === deps.popupUrl || (deps.testPopup && sender.url.startsWith(`${deps.popupUrl}?`));
}

export function isContentSender(sender: SenderInfo, deps: RouterDeps): boolean {
  return sender.id === deps.extensionId && sender.tab !== undefined;
}

export class Router {
  readonly #deps: RouterDeps;
  readonly #ports = new Set<PortInfo>();
  readonly #perSecond: RateLimiter;
  readonly #captured: RateLimiter;

  constructor(deps: RouterDeps) {
    this.#deps = deps;
    this.#perSecond = new RateLimiter(CONTENT_PER_SECOND, 1000, deps.now);
    this.#captured = new RateLimiter(CAPTURED_PER_MINUTE, 60_000, deps.now);
  }

  /** `runtime.onConnect`: accepts only the popup's port. */
  onConnect = (port: PortInfo): void => {
    if (port.name !== POPUP_PORT || !isPopupSender(port.sender, this.#deps)) {
      port.disconnect();
      return;
    }
    this.#ports.add(port);
    port.onDisconnect.addListener(() => {
      this.#ports.delete(port);
    });
    port.onMessage.addListener((message) => {
      void this.#onPopupMessage(port, message);
    });
  };

  /**
   * `runtime.onMessage`: a content script's `captured`, or the bridge's passkey messages, rate
   * limited; everything else is dropped. Returns `true` when the answer comes later (`sendResponse`).
   */
  onMessage = (message: unknown, sender: SenderInfo, sendResponse: (response: unknown) => void = () => undefined): boolean => {
    if (!isContentSender(sender, this.#deps)) {
      return false;
    }
    const key = String(sender.tab?.id ?? "");
    if (!this.#perSecond.allow(key)) {
      return false;
    }
    if (isPasskeyKind(message)) {
      return this.#onPasskey(message, sender, sendResponse);
    }
    if (isCapturedMessage(message) && this.#captured.allow(key)) {
      this.#deps.onCaptured?.(message, sender);
    }
    return false;
  };

  /** The bridge's two messages. A request is answered later through `sendResponse` (returns `true`). */
  #onPasskey(message: unknown, sender: SenderInfo, sendResponse: (response: unknown) => void): boolean {
    const { passkey } = this.#deps;
    if (passkey === undefined) {
      return false;
    }
    if (isPasskeyMessage(message)) {
      passkey.handle(message, sender).then(sendResponse, () => {
        sendResponse({ kind: "fallback" } satisfies PasskeyPageResult);
      });
      return true;
    }
    if (isPasskeyAbortMessage(message)) {
      passkey.abort(message, sender);
    }
    return false;
  }

  broadcast(event: PopupEvent): void {
    for (const port of this.#ports) {
      port.postMessage(event);
    }
  }

  forgetTab(tabId: number): void {
    this.#perSecond.forget(String(tabId));
    this.#captured.forget(String(tabId));
  }

  async #onPopupMessage(port: PortInfo, message: unknown): Promise<void> {
    if (!isPopupEnvelope(message)) {
      return;
    }
    const { id, request } = message;
    let reply: PopupReply;
    try {
      reply = { id, ok: true, result: await this.#deps.handlePopup(request, port.sender ?? {}) };
    } catch (error) {
      reply = { id, ok: false, error: errorCodeOf(error) };
    }
    if (this.#ports.has(port)) {
      port.postMessage(reply);
    }
  }
}
