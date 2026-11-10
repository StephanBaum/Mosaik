/**
 * E2E builds only: the password-change half of the fake desktop. It decides like the desktop
 * (`src-tauri/src/browser/rotation.rs`): frame rules, https only, an exact origin (no eTLD+1
 * match), then the attempt; the dialogs end as the test configured. Values are test data.
 */
import { hostOf, isSecureOrigin, samePort, sameSite, siteOf } from "../shared/origin.ts";
import type { RequestMap, ResultMap, RotationMatch } from "../shared/protocol.ts";
import type { E2EConfig, E2EItem, E2EAttempt } from "./e2e-desktop.ts";
import { Failure } from "./e2e-passkey.ts";

export type RotationOp =
  | "rotationForOrigin"
  | "rotationBegin"
  | "fillChange"
  | "fillPending"
  | "rotationResult"
  | "rotationAddOrigin";

const OPS: readonly string[] = ["rotationForOrigin", "rotationBegin", "fillChange", "fillPending", "rotationResult", "rotationAddOrigin"];

export const isRotationOp = (op: string): op is RotationOp => OPS.includes(op);

type Body = Record<string, unknown>;

/** The fake's public suffix list: the guess of the extension plus the private suffixes the tests use. */
const PRIVATE_SUFFIXES = ["github.io"];

function registrable(host: string): string | null {
  const suffix = PRIVATE_SUFFIXES.find((s) => host.endsWith(`.${s}`));
  if (suffix !== undefined) {
    const label = host.slice(0, -(suffix.length + 1)).split(".").at(-1);
    return label === undefined || label === "" ? null : `${label}.${suffix}`;
  }
  return siteOf(host);
}

/** The desktop's dialog-free verdict: https, the same port, the same registrable domain. */
function related(origin: string, from: string | null): boolean {
  if (from === null || !isSecureOrigin(origin) || !isSecureOrigin(from) || !samePort(origin, from)) {
    return false;
  }
  const first = registrable(hostOf(origin));
  return first !== null && first === registrable(hostOf(from));
}

export class FakeRotation {
  readonly #config: () => E2EConfig;
  readonly #nextValue: () => string;
  #counter = 0;

  constructor(config: () => E2EConfig, nextValue: () => string) {
    this.#config = config;
    this.#nextValue = nextValue;
  }

