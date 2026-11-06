/**
 * Passkey requests from the bridge (ADR-0040). This is where the trust boundary sits: the
 * page, the shim and the bridge are all page-influenced, so nothing they say about the origin,
 * the frame or the request is believed. The origin and the frame come from the browser
 * (`MessageSender`), the bridge's own `window.origin` must agree with them, the request ID is
 * made here, and the desktop (which checks the relying party ID against the public suffix list,
 * builds `clientDataJSON` itself and asks the user in its own dialog) decides.
 *
 * Whatever Mosaic does not answer goes back as `fallback`, and the page's call then reaches the
 * browser's own WebAuthn.
 */
import { errorCodeOf } from "../shared/errors.ts";
import { originOf } from "../shared/origin.ts";
import { PAGE_MAX_IN_FLIGHT } from "../shared/passkey-limits.ts";
import type { PasskeyAbortMessage, PasskeyMessage, PasskeyPageResult } from "../shared/passkey-messages.ts";
import {
  PASSKEY_PROMPT_TIMEOUT_MS,
  type PasskeyCreateBody,
  type PasskeyGetBody,
  type RequestMap,
  type ResultMap,
} from "../shared/protocol.ts";
import type { SenderInfo } from "./router.ts";

export interface PasskeyDeps {
  native: {
    request<O extends "passkeyCreate" | "passkeyGet" | "passkeyCancel">(op: O, body: RequestMap[O]): Promise<ResultMap[O]>;
    ensureSession(): Promise<void>;
    readonly connection: string;
  };
  tabs: { urlOf(tabId: number): Promise<string | undefined> };
  now: () => number;
  /** `bytes` random bytes as lowercase hex. */
  randomHex: (bytes: number) => string;
  /** The setting "Use Mosaic for passkeys" (scripts registered, site access granted). */
  enabled: () => Promise<boolean>;
}

interface Open {
  requestId: string;
  tabId: number;
  /** The sender's origin as the browser reports it (checked against the bridge and the tab before anything is sent). */
  origin: string;
  startedAt: number;
  cancelled: boolean;
  /** The desktop was asked: only then does a cancel need `passkeyCancel`. */
  sent: boolean;
}

const FALLBACK: PasskeyPageResult = { kind: "fallback" };
const NOT_ALLOWED: PasskeyPageResult = { kind: "error", name: "NotAllowedError" };
const REQUEST_ID_BYTES = 16;
/**
 * The codes that end the page's call with the browser's own error; every other code means the browser's WebAuthn takes over.
 * `rateLimited` is not one of them: the desktop's rate windows belong to the pairing, so one site could spend them and
 * make another site's call fail outright.
 */
const REFUSED = new Set(["cancelled", "busy", "timeout"]);

/** The sender's origin as the browser reports it: `sender.origin` (Chromium) or the origin of `sender.url` (Firefox). */
export function senderOrigin(sender: SenderInfo): string | null {
  if (sender.origin !== undefined) {
    return sender.origin;
  }
  try {
    return sender.url === undefined ? null : new URL(sender.url).origin;
  } catch {
    return null;
  }
}

function errorResult(error: unknown): PasskeyPageResult {
  const code = errorCodeOf(error);
  if (code === "passkeyExcluded") {
    return { kind: "error", name: "InvalidStateError" };
  }
  return REFUSED.has(code) ? NOT_ALLOWED : FALLBACK;
}

function createdResult(answer: ResultMap["passkeyCreate"]): PasskeyPageResult {
  const credential = answer.credential;
  if (answer.outcome !== "created" || credential === null) {
    return FALLBACK;
  }
  return {
    kind: "created",
    credentialId: credential.credentialId,
    clientDataJson: credential.clientDataJson,
    attestationObject: credential.attestationObject,
    authenticatorData: credential.authenticatorData,
    publicKeySpki: credential.publicKeySpki,
    publicKeyAlgorithm: credential.publicKeyAlgorithm,
  };
}

function assertedResult(answer: ResultMap["passkeyGet"]): PasskeyPageResult {
  const assertion = answer.assertion;
  if (answer.outcome !== "asserted" || assertion === null) {
    return FALLBACK;
  }
  return { kind: "asserted", ...assertion };
}

export class PasskeyService {
  readonly #deps: PasskeyDeps;
  /** Open requests by `tabId:documentId:pageRequestId`, so a page can only name (and abort) its own. */
  readonly #open = new Map<string, Open>();

  constructor(deps: PasskeyDeps) {
    this.#deps = deps;
  }

