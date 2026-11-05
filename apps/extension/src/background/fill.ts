/**
 * The fill algorithm (docs/EXTENSION_SECURITY.md, "fill"). Every fill starts with a user
 * gesture (a popup click, the `fill-login` shortcut or a context-menu item) and ends with one
 * credential in one frame. Nothing here runs on page load or on a page's request.
 */
import { ExtensionError, errorCodeOf, type ErrorCode } from "../shared/errors.ts";
import { newFillId } from "../shared/ids.ts";
import type { FillReply, FillSummary, FrameProbe } from "../shared/messages.ts";
import { originOf } from "../shared/origin.ts";
import type { LoginMatch, RequestMap, ResultMap } from "../shared/protocol.ts";
import type { Badge } from "./badge.ts";
import { chooseTarget, decideFrame, wantFor } from "./frames.ts";
import { lastErrorKey, lastFilledKey, type SessionStore } from "./session.ts";
import type { FrameTarget, TabApi } from "./tabs.ts";

/** How long a filled item counts as "the one whose code is wanted next". */
export const LAST_FILLED_TTL_MS = 5 * 60_000;

export interface FillDeps {
  native: {
    request<O extends "listForOrigin" | "fill" | "generate">(op: O, body: RequestMap[O]): Promise<ResultMap[O]>;
  };
  tabs: TabApi;
  session: SessionStore;
  badge: Badge;
  now: () => number;
}

export interface FillRequest {
  /** Where the user's click was: the popup, the shortcut or the context menu. */
  gesture: "popup" | "command" | "menu";
  tabId: number;
  tabUrl: string | undefined;
  /** The frame under the context menu, if that was the gesture. */
  menuFrameId: number | null;
  /** The item the user chose; null lets the shortcut and the menu pick the first match. */
  item: ItemRef | null;
  otpOnly: boolean;
  confirmFrame: boolean;
}

export interface ItemRef {
  vaultId: string;
  itemId: string;
  title: string | null;
  username: string | null;
  /** Null when not known; the desktop then answers a combined request without a code. */
  hasOtp: boolean | null;
}

export interface LastFilled {
  origin: string;
  vaultId: string;
  itemId: string;
  at: number;
  title: string | null;
  username: string | null;
  hasOtp: boolean;
  otpFilled: boolean;
}

export type FillOutcome = { ok: true; summary: FillSummary } | { ok: false; error: ErrorCode };

/** Test seams, set only by the e2e build: nothing in a production build assigns them. */
export const fillHooks: { afterProbe?: () => Promise<void> } = {};

interface Prepared {
  topOrigin: string;
  target: FrameProbe;
  frame: "top" | "sameOriginFrame";
}

/** Steps 1 to 5: top origin, injection, probe, target frame, frame decision. */
async function prepare(deps: FillDeps, req: FillRequest): Promise<Prepared> {
  const topOrigin = originOf(req.tabUrl);
  if (topOrigin === null) {
    throw new ExtensionError("unsupportedPage");
  }
  await deps.tabs.inject(req.tabId);
  const probes = await deps.tabs.probe(req.tabId);
  await fillHooks.afterProbe?.();
  const target = chooseTarget(probes, req.menuFrameId);
  if (target === null) {
    throw new ExtensionError("noFields");
  }
  const decision = decideFrame(target, topOrigin, req.confirmFrame);
  if (!decision.ok) {
    throw new ExtensionError(decision.error);
  }
  return { topOrigin, target, frame: decision.frame };
}

const frameOf = (p: Prepared): FrameTarget => ({ frameId: p.target.frameId, documentId: p.target.documentId });

async function recentlyFilled(deps: FillDeps, tabId: number, origin: string): Promise<LastFilled | null> {
  const last = (await deps.session.get(lastFilledKey(tabId))) as LastFilled | undefined;
  return last !== undefined && last.origin === origin && deps.now() - last.at < LAST_FILLED_TTL_MS ? last : null;
}

/** Step 6 for the shortcut and the menu: the first usable match, or the right one for a code field. */
async function pickItem(deps: FillDeps, req: FillRequest, p: Prepared): Promise<ItemRef> {
  const { items } = await deps.native.request("listForOrigin", { origin: p.target.origin, topOrigin: p.topOrigin });
  const usable = items.filter((item: LoginMatch) => item.blocked === null);
  const { counts } = p.target;
  const codeOnly = counts.otp > 0 && counts.username + counts.password === 0;
  const last = codeOnly ? await recentlyFilled(deps, req.tabId, p.target.origin) : null;
  const chosen =
    (last === null ? undefined : usable.find((m) => m.itemId === last.itemId && m.vaultId === last.vaultId)) ??
    (codeOnly ? usable.find((m) => m.hasOtp) : usable[0]);
  if (chosen === undefined) {
    throw new ExtensionError("noMatch");
  }
  return { vaultId: chosen.vaultId, itemId: chosen.itemId, title: chosen.title, username: chosen.username, hasOtp: chosen.hasOtp };
}

