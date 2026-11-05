/**
 * E2E builds only (never bundled into production, see `excludeE2e` and `check-dist.mjs`): a
 * stand-in for host and desktop that speaks the host envelope. It decides like the desktop for
 * the cases the hostile pages need: exact-origin string equality and the HTTP rule. Its log
 * holds ops and origins, never a credential.
 */
import type { WireErrorCode } from "../shared/errors.ts";
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
}

export interface E2EConfig {
  connection: DesktopState;
  paired: boolean;
  autoApprovePairing: boolean;
  items: E2EItem[];
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
});

type Body = Record<string, unknown>;

class Failure extends Error {
  readonly code: WireErrorCode;

  constructor(code: WireErrorCode) {
    super(code);
    this.code = code;
  }
}

const insecure = (origin: string): boolean => origin.startsWith("http:");

export class FakeDesktop {
  config: E2EConfig = defaultConfig();
  readonly log: LogEntry[] = [];
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
        const respond = (outcome: () => unknown): void => {
          try {
            const body = outcome();
            if (request.op === "hello") {
              authenticated = (body as ResultMap["hello"]).paired;
            }
            reply({ v: 1, id: request.id, ok: true, body });
          } catch (error) {
            const code = error instanceof Failure ? error.code : "internal";
            reply({ v: 1, id: request.id, ok: false, error: code });
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

  #authenticated(op: Exclude<keyof RequestMap, "hello" | "pair" | "status" | "focus">, body: Body): unknown {
    switch (op) {
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
        this.#generated++;
        return { value: `${GENERATED}-${String(this.#generated)}`, entropyBits: 100, strength: "strong", rulesSource: null };
      case "checkSave":
        return { kind: "new", itemTitle: null };
      case "proposeSave":
        return { outcome: "cancelled", vaultId: null, itemId: null };
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
