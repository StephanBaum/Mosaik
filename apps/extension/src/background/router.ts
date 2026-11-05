/**
 * Who may talk to the background. Popup operations come only over a port named `popup` from
 * the extension's own popup page; the one message a content script may send is `captured`.
 * Anything else is dropped without a reply, so a page or another extension learns nothing.
 * The worker listens to nothing from other extensions or web pages (no external-message listeners).
 */
import { errorCodeOf } from "../shared/errors.ts";
import {
  isCapturedMessage,
  isPopupEnvelope,
  POPUP_PORT,
  type PopupEvent,
  type PopupReply,
  type PopupRequest,
} from "../shared/messages.ts";
import { RateLimiter } from "./ratelimit.ts";

/** The part of `MessageSender` the router reads. */
export interface SenderInfo {
  id?: string | undefined;
  url?: string | undefined;
  tab?: { id?: number | undefined } | undefined;
  frameId?: number | undefined;
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
  /** The opt-in capture script's message (T4); without a handler it is dropped. */
  onCaptured?: (message: unknown, sender: SenderInfo) => void;
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

  /** `runtime.onMessage`: a content script's `captured`, rate limited; everything else is dropped. */
  onMessage = (message: unknown, sender: SenderInfo): void => {
    if (!isContentSender(sender, this.#deps)) {
      return;
    }
    const key = String(sender.tab?.id ?? "");
    if (!this.#perSecond.allow(key) || !isCapturedMessage(message) || !this.#captured.allow(key)) {
      return;
    }
    this.#deps.onCaptured?.(message, sender);
  };

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
