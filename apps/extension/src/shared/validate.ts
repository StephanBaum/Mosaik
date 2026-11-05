/**
 * Strict validators for what the host sends back, and for the bodies the extension sends
 * (the fixture test runs both directions). Objects must have exactly the expected keys, like
 * the Rust side with `deny_unknown_fields` and every key present.
 */
import { isWireErrorCode } from "./errors.ts";
import {
  OPS,
  type CopySource,
  type HostEvent,
  type HostMessage,
  type Op,
  type OriginWarning,
  type RequestMap,
  type ResultMap,
} from "./protocol.ts";

export type Guard<T> = (value: unknown) => value is T;
type Shape = Readonly<Record<string, Guard<unknown>>>;
type Infer<S extends Shape> = { [K in keyof S]: S[K] extends Guard<infer T> ? T : never };

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export const str =
  (maxLength = 4096): Guard<string> =>
  (value): value is string =>
    typeof value === "string" && value.length <= maxLength;

export const bool: Guard<boolean> = (value): value is boolean => typeof value === "boolean";

export const int =
  (min: number, max: number): Guard<number> =>
  (value): value is number =>
    typeof value === "number" && Number.isInteger(value) && value >= min && value <= max;

export const num: Guard<number> = (value): value is number =>
  typeof value === "number" && Number.isFinite(value);

export const unknownValue: Guard<unknown> = (value): value is unknown => value !== undefined;

export const exactly =
  <const T extends string | number | boolean>(expected: T): Guard<T> =>
  (value): value is T =>
    value === expected;

export const lit =
  <const T extends string>(...allowed: readonly T[]): Guard<T> =>
  (value): value is T =>
    typeof value === "string" && (allowed as readonly string[]).includes(value);

export const nullable =
  <T>(inner: Guard<T>): Guard<T | null> =>
  (value): value is T | null =>
    value === null || inner(value);

export const arr =
  <T>(inner: Guard<T>, maxLength: number): Guard<T[]> =>
  (value): value is T[] =>
    Array.isArray(value) && value.length <= maxLength && value.every((item) => inner(item));

/** An object with exactly the keys of `shape`, each present and passing its guard. */
export const obj =
  <const S extends Shape>(shape: S): Guard<Infer<S>> =>
  (value): value is Infer<S> => {
    if (!isRecord(value)) {
      return false;
    }
    const keys = Object.keys(shape);
    if (Object.keys(value).length !== keys.length) {
      return false;
    }
    return keys.every((key) => Object.hasOwn(value, key) && shape[key]?.(value[key]) === true);
  };

/** One of several objects, told apart by the string at key `tag`. */
export const tagged =
  <T>(tag: string, variants: Readonly<Record<string, Guard<T>>>): Guard<T> =>
  (value): value is T => {
    if (!isRecord(value)) {
      return false;
    }
    const kind = value[tag];
    return typeof kind === "string" && Object.hasOwn(variants, kind) && variants[kind]?.(value) === true;
  };

const empty = obj({});
const id = str(128);
const one = exactly(1 as const);
const origin = str(2048);
const lockState = lit("locked", "unlocked");
const hex = (length: number): Guard<string> => {
  const pattern = new RegExp(`^[0-9a-f]{${String(length)}}$`);
  return (value): value is string => typeof value === "string" && pattern.test(value);
};
export const isPairing = obj({ pairingId: hex(32), key: hex(64) });
const hint = lit("chrome", "edge", "brave", "opera", "vivaldi", "chromium", "firefox", "other");

const loginMatch = obj({
  vaultId: id,
  itemId: id,
  title: nullable(str(256)),
  username: nullable(str(256)),
  host: nullable(str(512)),
  hasOtp: bool,
  favorite: bool,
  matchedBy: lit("exactOrigin", "etld1"),
  blocked: nullable(lit("insecure")),
});

const searchHit = obj({
  vaultId: id,
  itemId: id,
  title: str(256),
  username: nullable(str(256)),
  host: nullable(str(512)),
  hasOtp: bool,
  favorite: bool,
  fillable: bool,
});

const warning = tagged<OriginWarning>("kind", {
  idn: obj({ kind: lit("idn"), unicodeHost: str(512) }),
  insecure: obj({ kind: lit("insecure") }),
  lookalike: obj({ kind: lit("lookalike"), savedHost: str(512) }),
});

