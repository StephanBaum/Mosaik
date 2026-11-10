/**
 * E2E builds only (never bundled into production, see `excludeE2e` and `check-dist.mjs`): a
 * stand-in for host and desktop that speaks the host envelope. It decides like the desktop for
 * the cases the hostile pages need: exact-origin string equality and the HTTP rule. Its log
 * holds ops and origins, never a credential.
 */
import { hostOf } from "../shared/origin.ts";
import type {
  DesktopState,
  LoginMatch,
  OriginWarning,
  Pairing,
  RequestMap,
  ResultMap,
  SearchHit,
} from "../shared/protocol.ts";
import { isOp, isRequestBody } from "../shared/validate.ts";
import { FakePasskeys, Failure } from "./e2e-passkey.ts";
import { FakeRotation, isRotationOp } from "./e2e-rotation.ts";
import type { NativePort } from "./native.ts";

export interface E2EItem {
  vaultId: string;
  itemId: string;
  title: string;
  username: string;
  password: string;
  origin: string;
  allowHttp: boolean;
  totp: string | null;
  /** Related addresses added through `rotationAddOrigin` (exact origins, like saved URLs). */
  extraOrigins?: string[];
}

/** One open password change of the fake desktop. The value is test data. */
export interface E2EAttempt {
  vaultId: string;
  itemId: string;
  attemptId: string;
  value: string;
  stage: "prepared" | "delivered";
}

export interface E2EConfig {
  connection: DesktopState;
  paired: boolean;
  autoApprovePairing: boolean;
  items: E2EItem[];
  /** What `checkSave` answers, and how the (pretend) desktop dialog ends a `proposeSave`. */
  save: { check: ResultMap["checkSave"]; outcome: ResultMap["proposeSave"]["outcome"] };
  /** The open password changes, and how the (pretend) desktop dialogs end. */
  rotation: {
    attempts: E2EAttempt[];
    begin: ResultMap["rotationBegin"]["outcome"];
    result: ResultMap["rotationResult"]["outcome"];
    addOrigin: ResultMap["rotationAddOrigin"]["outcome"];
  };
}

/** A `proposeSave` the fake desktop received (test data only; the real desktop keeps the password in Rust). */
export interface SaveProposal {
  origin: string;
  topOrigin: string;
  username: string | null;
  password: string;
}

export interface LogEntry {
  op: string;
  origin: string | null;
  topOrigin: string | null;
  frame: string | null;
}

export const FAKE_PAIRING: Pairing = { pairingId: "11".repeat(16), key: "22".repeat(32) };
const GENERATED = "Gen-Pw-7731!xYzQ";

export const defaultConfig = (): E2EConfig => ({
  connection: "unlocked",
  paired: true,
  autoApprovePairing: true,
  items: [],
  save: { check: { kind: "new", itemTitle: null }, outcome: "saved" },
  rotation: { attempts: [], begin: "started", result: "confirmed", addOrigin: "added" },
});

type Body = Record<string, unknown>;

const insecure = (origin: string): boolean => origin.startsWith("http:");

export class FakeDesktop {
  config: E2EConfig = defaultConfig();
  readonly log: LogEntry[] = [];
  readonly proposals: SaveProposal[] = [];
  /** The passkey half: dialog decisions, the request log and the credentials it made. */
  readonly passkeys = new FakePasskeys();
  /** The password-change half: the open attempts and how its dialogs end. */
  readonly rotation = new FakeRotation(
    () => this.config,
    () => this.#nextGenerated(),
  );
  #generated = 0;

  /** A port the `NativeClient` can use in place of `connectNative`. */
  createPort(): NativePort {
    const messageListeners: ((message: unknown) => void)[] = [];
    let authenticated = false;
    const reply = (message: unknown): void => {
      setTimeout(() => {
        for (const listener of messageListeners) {
          listener(message);
        }
      }, 0);
    };
    return {
      postMessage: (raw) => {
        const request = raw as { id: number; op: string; body: unknown };
        const succeed = (body: unknown): void => {
          if (request.op === "hello") {
            authenticated = (body as ResultMap["hello"]).paired;
          }
          reply({ v: 1, id: request.id, ok: true, body });
        };
        const fail = (error: unknown): void => {
          reply({ v: 1, id: request.id, ok: false, error: error instanceof Failure ? error.code : "internal" });
        };
        // A passkey op waits for the test to decide in the pretend dialog, so its answer is a promise.
        const respond = (outcome: () => unknown): void => {
          try {
            const body = outcome();
            if (body instanceof Promise) {
              body.then(succeed, fail);
            } else {
              succeed(body);
            }
          } catch (error) {
            fail(error);
          }
        };
        respond(() => this.#handle(request, authenticated, reply));
      },
      disconnect: () => undefined,
      onMessage: {
        addListener: (listener) => {
          messageListeners.push(listener);
        },
      },
      onDisconnect: { addListener: () => undefined },
    };
  }

  #handle(request: { op: string; body: unknown }, authenticated: boolean, reply: (m: unknown) => void): unknown {
    const op = request.op;
    if (!isOp(op) || !isRequestBody(op, request.body)) {
      throw new Failure("invalidRequest");
    }
    this.log.push(this.#entry(op, request.body));
    if (op === "hello") {
      return this.#hello(request.body as RequestMap["hello"]);
    }
    if (op === "pair") {
      return this.#pair(reply);
    }
    if (this.config.connection === "notRunning") {
      throw new Failure("desktopNotRunning");
    }
    if (op === "status") {
      return { state: this.config.connection };
    }
    if (op === "focus") {
      return {};
    }
    if (!authenticated) {
      throw new Failure("notPaired");
    }
    if (this.config.connection === "locked" && op !== "unpair") {
      throw new Failure("locked");
    }
    return this.#authenticated(op, request.body as Body);
  }

  #nextGenerated(): string {
    this.#generated++;
    return `${GENERATED}-${String(this.#generated)}`;
  }