async function remember(deps: FillDeps, tabId: number, origin: string, filled: ItemRef & { otpFilled: boolean }): Promise<void> {
  const record: LastFilled = {
    origin,
    vaultId: filled.vaultId,
    itemId: filled.itemId,
    at: deps.now(),
    title: filled.title,
    username: filled.username,
    hasOtp: filled.hasOtp === true,
    otpFilled: filled.otpFilled,
  };
  await deps.session.set(lastFilledKey(tabId), record);
}

/** Steps 6 to 9: choose the item, ask the desktop, deliver to the frame, drop every reference. */
async function fillLogin(deps: FillDeps, req: FillRequest): Promise<FillSummary> {
  const p = await prepare(deps, req);
  const item = req.item ?? (await pickItem(deps, req, p));
  const want = wantFor(p.target.counts, item.hasOtp, req.otpOnly);
  if (!want.login && !want.otp) {
    throw new ExtensionError("noFields");
  }
  const credential = await deps.native.request("fill", {
    vaultId: item.vaultId,
    itemId: item.itemId,
    origin: p.target.origin,
    topOrigin: p.topOrigin,
    frame: p.frame,
    want,
  });
  const message = {
    type: "fill" as const,
    fillId: newFillId(),
    expectedOrigin: p.target.origin,
    username: credential.username,
    password: credential.password,
    otp: credential.otp,
  };
  Object.assign(credential, { username: null, password: null, otp: null });
  let reply: FillReply;
  try {
    reply = await deps.tabs.send(req.tabId, frameOf(p), message);
  } finally {
    Object.assign(message, { username: null, password: null, otp: null });
  }
  if (!reply.ok) {
    throw new ExtensionError(reply.error);
  }
  await remember(deps, req.tabId, p.target.origin, { ...item, otpFilled: reply.filled.otp > 0 });
  return reply.filled;
}

async function setFailure(deps: FillDeps, req: FillRequest, error: ErrorCode): Promise<void> {
  await deps.session.set(lastErrorKey(req.tabId), error);
  if (req.gesture !== "popup") {
    await deps.badge.set(req.tabId, "!");
    await deps.badge.openPopup();
  }
}

/**
 * Runs a fill for a gesture and reports what happened. A failure from the shortcut or the
 * menu is shown through the badge and the popup, since nothing else may draw on the page.
 */
export async function runFill(deps: FillDeps, req: FillRequest): Promise<FillOutcome> {
  return runGesture(deps, req, () => fillLogin(deps, req));
}

export async function runGesture(
  deps: FillDeps,
  req: FillRequest,
  action: () => Promise<FillSummary>,
): Promise<FillOutcome> {
  try {
    const summary = await action();
    await deps.session.remove(lastErrorKey(req.tabId));
    await deps.badge.clear(req.tabId);
    return { ok: true, summary };
  } catch (error) {
    const code = errorCodeOf(error);
    await setFailure(deps, req, code);
    return { ok: false, error: code };
  }
}

/** A password the extension generated, held in memory for two minutes for a following fill. */
export class GeneratedHold {
  #value: string | null = null;
  #expires = 0;
  readonly #now: () => number;

  constructor(now: () => number) {
    this.#now = now;
  }

  hold(value: string): void {
    this.#value = value;
    this.#expires = this.#now() + 120_000;
  }

  take(): string | null {
    const value = this.#now() < this.#expires ? this.#value : null;
    this.#value = null;
    return value;
  }
}

async function generateFor(deps: FillDeps, p: Prepared, hold: GeneratedHold): Promise<string> {
  const held = hold.take();
  if (held !== null) {
    return held;
  }
  const host = new URL(p.target.origin).hostname;
  const generated = await deps.native.request("generate", {
    request: { kind: "forSite", host, rules: p.target.passwordRules },
  });
  return generated.value;
}

/** The context-menu "generate" item and the popup's "Fill": a new password into the sign-up form. */
export async function fillGeneratedPassword(
  deps: FillDeps,
  req: FillRequest,
  hold: GeneratedHold,
): Promise<FillOutcome> {
  return runGesture(deps, req, async () => {
    const p = await prepare(deps, req);
    if (p.target.counts.newPassword === 0) {
      throw new ExtensionError("noFields");
    }
    const message = {
      type: "fillGenerated" as const,
      fillId: newFillId(),
      expectedOrigin: p.target.origin,
      password: await generateFor(deps, p, hold),
    };
    let reply: FillReply;
    try {
      reply = await deps.tabs.send(req.tabId, frameOf(p), message);
    } finally {
      Object.assign(message, { password: null });
    }
    if (!reply.ok) {
      throw new ExtensionError(reply.error);
    }
    return reply.filled;
  });
}