  get #state(): E2EConfig["rotation"] {
    return this.#config().rotation;
  }

  handle(op: RotationOp, body: Body): unknown {
    const typed = body as unknown;
    switch (op) {
      case "rotationForOrigin":
        return this.#rotationsFor(body["origin"] as string, body["relatedTo"] as string | null);
      case "rotationBegin":
        return this.#rotationBegin(typed as RequestMap["rotationBegin"]);
      case "fillChange":
        return this.#fillChange(typed as RequestMap["fillChange"]);
      case "fillPending":
        return this.#fillPending(typed as RequestMap["fillPending"]);
      case "rotationResult":
        return this.#rotationResult(typed as RequestMap["rotationResult"]);
      case "rotationAddOrigin":
        return this.#rotationAddOrigin(typed as RequestMap["rotationAddOrigin"]);
    }
  }

  /** The desktop's rotation checks, in its order: frame rules, https only, exact origin, then the attempt. */
  #rotationItem(origin: string, topOrigin: string, frame: "top" | "sameOriginFrame" | null, ids: { vaultId: string; itemId: string }): E2EItem {
    if (origin !== topOrigin) {
      throw new Failure(frame === "top" ? "invalidRequest" : "crossOriginFrame");
    }
    if (!isSecureOrigin(origin)) {
      throw new Failure("insecureOrigin");
    }
    const item = this.#config().items.find((i) => i.itemId === ids.itemId && i.vaultId === ids.vaultId);
    if (item === undefined) {
      throw new Failure("itemNotFound");
    }
    if (item.origin !== origin && item.extraOrigins?.includes(origin) !== true) {
      throw new Failure("originMismatch");
    }
    return item;
  }

  #attempt(ids: { vaultId: string; itemId: string; attemptId: string }): E2EAttempt {
    const found = this.#state.attempts.find(
      (a) => a.itemId === ids.itemId && a.vaultId === ids.vaultId && a.attemptId === ids.attemptId,
    );
    if (found === undefined) {
      throw new Failure("rotationNotFound");
    }
    return found;
  }

  #rotationsFor(origin: string, relatedTo: string | null): ResultMap["rotationForOrigin"] {
    const attempts: RotationMatch[] = [];
    for (const attempt of this.#state.attempts) {
      const item = this.#config().items.find((i) => i.itemId === attempt.itemId && i.vaultId === attempt.vaultId);
      if (item === undefined || !isSecureOrigin(origin) || (item.origin !== origin && item.extraOrigins?.includes(origin) !== true)) {
        continue;
      }
      attempts.push({
        vaultId: attempt.vaultId,
        itemId: attempt.itemId,
        attemptId: attempt.attemptId,
        title: item.title,
        username: item.username,
        host: hostOf(origin),
        stage: attempt.stage,
        startedAt: 1_767_225_600_000,
        changeUrl: `${item.origin}/change-password.html`,
      });
    }
    return { attempts, related: related(origin, relatedTo) };
  }

  #rotationBegin(body: RequestMap["rotationBegin"]): ResultMap["rotationBegin"] {
    this.#rotationItem(body.origin, body.topOrigin, null, body);
    if (this.#state.begin === "cancelled") {
      return { outcome: "cancelled", attemptId: null };
    }
    if (this.#state.attempts.filter((a) => a.itemId === body.itemId).length >= 4) {
      throw new Failure("rotationLimit");
    }
    this.#counter++;
    const attemptId = `a${String(this.#counter).padStart(16, "0")}.e2e.0`;
    this.#state.attempts.push({
      vaultId: body.vaultId,
      itemId: body.itemId,
      attemptId,
      value: this.#nextValue(),
      stage: "prepared",
    });
    return { outcome: "started", attemptId };
  }

  #fillChange(body: RequestMap["fillChange"]): ResultMap["fillChange"] {
    const item = this.#rotationItem(body.origin, body.topOrigin, body.frame, body);
    const attempt = this.#attempt(body);
    attempt.stage = "delivered";
    return { current: body.shape === "currentAndNew" ? item.password : null, new: attempt.value };
  }

  #fillPending(body: RequestMap["fillPending"]): ResultMap["fillPending"] {
    const item = this.#rotationItem(body.origin, body.topOrigin, body.frame, body);
    const attempt = this.#attempt(body);
    attempt.stage = "delivered";
    return { username: item.username, password: attempt.value };
  }

  #rotationResult(body: RequestMap["rotationResult"]): ResultMap["rotationResult"] {
    const item = this.#rotationItem(body.origin, body.topOrigin, null, body);
    const attempt = this.#attempt(body);
    const outcome = this.#state.result;
    if (outcome === "cancelled") {
      return { outcome };
    }
    if (outcome === "confirmed" && body.outcome === "succeeded") {
      item.password = attempt.value;
    }
    this.#state.attempts = this.#state.attempts.filter((a) => a !== attempt);
    return { outcome: body.outcome === "succeeded" ? outcome : "keptOld" };
  }

  #rotationAddOrigin(body: RequestMap["rotationAddOrigin"]): ResultMap["rotationAddOrigin"] {
    const item = this.#config().items.find((i) => i.itemId === body.itemId && i.vaultId === body.vaultId);
    if (item === undefined) {
      throw new Failure("itemNotFound");
    }
    this.#attempt(body);
    if (body.origin !== body.topOrigin || !sameSite(body.origin, item.origin) || !sameSite(body.fromOrigin, item.origin)) {
      throw new Failure("originNotRelated");
    }
    if (this.#state.addOrigin === "cancelled") {
      return { outcome: "cancelled" };
    }
    item.extraOrigins = [...(item.extraOrigins ?? []), body.origin];
    return { outcome: "added" };
  }
}