  #entry(op: string, body: unknown): LogEntry {
    const fields = (typeof body === "object" && body !== null ? body : {}) as Body;
    const text = (key: string): string | null => {
      const value = fields[key];
      return typeof value === "string" ? value : null;
    };
    return { op, origin: text("origin"), topOrigin: text("topOrigin"), frame: text("frame") };
  }

  #hello(body: RequestMap["hello"]): ResultMap["hello"] {
    const known = body.pairing?.pairingId === FAKE_PAIRING.pairingId && this.config.paired;
    return { desktop: this.config.connection, paired: this.config.connection !== "notRunning" && known };
  }

  #pair(reply: (m: unknown) => void): ResultMap["pair"] {
    if (this.config.connection === "notRunning") {
      throw new Failure("desktopNotRunning");
    }
    if (this.config.connection === "locked") {
      throw new Failure("locked");
    }
    reply({ v: 1, event: "pairCode", requestId: 1, code: "4F7K2Q", expiresInSeconds: 120 });
    if (!this.config.autoApprovePairing) {
      throw new Failure("pairingDenied");
    }
    this.config.paired = true;
    return FAKE_PAIRING;
  }

  // eslint-disable-next-line complexity -- one flat dispatch over the ops, like the desktop's allowlist
  #authenticated(op: Exclude<keyof RequestMap, "hello" | "pair" | "status" | "focus">, body: Body): unknown {
    if (isRotationOp(op)) {
      return this.rotation.handle(op, body);
    }
    switch (op) {
      case "passkeyCreate":
        return this.passkeys.create(body as unknown as RequestMap["passkeyCreate"]);
      case "passkeyGet":
        return this.passkeys.get(body as unknown as RequestMap["passkeyGet"]);
      case "passkeyCancel":
        this.passkeys.cancel((body as unknown as RequestMap["passkeyCancel"]).requestId);
        return {};
      case "listForOrigin":
        return this.#list(body["origin"] as string);
      case "search":
        return this.#search(body["query"] as string, body["origin"] as string | null);
      case "fill":
        return this.#fill(body as RequestMap["fill"]);
      case "copy":
        return { clearAfterSeconds: 30 };
      case "generate":
        // Every value differs, so a test can tell a held password from a fresh one.
        return { value: this.#nextGenerated(), entropyBits: 100, strength: "strong", rulesSource: null };
      case "checkSave":
        return { ...this.config.save.check };
      case "proposeSave": {
        const { origin, topOrigin, username, password } = body as unknown as SaveProposal;
        this.proposals.push({ origin, topOrigin, username, password });
        return { outcome: this.config.save.outcome, vaultId: null, itemId: null };
      }
      case "unpair":
        this.config.paired = false;
        return {};
    }
  }

  #list(origin: string): ResultMap["listForOrigin"] {
    const items: LoginMatch[] = this.config.items
      .filter((item) => item.origin === origin)
      .map((item) => ({
        vaultId: item.vaultId,
        itemId: item.itemId,
        title: item.title,
        username: item.username,
        host: hostOf(item.origin),
        hasOtp: item.totp !== null,
        favorite: false,
        matchedBy: "exactOrigin",
        blocked: insecure(origin) && !item.allowHttp ? "insecure" : null,
      }));
    const warnings: OriginWarning[] = [];
    const host = hostOf(origin);
    if (host.split(".").some((label) => label.startsWith("xn--"))) {
      warnings.push({ kind: "idn", unicodeHost: host });
    }
    if (insecure(origin)) {
      warnings.push({ kind: "insecure" });
    }
    return { items, warnings };
  }

  #search(query: string, origin: string | null): ResultMap["search"] {
    const needle = query.toLowerCase();
    const items: SearchHit[] = this.config.items
      .filter((item) => item.title.toLowerCase().includes(needle))
      .map((item) => ({
        vaultId: item.vaultId,
        itemId: item.itemId,
        title: item.title,
        username: item.username,
        host: hostOf(item.origin),
        hasOtp: item.totp !== null,
        favorite: false,
        fillable: item.origin === origin && !(insecure(item.origin) && !item.allowHttp),
      }));
    return { items };
  }

  /** The desktop's checks for a fill, in its order. */
  #checkFill(body: RequestMap["fill"], item: E2EItem | undefined): E2EItem {
    if (body.origin !== body.topOrigin) {
      throw new Failure(body.frame === "top" ? "invalidRequest" : "crossOriginFrame");
    }
    if (item === undefined) {
      throw new Failure("itemNotFound");
    }
    if (item.origin !== body.origin) {
      throw new Failure("originMismatch");
    }
    if (insecure(body.origin) && !item.allowHttp) {
      throw new Failure("insecureOrigin");
    }
    if (body.want.otp && item.totp === null && !body.want.login) {
      throw new Failure("noOtp");
    }
    return item;
  }

  #fill(body: RequestMap["fill"]): ResultMap["fill"] {
    const found = this.config.items.find((i) => i.itemId === body.itemId && i.vaultId === body.vaultId);
    const item = this.#checkFill(body, found);
    return {
      username: body.want.login ? item.username : null,
      password: body.want.login ? item.password : null,
      otp: body.want.otp ? item.totp : null,
    };
  }
}
