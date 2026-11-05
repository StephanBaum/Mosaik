/**
 * The link to the native host (docs/NATIVE_MESSAGING.md). One lazy port, requests matched to
 * responses by ID, a 20 s timeout per request, at most four in flight, the port closed after
 * 60 s of silence. The host does the authentication; the extension only stores the pairing.
 */
import { ExtensionError, type ErrorCode } from "../shared/errors.ts";
import { HOST_NAME } from "../shared/ids.ts";
import type { ConnectionState } from "../shared/messages.ts";
import {
  IDLE_DISCONNECT_MS,
  MAX_IN_FLIGHT,
  PAIR_TIMEOUT_MS,
  PROTOCOL_VERSION,
  REQUEST_TIMEOUT_MS,
  type DesktopState,
  type HostEvent,
  type Hint,
  type Op,
  type RequestEnvelope,
  type RequestMap,
  type ResultMap,
} from "../shared/protocol.ts";
import { isResultBody, parseHostMessage } from "../shared/validate.ts";
import type { PairingStore } from "./pairing.ts";

/** The part of `chrome.runtime.Port` the client uses (the e2e build supplies a fake). */
export interface NativePort {
  postMessage(message: unknown): void;
  disconnect(): void;
  onMessage: { addListener(listener: (message: unknown) => void): void };
  onDisconnect: { addListener(listener: () => void): void };
}

export type PortFactory = (hostName: string) => NativePort;

export interface NativeOptions {
  connect: PortFactory;
  /** Reads (and clears) `chrome.runtime.lastError` after a disconnect. */
  lastError: () => string | undefined;
  pairings: PairingStore;
  hint: Hint;
  onEvent: (event: HostEvent) => void;
}

interface Pending {
  op: Op;
  resolve: (body: unknown) => void;
  reject: (error: ExtensionError) => void;
  timer: ReturnType<typeof setTimeout>;
}

/** What the client knows of the link, for the popup. */
export interface LinkState {
  checked: boolean;
  desktop: DesktopState | null;
  /** The host established a session with the stored pairing. */
  paired: boolean;
  hasPairing: boolean;
  pairing: boolean;
  error: ErrorCode | null;
}

function failureOf(error: ErrorCode | null): ConnectionState | null {
  if (error === "hostMissing" || error === "hostForbidden") {
    return error;
  }
  return error === "protocolVersion" ? "protocolMismatch" : null;
}

export function connectionOf(link: LinkState): ConnectionState {
  const { error } = link;
  if (link.pairing) {
    return "pairing";
  }
  const failed = failureOf(error);
  if (failed !== null) {
    return failed;
  }
  if (!link.checked) {
    return "checking";
  }
  if (error === "desktopNotRunning" || link.desktop === "notRunning") {
    return "desktopNotRunning";
  }
  if (error === "notPaired" || !link.hasPairing || (error === null && !link.paired)) {
    return "notPaired";
  }
  if (error !== null) {
    return "error";
  }
  return link.desktop === "locked" ? "locked" : "unlocked";
}

/** Maps the browser's wording for a failed `connectNative` to a code. */
export function disconnectCode(message: string | undefined): ErrorCode {
  const text = (message ?? "").toLowerCase();
  if (text.includes("not found")) {
    return "hostMissing";
  }
  if (text.includes("forbidden")) {
    return "hostForbidden";
  }
  return "hostExited";
}

export class NativeClient {
  readonly #options: NativeOptions;
  readonly #pending = new Map<number, Pending>();
  #port: NativePort | null = null;
  #nextId = 1;
  /** Requests made and not yet answered, including ones still waiting for the session. */
  #active = 0;
  #idle: ReturnType<typeof setTimeout> | null = null;
  #session: Promise<void> | null = null;
  readonly state: LinkState = {
    checked: false,
    desktop: null,
    paired: false,
    hasPairing: false,
    pairing: false,
    error: null,
  };

  constructor(options: NativeOptions) {
    this.#options = options;
  }

  get connection(): ConnectionState {
    return connectionOf(this.state);
  }

  /** Sends a request over the established session (opening the port and saying `hello` first). */
  async request<O extends Exclude<Op, "hello" | "pair">>(op: O, body: RequestMap[O]): Promise<ResultMap[O]> {
    if (this.#active >= MAX_IN_FLIGHT) {
      throw new ExtensionError("busy");
    }
    this.#active++;
    try {
      await this.ensureSession();
      return await this.#send(op, body, REQUEST_TIMEOUT_MS);
    } finally {
      this.#active--;
    }
  }

