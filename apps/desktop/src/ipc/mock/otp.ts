/**
 * Mock one-time code setup (ADR-0031). The mock keeps a parsed setup behind a pending ID
 * like Rust does; the "scanned" code is always GitHub / alex@example.com. `parseMockOtp` is
 * mock-only and much simpler than `mosaic-totp`.
 */
import type { OtpParseRequest, OtpPending, TotpCode } from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

/** What a scan finds, without the ID and expiry the mock adds. */
type Parsed = Omit<OtpPending, "pendingId" | "expiresInSeconds">;

export const MOCK_SCANNED: Parsed = {
  scheme: "totp",
  issuer: "GitHub",
  accountName: "alex@example.com",
  algorithm: "sha1",
  digits: 6,
  period: 30,
  strength: "belowRfc4226",
};

/** The code every mock setup shows. */
export const MOCK_CODE = "492039";

const BASE32 = /^[A-Z2-7]+=*$/i;

function strengthOf(secret: string): Parsed["strength"] {
  const length = secret.replace(/=+$/, "").length;
  if (length < 16) {
    return "weak";
  }
  return length < 26 ? "belowRfc4226" : "rfc4226";
}

function algorithmOf(value: string | null): Parsed["algorithm"] {
  const normal = (value ?? "").toLowerCase();
  return normal === "sha256" || normal === "sha512" ? normal : "sha1";
}

function numberParam(value: string | null, fallback: number): number {
  const parsed = Number(value);
  return value !== null && Number.isInteger(parsed) ? parsed : fallback;
}

/** The URL of an `otpauth://` link, checked for prefix, type and secret. */
function checkedUrl(text: string): URL {
  const lower = text.toLowerCase();
  if (lower.startsWith("otpauth-migration://")) {
    fail("otpQrMigration");
  }
  if (!lower.startsWith("otpauth://") || !URL.canParse(text)) {
    fail("otpUriInvalid");
  }
  const url = new URL(text);
  const type = url.hostname.toLowerCase();
  if ((type !== "totp" && type !== "hotp") || !BASE32.test(url.searchParams.get("secret") ?? "")) {
    fail("otpUriInvalid");
  }
  return url;
}

/** The issuer before the first colon of the label (if any) and the account after it. */
function splitLabel(url: URL): { issuer: string | null; account: string } {
  const label = decodeURIComponent(url.pathname.replace(/^\//, ""));
  const split = label.indexOf(":");
  return split < 0
    ? { issuer: null, account: label.trim() }
    : { issuer: label.slice(0, split).trim(), account: label.slice(split + 1).trim() };
}

function parseUri(text: string): Parsed {
  const url = checkedUrl(text.trim());
  const label = splitLabel(url);
  const issuer = url.searchParams.get("issuer") ?? label.issuer;
  const scheme = url.hostname.toLowerCase() === "hotp" ? "hotp" : "totp";
  return {
    scheme,
    issuer: issuer === "" ? null : issuer,
    accountName: label.account === "" ? null : label.account,
    algorithm: algorithmOf(url.searchParams.get("algorithm")),
    digits: numberParam(url.searchParams.get("digits"), 6),
    period: scheme === "hotp" ? null : numberParam(url.searchParams.get("period"), 30),
    strength: strengthOf(url.searchParams.get("secret") ?? ""),
  };
}

function parseKey(request: Extract<OtpParseRequest, { kind: "key" }>): Parsed {
  const secret = request.secret.replace(/[\s-]/g, "");
  if (!BASE32.test(secret)) {
    fail("otpKeyInvalid");
  }
  if (request.digits < 6 || request.digits > 8 || request.period < 1 || request.period > 300) {
    fail("invalidInput");
  }
  const clean = (value: string | null): string | null => {
    const trimmed = value?.trim() ?? "";
    return trimmed === "" ? null : trimmed;
  };
  return {
    scheme: "totp",
    issuer: clean(request.issuer),
    accountName: clean(request.accountName),
    algorithm: request.algorithm,
    digits: request.digits,
    period: request.period,
    strength: strengthOf(secret),
  };
}

export function parseMockOtp(request: OtpParseRequest): Parsed {
  return request.kind === "uri" ? parseUri(request.uri) : parseKey(request);
}

export function holdMockPending(state: MockState, parsed: Parsed): OtpPending {
  state.otpPendingCounter += 1;
  const pending: OtpPending = {
    ...parsed,
    pendingId: `otp-${String(state.otpPendingCounter)}`,
    expiresInSeconds: 600,
  };
  state.otpPending.set(pending.pendingId, pending);
  return pending;
}

/** Drops every pending setup (a lock). */
export function clearMockPending(state: MockState): void {
  state.otpPending.clear();
}

export function mockPendingCode(state: MockState, pendingId: string): TotpCode {
  requireUnlocked(state);
  const pending = state.otpPending.get(pendingId) ?? fail("otpPendingExpired");
  if (pending.scheme === "hotp" || pending.period === null) {
    fail("invalidInput");
  }
  const elapsed = Math.floor(state.now() / 1000) % pending.period;
  return { code: MOCK_CODE, secondsRemaining: pending.period - elapsed, period: pending.period };
}

export const otpHandlers: MockHandlers = {
  otp_scan: (state, request) => {
    requireUnlocked(state);
    const forced = state.options.otpScan;
    if (forced === "cancel" && request.source === "file") {
      return { kind: "cancelled" };
    }
    if (request.source === "screen" && !state.options.screenScan) {
      fail("otpScanUnsupported");
    }
    if (forced !== null && forced !== "cancel") {
      fail(forced);
    }
    const pending = holdMockPending(state, MOCK_SCANNED);
    return { kind: "found", pending, clipboardCleared: request.source === "clipboard" };
  },

  otp_parse: (state, request) => {
    requireUnlocked(state);
    return holdMockPending(state, parseMockOtp(request));
  },

  otp_pending_code: (state, request) => mockPendingCode(state, request.pendingId),

  otp_pending_discard: (state, request) => {
    state.otpPending.delete(request.pendingId);
    return null;
  },
};