export const copySource = tagged<CopySource>("kind", {
  item: obj({ kind: lit("item"), vaultId: id, itemId: id, field: lit("username", "password", "totp") }),
  generated: obj({ kind: lit("generated") }),
});

/** The desktop validates the inside of a generator request; here it must be a tagged object. */
export const isGenerateRequest = (value: unknown): value is RequestMap["generate"]["request"] =>
  isRecord(value) && typeof value["kind"] === "string";

const saveBody = obj({ origin, topOrigin: origin, username: nullable(str(256)), password: str(1024) });

type Guards<M> = { readonly [O in Op]: Guard<M[O & keyof M]> };

const requestGuards: Guards<RequestMap> = {
  hello: obj({ pairing: nullable(isPairing), hint }),
  pair: obj({ hint }),
  status: empty,
  focus: empty,
  listForOrigin: obj({ origin, topOrigin: origin }),
  search: obj({ query: str(128), origin: nullable(origin) }),
  fill: obj({
    vaultId: id,
    itemId: id,
    origin,
    topOrigin: origin,
    frame: lit("top", "sameOriginFrame"),
    want: obj({ login: bool, otp: bool }),
  }),
  copy: obj({ source: copySource }),
  generate: obj({ request: isGenerateRequest }),
  checkSave: saveBody,
  proposeSave: saveBody,
  unpair: empty,
};

const resultGuards: Guards<ResultMap> = {
  hello: obj({ desktop: lit("notRunning", "locked", "unlocked"), paired: bool }),
  pair: isPairing,
  status: obj({ state: lockState }),
  focus: empty,
  listForOrigin: obj({ items: arr(loginMatch, 50), warnings: arr(warning, 8) }),
  search: obj({ items: arr(searchHit, 20) }),
  fill: obj({ username: nullable(str(256)), password: nullable(str(1024)), otp: nullable(str(32)) }),
  copy: obj({ clearAfterSeconds: nullable(int(0, 86_400)) }),
  generate: obj({
    value: str(1024),
    entropyBits: num,
    strength: str(32),
    rulesSource: nullable(lit("page", "siteQuirks")),
  }),
  checkSave: obj({ kind: lit("new", "update", "unchanged"), itemTitle: nullable(str(256)) }),
  proposeSave: obj({
    outcome: lit("saved", "updated", "cancelled"),
    vaultId: nullable(id),
    itemId: nullable(id),
  }),
  unpair: empty,
};

export function isOp(value: unknown): value is Op {
  return typeof value === "string" && (OPS as readonly string[]).includes(value);
}

/** Whether `body` is a valid request body of `op`. */
export function isRequestBody<O extends Op>(op: O, body: unknown): body is RequestMap[O] {
  return requestGuards[op](body);
}

/** Whether `body` is a valid response body of `op`. */
export function isResultBody<O extends Op>(op: O, body: unknown): body is ResultMap[O] {
  return resultGuards[op](body);
}

const requestId = int(0, 4_294_967_295);

const isEvent: Guard<HostEvent> = tagged<HostEvent>("event", {
  state: obj({ v: one, event: exactly("state" as const), state: lockState }),
  unpaired: obj({ v: one, event: exactly("unpaired" as const) }),
  disconnected: obj({ v: one, event: exactly("disconnected" as const) }),
  pairCode: obj({
    v: one,
    event: exactly("pairCode" as const),
    requestId,
    code: str(16),
    expiresInSeconds: int(0, 3600),
  }),
});

const okResponse = obj({ v: one, id: requestId, ok: exactly(true as const), body: unknownValue });
const failResponse = obj({
  v: one,
  id: requestId,
  ok: exactly(false as const),
  error: isWireErrorCode,
});

/**
 * Parses one message from the host: a response of the right shape or a known event. Anything
 * else (unknown keys, wrong version, unknown event) is `null` and gets dropped.
 */
export function parseHostMessage(raw: unknown): HostMessage | null {
  if (isEvent(raw)) {
    return { kind: "event", event: raw };
  }
  if (okResponse(raw)) {
    return { kind: "response", id: raw.id, ok: true, body: raw.body };
  }
  if (failResponse(raw)) {
    return { kind: "response", id: raw.id, ok: false, error: raw.error };
  }
  return null;
}