  /** The origin of the page, or `null` when this sender may not ask (not the top frame of an https page that agrees with itself). */
  async #origin(claimed: string, sender: SenderInfo): Promise<{ tabId: number; origin: string } | null> {
    const tabId = sender.tab?.id;
    const origin = senderOrigin(sender);
    if (sender.frameId !== 0 || tabId === undefined || origin === null || origin !== claimed || !origin.startsWith("https://")) {
      return null;
    }
    // The tab must still show that site: a page navigated away from it is not asking for its passkeys.
    return originOf(await this.#deps.tabs.urlOf(tabId).catch(() => undefined)) === origin ? { tabId, origin } : null;
  }

  async #ready(): Promise<boolean> {
    const { native } = this.#deps;
    try {
      if (!(await this.#deps.enabled())) {
        return false;
      }
      await native.ensureSession();
    } catch {
      return false;
    }
    // Locked, not paired or no desktop: the browser's own WebAuthn answers, and the desktop is not asked.
    return native.connection === "unlocked";
  }

  async handle(message: PasskeyMessage, sender: SenderInfo): Promise<PasskeyPageResult> {
    const tabId = sender.tab?.id;
    if (tabId === undefined || sender.frameId !== 0) {
      return FALLBACK;
    }
    this.#prune();
    const key = this.#key(tabId, sender, message.pageRequestId);
    const mine = [...this.#open.values()].filter((entry) => entry.tabId === tabId).length;
    if (this.#open.has(key) || mine >= PAGE_MAX_IN_FLIGHT) {
      return FALLBACK;
    }
    // Held from the first moment, so an abort, a navigation or a closed tab that comes while the
    // checks below wait is not lost: the desktop is then never asked.
    const open: Open = {
      requestId: this.#deps.randomHex(REQUEST_ID_BYTES),
      tabId,
      origin: senderOrigin(sender) ?? "",
      startedAt: this.#deps.now(),
      cancelled: false,
      sent: false,
    };
    this.#open.set(key, open);
    try {
      const site = await this.#origin(message.origin, sender);
      if (site === null || !(await this.#ready())) {
        return FALLBACK;
      }
      if (open.cancelled) {
        return NOT_ALLOWED;
      }
      open.sent = true;
      return await this.#ask(message, open);
    } catch (error) {
      return errorResult(error);
    } finally {
      if (this.#open.get(key) === open) {
        this.#open.delete(key);
      }
    }
  }

  /** Both origins are the sender's: the desktop refuses a frame whose origin differs from the tab's. */
  async #ask(message: PasskeyMessage, open: Open): Promise<PasskeyPageResult> {
    const { native } = this.#deps;
    const base = { requestId: open.requestId, origin: open.origin, topOrigin: open.origin };
    if (message.op === "create") {
      const body: PasskeyCreateBody = { ...base, ...message.options };
      return createdResult(await native.request("passkeyCreate", body));
    }
    const body: PasskeyGetBody = { ...base, ...message.options };
    return assertedResult(await native.request("passkeyGet", body));
  }

  /** The page aborted its call (`AbortSignal`): close the desktop's dialog for it, if it is the sender's own request. */
  abort(message: PasskeyAbortMessage, sender: SenderInfo): void {
    const tabId = sender.tab?.id;
    if (tabId === undefined || sender.frameId !== 0) {
      return;
    }
    const open = this.#open.get(this.#key(tabId, sender, message.pageRequestId));
    if (open !== undefined) {
      this.#cancel(open);
    }
  }

  /** The tab was closed or replaced. */
  onTabGone(tabId: number): void {
    this.#cancelWhere((open) => open.tabId === tabId);
  }

  /** The tab loads another document (`url` undefined) or moved to `url`: requests of the old page end. */
  onNavigated(tabId: number, url?: string): void {
    this.#cancelWhere((open) => open.tabId === tabId && (url === undefined || originOf(url) !== open.origin));
  }

  #cancelWhere(match: (open: Open) => boolean): void {
    for (const open of this.#open.values()) {
      if (match(open)) {
        this.#cancel(open);
      }
    }
  }

  #cancel(open: Open): void {
    if (open.cancelled) {
      return;
    }
    open.cancelled = true;
    if (!open.sent) {
      // `handle` sees the flag before it asks the desktop.
      return;
    }
    // The answer to the open request follows (`cancelled`); a failed cancel leaves the dialog to expire.
    void this.#deps.native.request("passkeyCancel", { requestId: open.requestId }).catch(() => undefined);
  }

  #key(tabId: number, sender: SenderInfo, pageRequestId: number): string {
    return `${String(tabId)}:${sender.documentId ?? ""}:${String(pageRequestId)}`;
  }

  /** Forgets requests that outlived the desktop's own timeout (the worker was suspended meanwhile). */
  #prune(): void {
    const limit = this.#deps.now() - PASSKEY_PROMPT_TIMEOUT_MS;
    for (const [key, open] of this.#open) {
      if (open.startedAt < limit) {
        this.#open.delete(key);
      }
    }
  }
}