  /** Starts the session (once) and reports the desktop state; used by the popup to learn the link. */
  ensureSession(): Promise<void> {
    this.#session ??= this.#hello().finally(() => {
      this.state.checked = true;
    });
    const session = this.#session;
    session.catch(() => {
      if (this.#session === session) {
        this.#session = null;
      }
    });
    return session;
  }

  /** Pairs with the desktop: the user approves there; `onEvent` receives the code to compare. */
  async pair(): Promise<void> {
    this.state.pairing = true;
    this.state.error = null;
    try {
      this.#open();
      const pairing = await this.#send("pair", { hint: this.#options.hint }, PAIR_TIMEOUT_MS);
      await this.#options.pairings.save(pairing);
      this.#session = null;
    } catch (error) {
      this.#noteError(error);
      throw error;
    } finally {
      this.state.pairing = false;
    }
    await this.ensureSession();
  }

  /** Tells the desktop to forget this pairing, then forgets it here. */
  async unpair(): Promise<void> {
    try {
      await this.request("unpair", {});
    } finally {
      await this.#options.pairings.clear();
      this.state.hasPairing = false;
      this.state.paired = false;
      this.disconnect();
    }
  }

  disconnect(): void {
    const port = this.#port;
    this.#port = null;
    this.#session = null;
    this.#clearIdle();
    port?.disconnect();
    this.#failAll(new ExtensionError("hostExited"));
  }

  async #hello(): Promise<void> {
    const pairing = await this.#options.pairings.load();
    this.state.hasPairing = pairing !== null;
    try {
      this.#open();
      const result = await this.#send(
        "hello",
        { pairing, hint: this.#options.hint },
        REQUEST_TIMEOUT_MS,
      );
      this.state.desktop = result.desktop;
      this.state.paired = result.paired && pairing !== null;
      this.state.error = null;
    } catch (error) {
      this.#noteError(error);
      throw error;
    }
  }

  #noteError(error: unknown): void {
    this.state.error = error instanceof ExtensionError ? error.code : "internal";
    this.state.checked = true;
    if (this.state.error === "notPaired") {
      // The desktop does not know this pairing. The record stays: the popup offers "Connect again".
      this.state.paired = false;
    }
  }

  #open(): NativePort {
    if (this.#port !== null) {
      return this.#port;
    }
    const port = this.#options.connect(HOST_NAME);
    this.#port = port;
    port.onMessage.addListener((message) => {
      this.#onMessage(port, message);
    });
    port.onDisconnect.addListener(() => {
      this.#onDisconnect(port);
    });
    return port;
  }

  #send<O extends Op>(op: O, body: RequestMap[O], timeoutMs: number): Promise<ResultMap[O]> {
    const port = this.#port;
    if (port === null) {
      return Promise.reject(new ExtensionError("hostExited"));
    }
    const id = this.#nextId++;
    const envelope: RequestEnvelope<O> = { v: PROTOCOL_VERSION, id, op, body };
    return new Promise<ResultMap[O]>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new ExtensionError("timeout"));
        this.#touch();
      }, timeoutMs);
      this.#pending.set(id, {
        op,
        resolve: (value) => {
          resolve(value as ResultMap[O]);
        },
        reject,
        timer,
      });
      this.#clearIdle();
      try {
        port.postMessage(envelope);
      } catch {
        this.#settle(id)?.reject(new ExtensionError("hostExited"));
      }
    });
  }

  #settle(id: number): Pending | undefined {
    const pending = this.#pending.get(id);
    if (pending !== undefined) {
      clearTimeout(pending.timer);
      this.#pending.delete(id);
    }
    return pending;
  }

  #onMessage(port: NativePort, raw: unknown): void {
    if (port !== this.#port) {
      return;
    }
    const message = parseHostMessage(raw);
    if (message === null) {
      return;
    }
    if (message.kind === "event") {
      this.#onEvent(message.event);
      return;
    }
    const pending = this.#settle(message.id);
    if (pending === undefined) {
      return;
    }
    if (!message.ok) {
      pending.reject(new ExtensionError(message.error));
    } else if (isResultBody(pending.op, message.body)) {
      pending.resolve(message.body);
    } else {
      pending.reject(new ExtensionError("internal"));
    }
    this.#touch();
  }

  #onEvent(event: HostEvent): void {
    if (event.event === "state") {
      this.state.desktop = event.state;
    } else if (event.event === "unpaired") {
      void this.#options.pairings.clear();
      this.state.hasPairing = false;
      this.state.paired = false;
      this.#session = null;
    } else if (event.event === "disconnected") {
      this.state.desktop = "notRunning";
      this.#session = null;
    }
    this.#options.onEvent(event);
  }

  #onDisconnect(port: NativePort): void {
    if (port !== this.#port) {
      return;
    }
    const code = disconnectCode(this.#options.lastError());
    this.#port = null;
    this.#session = null;
    this.#clearIdle();
    this.state.error = code;
    this.state.checked = true;
    this.#failAll(new ExtensionError(code));
  }

  #failAll(error: ExtensionError): void {
    for (const [id, pending] of this.#pending) {
      clearTimeout(pending.timer);
      this.#pending.delete(id);
      pending.reject(error);
    }
  }

  #clearIdle(): void {
    if (this.#idle !== null) {
      clearTimeout(this.#idle);
      this.#idle = null;
    }
  }

  /** Closes the port after a minute with nothing in flight. */
  #touch(): void {
    this.#clearIdle();
    if (this.#pending.size === 0 && this.#port !== null) {
      this.#idle = setTimeout(() => {
        this.disconnect();
      }, IDLE_DISCONNECT_MS);
    }
  }
}
