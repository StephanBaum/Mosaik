/**
 * The passkey shim (ADR-0040). It runs in the page's own JavaScript world at `document_start`
 * in the top frame of https pages, after the user turned "Use Mosaic for passkeys" on. It
 * replaces `CredentialsContainer.prototype.create` and `.get`: a request Mosaic can answer
 * goes to the bridge (isolated world) and from there to the background and the desktop, and
 * every other request, and every failure, goes to the browser's own WebAuthn with the page's
 * original arguments and `this`.
 *
 * This code is page-visible and page-controlled by design: the page can replace it, watch its
 * messages and forge answers. That gains it nothing it does not already have through
 * `navigator.credentials` for its own origin. Nothing here is trusted by the background, which
 * reads origin and frame from the browser (docs/EXTENSION_SECURITY.md, "Passkeys").
 */
import { BRIDGE_ACK_TIMEOUT_MS } from "../shared/passkey-limits.ts";
import {
  PASSKEY_ABORT_TYPE,
  PASSKEY_REQUEST_TYPE,
  isShimAck,
  isShimResponse,
  type PasskeyOp,
  type PasskeyPageResult,
  type SerializedCreate,
  type SerializedGet,
} from "../shared/passkey-messages.ts";
import { assertedCredential, createdCredential, type CredentialClasses } from "./credential.ts";
import { prepareCreate, prepareGet, signalOf } from "./serialize.ts";

type Native = (...args: unknown[]) => unknown;
type PageWindow = Window & typeof globalThis;

/** Marks a window whose prototype this script already replaced (the browser can inject a script twice). */
const MARKER = "mosaicPasskey";

interface Pending {
  readonly id: number;
  readonly op: PasskeyOp;
  readonly thisArg: unknown;
  readonly args: unknown[];
  readonly native: Native;
  readonly credProps: boolean;
  readonly signal: AbortSignal | null;
  readonly resolve: (value: unknown) => void;
  readonly reject: (reason: unknown) => void;
  onAbort: (() => void) | null;
  ackTimer: number | null;
}

const MESSAGES = {
  NotAllowedError: "The operation either timed out or was not allowed.",
  InvalidStateError: "The user attempted to register an authenticator that contains one of the credentials already registered with the relying party.",
} as const;

/** Everything the shim takes from the page's globals once, before any page script has run. */
function capture(win: PageWindow) {
  /* eslint-disable @typescript-eslint/unbound-method -- taken to be applied with an explicit `this`, so a page cannot swap them later */
  return {
    win,
    apply: Reflect.apply.bind(Reflect) as (target: object, thisArg: unknown, args: unknown[]) => unknown,
    postMessage: win.postMessage,
    addEventListener: win.addEventListener,
    signalAdd: AbortSignal.prototype.addEventListener,
    signalRemove: AbortSignal.prototype.removeEventListener,
    setTimeout: win.setTimeout.bind(win),
    clearTimeout: win.clearTimeout.bind(win),
    Promise,
    Map,
    DOMException,
  };
  /* eslint-enable @typescript-eslint/unbound-method */
}

type Captured = ReturnType<typeof capture>;

class Shim {
  readonly #c: Captured;
  readonly #classes: CredentialClasses | null;
  readonly #credentials: unknown;
  readonly #pending = new Map<number, Pending>();
  #nextId = 1;

  constructor(c: Captured, classes: CredentialClasses | null, credentials: unknown) {
    this.#c = c;
    this.#classes = classes;
    this.#credentials = credentials;
    c.apply(c.addEventListener, c.win, ["message", (event: MessageEvent) => {
      this.#onMessage(event);
    }]);
  }

  /** The replacement for `create` and `get`. */
  intercept(op: PasskeyOp, thisArg: unknown, args: unknown[], native: Native): unknown {
    const passthrough = (): unknown => this.#c.apply(native, thisArg, args);
    if (this.#classes === null || thisArg !== this.#credentials || !this.#c.win.origin.startsWith("https:")) {
      return passthrough();
    }
    const options = args[0];
    const prepared = this.#prepare(op, options);
    if (prepared === null) {
      return passthrough();
    }
    const signal = signalOf(options);
    if (signal?.aborted === true) {
      // The reason is whatever the page's signal carries (WebAuthn rejects with `signal.reason`).
      // eslint-disable-next-line @typescript-eslint/prefer-promise-reject-errors
      return this.#c.Promise.reject(abortReason(signal, this.#c.DOMException));
    }
    const id = this.#nextId++;
    const { Promise: P } = this.#c;
    return new P((resolve, reject) => {
      const entry: Pending = {
        id, op, thisArg, args, native, signal, resolve, reject,
        credProps: prepared.credProps, onAbort: null, ackTimer: null,
      };
      this.#pending.set(id, entry);
      if (!this.#send({ type: PASSKEY_REQUEST_TYPE, id, op, options: prepared.wire })) {
        this.#fallback(entry);
        return;
      }
      this.#watch(entry);
    });
  }

  #prepare(op: PasskeyOp, options: unknown): { wire: SerializedCreate | SerializedGet; credProps: boolean } | null {
    if (op === "create") {
      return prepareCreate(options);
    }
    const wire = prepareGet(options);
    return wire === null ? null : { wire, credProps: false };
  }

  #send(message: unknown): boolean {
    try {
      this.#c.apply(this.#c.postMessage, this.#c.win, [message, this.#c.win.origin]);
      return true;
    } catch {
      return false;
    }
  }

  /** Falls back when the bridge does not answer at all, and tells the bridge when the page aborts. */
  #watch(entry: Pending): void {
    const c = this.#c;
    entry.ackTimer = c.setTimeout(() => {
      // A bridge that is only late (a busy page) may have passed the request on already: it must
      // not leave a desktop dialog open beside the browser's own.
      this.#send({ type: PASSKEY_ABORT_TYPE, id: entry.id });
      this.#fallback(entry);
    }, BRIDGE_ACK_TIMEOUT_MS);
    const { signal } = entry;
    if (signal !== null) {
      entry.onAbort = () => {
        if (this.#release(entry)) {
          this.#send({ type: PASSKEY_ABORT_TYPE, id: entry.id });
          entry.reject(abortReason(signal, c.DOMException));
        }
      };
      c.apply(c.signalAdd, signal, ["abort", entry.onAbort, { once: true }]);
    }
  }

  /** Forgets a request; `false` when it was already settled. */
  #release(entry: Pending): boolean {
    if (this.#pending.get(entry.id) !== entry) {
      return false;
    }
    const c = this.#c;
    this.#pending.delete(entry.id);
    if (entry.ackTimer !== null) {
      c.clearTimeout(entry.ackTimer);
    }
    if (entry.signal !== null && entry.onAbort !== null) {
      c.apply(c.signalRemove, entry.signal, ["abort", entry.onAbort]);
    }
    return true;
  }

  #fallback(entry: Pending): void {
    if (!this.#release(entry)) {
      return;
    }
    try {
      entry.resolve(this.#c.apply(entry.native, entry.thisArg, entry.args));
    } catch (error) {
      entry.reject(error);
    }
  }

  #onMessage(event: MessageEvent): void {
    // Only this window can post here (a child frame's postMessage has another source); the page itself can, by design.
    if (event.source !== this.#c.win) {
      return;
    }
    const data: unknown = event.data;
    if (isShimAck(data)) {
      const entry = this.#pending.get(data.id);
      if (entry?.ackTimer != null) {
        this.#c.clearTimeout(entry.ackTimer);
        entry.ackTimer = null;
      }
    } else if (isShimResponse(data)) {
      const entry = this.#pending.get(data.id);
      if (entry !== undefined) {
        this.#settle(entry, data.result);
      }
    }
  }

  #settle(entry: Pending, result: PasskeyPageResult): void {
    if (result.kind === "fallback") {
      this.#fallback(entry);
      return;
    }
    if (!this.#release(entry)) {
      return;
    }
    if (result.kind === "error") {
      entry.reject(new this.#c.DOMException(MESSAGES[result.name], result.name));
      return;
    }
    const classes = this.#classes;
    const credential =
      classes === null
        ? null
        : result.kind === "created" && entry.op === "create"
          ? createdCredential(classes, result, entry.credProps)
          : result.kind === "asserted" && entry.op === "get"
            ? assertedCredential(classes, result, entry.credProps)
            : null;
    if (credential === null) {
      entry.reject(new this.#c.DOMException(MESSAGES.NotAllowedError, "NotAllowedError"));
    } else {
      entry.resolve(credential);
    }
  }
}

function abortReason(signal: AbortSignal, DOMExceptionClass: typeof DOMException): unknown {
  const reason: unknown = signal.reason;
  return reason ?? new DOMExceptionClass("", "AbortError");
}

function classesOf(win: PageWindow): CredentialClasses | null {
  const credential = (win as { PublicKeyCredential?: CredentialClasses["PublicKeyCredential"] }).PublicKeyCredential;
  if (credential === undefined) {
    return null;
  }
  return {
    PublicKeyCredential: credential,
    AuthenticatorAttestationResponse: (win as { AuthenticatorAttestationResponse?: { prototype: object } }).AuthenticatorAttestationResponse,
    AuthenticatorAssertionResponse: (win as { AuthenticatorAssertionResponse?: { prototype: object } }).AuthenticatorAssertionResponse,
  };
}

/**
 * Replaces `create` and `get` on `CredentialsContainer.prototype` (same descriptor, so they stay
 * writable, enumerable and configurable like the browser's). Returns `false` where there is
 * nothing to replace or the window was already done.
 */
export function installShim(win: PageWindow): boolean {
  const proto = (win as unknown as { CredentialsContainer?: { prototype: Record<string, unknown> } }).CredentialsContainer?.prototype;
  if (proto === undefined || Object.hasOwn(win, MARKER)) {
    return false;
  }
  Object.defineProperty(win, MARKER, { value: true, enumerable: false });
  const shim = new Shim(capture(win), classesOf(win), win.navigator.credentials);
  for (const op of ["create", "get"] as const) {
    const descriptor = Object.getOwnPropertyDescriptor(proto, op);
    const native = descriptor?.value as Native | undefined;
    if (descriptor === undefined || native === undefined) {
      continue;
    }
    // A method with a rest parameter keeps `length` 0 like the native one; the name is the property's.
    const replacement = {
      [op](this: unknown, ...args: unknown[]): unknown {
        return shim.intercept(op, this, args, native);
      },
    }[op];
    Object.defineProperty(proto, op, { ...descriptor, value: replacement });
  }
  return true;
}
