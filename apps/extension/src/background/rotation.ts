/**
 * The guided password change on a site (docs/EXTENSION_SECURITY.md, "password change").
 *
 * - Everything here starts from a click in the popup. Opening the change page injects nothing, and
 *   no change fill runs on page load, on a shortcut or on a page's request (OD-M7b-8).
 * - The **probe's classification decides before the desktop is asked** for a secret: a page with
 *   no change form (`changeFormNotFound`) or one that cannot be told apart (`changeFormAmbiguous`)
 *   never gets a secret requested.
 * - A change fill sends **one** message with the two values of **one** login to one frame and one
 *   document, then drops every reference (ADR-0068, brief section 4.5: one credential per content
 *   script).
 * - Starting a change, reporting its result and adding a related address only ask the desktop,
 *   which opens a dialog; the extension never writes. The new password is never shown here.
 * - `rotationTab:<tabId>` (in `storage.session`) names the login and the origin of a tab the
 *   extension sent to a change page. Only in such a tab, and only on the same site (a registrable
 *   domain, https), the popup offers to add the address it ended up on.
 */
import { ExtensionError } from "../shared/errors.ts";
import { newFillId } from "../shared/ids.ts";
import type {
  FillReply,
  FillSummary,
  FrameProbe,
  FrameRef,
  PopupState,
  RotationTabOffer,
} from "../shared/messages.ts";
import { hostOf, isSecureOrigin, originOf, sameSite, unicodeHostOf } from "../shared/origin.ts";
import type { RequestMap, ResultMap, RotationMatch } from "../shared/protocol.ts";
import { chooseTarget, decideFrame, wantFor } from "./frames.ts";
import { fillHooks, isConfirmed } from "./fill.ts";
import { rotationTabKey, type SessionStore } from "./session.ts";
import type { FrameTarget, TabApi } from "./tabs.ts";

type RotationOp =
  | "rotationForOrigin"
  | "rotationBegin"
  | "fillChange"
  | "fillPending"
  | "rotationResult"
  | "rotationAddOrigin";

export interface RotationDeps {
  native: {
    request<O extends RotationOp>(op: O, body: RequestMap[O]): Promise<ResultMap[O]>;
  };
  tabs: TabApi;
  session: SessionStore;
  now: () => number;
}

/** A tab the extension sent to a change page: what the popup needs to offer a related address. */
export interface RotationTabRecord {
  vaultId: string;
  itemId: string;
  attemptId: string;
  /** The origin the change page was opened from (the login's own). */
  fromOrigin: string;
  title: string | null;
  at: number;
}

/** A record is kept for this long, then forgotten. */
export const ROTATION_TAB_TTL_MS = 15 * 60_000;

export interface RotationTab {
  id: number;
  url: string | undefined;
}

export interface AttemptRef {
  vaultId: string;
  itemId: string;
  attemptId: string;
}

function isRecord(value: unknown): value is RotationTabRecord {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const r = value as Record<string, unknown>;
  return (
    typeof r["vaultId"] === "string" &&
    typeof r["itemId"] === "string" &&
    typeof r["attemptId"] === "string" &&
    typeof r["fromOrigin"] === "string" &&
    typeof r["at"] === "number"
  );
}

/** The https origin of a tab, or `unsupportedPage`: the change flow never runs on http. */
function httpsOrigin(tab: RotationTab): string {
  const origin = originOf(tab.url);
  if (origin === null || !isSecureOrigin(origin)) {
    throw new ExtensionError("insecureOrigin");
  }
  return origin;
}

/** The change page to open: the desktop's, if it stays on the tab's site over https, else the well-known one. */
export function changePageFor(tabOrigin: string, changeUrl: string | null): string {
  if (changeUrl !== null) {
    try {
      const url = new URL(changeUrl);
      if (url.protocol === "https:" && sameSite(url.origin, tabOrigin)) {
        return url.href;
      }
    } catch {
      // Not a URL: the well-known address is used.
    }
  }
  return `${tabOrigin}/.well-known/change-password`;
}

/**
 * The frame a change fill goes to: the top frame if it has a change form, else the one frame that
 * has one, else the deepest with focus; several candidates and no focus are `changeFormAmbiguous`.
 */
export function chooseChangeTarget(probes: readonly FrameProbe[]): FrameProbe {
  const candidates = probes.filter((p) => p.changeShape !== "none");
  const top = candidates.find((p) => p.frameId === 0);
  if (top !== undefined) {
    return top;
  }
  const [only, ...more] = candidates;
  if (only === undefined) {
    throw new ExtensionError("changeFormNotFound");
  }
  if (more.length === 0) {
    return only;
  }
  const focused = candidates.filter((p) => p.activeField);
  const [one, ...others] = focused;
  if (one === undefined || others.length > 0) {
    throw new ExtensionError("changeFormAmbiguous");
  }
  return one;
}

const frameOf = (target: FrameProbe): FrameTarget => ({ frameId: target.frameId, documentId: target.documentId });

export class RotationService {
  readonly #deps: RotationDeps;

  constructor(deps: RotationDeps) {
    this.#deps = deps;
  }

  /** The open attempts for this tab and the related address to offer, for the popup state. */
  async forTab(tab: RotationTab): Promise<Pick<PopupState, "rotations" | "rotationTabOffer">> {
    const origin = originOf(tab.url);
    if (origin === null || !isSecureOrigin(origin)) {
      return { rotations: [], rotationTabOffer: null };
    }
    // An older desktop does not know the op (`invalidRequest`): no rotation controls then.
    const listing = await this.#deps.native
      .request("rotationForOrigin", { origin, topOrigin: origin })
      .catch(() => ({ attempts: [] as RotationMatch[] }));
    return { rotations: listing.attempts, rotationTabOffer: await this.#offer(tab, origin, listing.attempts) };
  }

  async #offer(tab: RotationTab, origin: string, open: readonly RotationMatch[]): Promise<RotationTabOffer | null> {
    const record = await this.#record(tab.id);
    if (record === null) {
      return null;
    }
    const stillSite = sameSite(origin, record.fromOrigin);
    if (!stillSite) {
      await this.#deps.session.remove(rotationTabKey(tab.id));
      return null;
    }
    if (origin === record.fromOrigin || open.some((m) => m.attemptId === record.attemptId)) {
      return null;
    }
    const host = hostOf(origin);
    return { host, unicodeHost: unicodeHostOf(host), title: record.title };
  }

  /** The tab's record while it is fresh; an expired one is forgotten. */
  async #record(tabId: number): Promise<RotationTabRecord | null> {
    const stored = await this.#deps.session.get(rotationTabKey(tabId));
    if (!isRecord(stored)) {
      return null;
    }
    if (this.#deps.now() - stored.at >= ROTATION_TAB_TTL_MS) {
      await this.#deps.session.remove(rotationTabKey(tabId));
      return null;
    }
    return stored;
  }

  async forgetTab(tabId: number): Promise<void> {
    await this.#deps.session.remove(rotationTabKey(tabId));
  }

  /** "Change password…": the desktop shows the generated password and asks; nothing is stored before. */
  async begin(request: { vaultId: string; itemId: string }, tab: RotationTab): Promise<ResultMap["rotationBegin"]> {
    const origin = httpsOrigin(tab);
    await this.#deps.tabs.inject(tab.id);
    const top = (await this.#deps.tabs.probe(tab.id)).find((p) => p.frameId === 0);
    const rules = top !== undefined && top.origin === origin ? top.passwordRules : null;
    return this.#deps.native.request("rotationBegin", {
      vaultId: request.vaultId,
      itemId: request.itemId,
      origin,
      topOrigin: origin,
      rules,
    });
  }

  /**
   * "Open change page": the tab goes to the desktop's validated address, or to the well-known one;
   * the tab is remembered. Nothing is injected.
   */
  async openPage(request: AttemptRef, tab: RotationTab): Promise<Record<string, never>> {
    const origin = httpsOrigin(tab);
    const { attempts } = await this.#deps.native.request("rotationForOrigin", { origin, topOrigin: origin });
    const match = attempts.find(
      (a) => a.vaultId === request.vaultId && a.itemId === request.itemId && a.attemptId === request.attemptId,
    );
    if (match === undefined) {
      throw new ExtensionError("rotationNotFound");
    }
    await this.#deps.tabs.navigate(tab.id, changePageFor(origin, match.changeUrl));
    const record: RotationTabRecord = {
      vaultId: match.vaultId,
      itemId: match.itemId,
      attemptId: match.attemptId,
      fromOrigin: origin,
      title: match.title,
      at: this.#deps.now(),
    };
    await this.#deps.session.set(rotationTabKey(tab.id), record);
    return {};
  }

  /** "Fill change form": the current password into the current field, the new into the new ones. Never submits. */
  async fillChange(request: AttemptRef & { confirmFrame: FrameRef | null }, tab: RotationTab): Promise<FillSummary> {
    const topOrigin = originOf(tab.url);
    if (topOrigin === null) {
      throw new ExtensionError("unsupportedPage");
    }
    await this.#deps.tabs.inject(tab.id);
    const probes = await this.#deps.tabs.probe(tab.id);
    await fillHooks.afterProbe?.();
    const target = chooseChangeTarget(probes);
    const decision = decideFrame(target, topOrigin, isConfirmed(request.confirmFrame, target));
    if (!decision.ok) {
      throw new ExtensionError(decision.error);
    }
    // The classification decides before the desktop is asked: no secret is requested for a form
    // that is not a change form or that cannot be told apart.
    if (target.changeShape === "ambiguous") {
      throw new ExtensionError("changeFormAmbiguous");
    }
    if (target.changeShape === "none") {
      throw new ExtensionError("changeFormNotFound");
    }
    const values = await this.#deps.native.request("fillChange", {
      vaultId: request.vaultId,
      itemId: request.itemId,
      attemptId: request.attemptId,
      origin: target.origin,
      topOrigin,
      frame: decision.frame,
      shape: target.changeShape,
    });
    const message = {
      type: "fillChange" as const,
      fillId: newFillId(),
      expectedOrigin: target.origin,
      shape: target.changeShape,
      current: values.current,
      new: values.new,
    };
    Object.assign(values, { current: null, new: "" });
    let reply: FillReply;
    try {
      reply = await this.#deps.tabs.send(tab.id, frameOf(target), message);
    } finally {
      Object.assign(message, { current: null, new: "" });
    }
    if (!reply.ok) {
      throw new ExtensionError(reply.error);
    }
    return reply.filled;
  }

  /**
   * "Fill new password to sign in": after a change the site accepted but the user has not
   * confirmed, the sign-in form gets the user name and the new password.
   */
  async fillPending(request: AttemptRef & { confirmFrame: FrameRef | null }, tab: RotationTab): Promise<FillSummary> {
    const topOrigin = originOf(tab.url);
    if (topOrigin === null) {
      throw new ExtensionError("unsupportedPage");
    }
    await this.#deps.tabs.inject(tab.id);
    const probes = await this.#deps.tabs.probe(tab.id);
    await fillHooks.afterProbe?.();
    const target = chooseTarget(probes, null);
    if (target === null) {
      throw new ExtensionError("noFields");
    }
    const decision = decideFrame(target, topOrigin, isConfirmed(request.confirmFrame, target));
    if (!decision.ok) {
      throw new ExtensionError(decision.error);
    }
    if (!wantFor(target.counts, null, false).login) {
      throw new ExtensionError("noFields");
    }
    const values = await this.#deps.native.request("fillPending", {
      vaultId: request.vaultId,
      itemId: request.itemId,
      attemptId: request.attemptId,
      origin: target.origin,
      topOrigin,
      frame: decision.frame,
    });
    const message = {
      type: "fill" as const,
      fillId: newFillId(),
      expectedOrigin: target.origin,
      username: values.username,
      password: values.password as string | null,
      otp: null,
    };
    Object.assign(values, { username: null, password: "" });
    let reply: FillReply;
    try {
      reply = await this.#deps.tabs.send(tab.id, frameOf(target), message);
    } finally {
      Object.assign(message, { username: null, password: null });
    }
    if (!reply.ok) {
      throw new ExtensionError(reply.error);
    }
    return reply.filled;
  }

  /** "It worked" or "It didn't work": the desktop dialog commits (or not). */
  async rotationResult(
    request: AttemptRef & { outcome: "succeeded" | "failed" },
    tab: RotationTab,
  ): Promise<ResultMap["rotationResult"]> {
    const origin = httpsOrigin(tab);
    return await this.#deps.native.request("rotationResult", {
      vaultId: request.vaultId,
      itemId: request.itemId,
      attemptId: request.attemptId,
      origin,
      topOrigin: origin,
      outcome: request.outcome,
    });
  }

  /**
   * "Add it": only in a tab this extension sent to a change page, from the login's own origin,
   * and only on the same site. The desktop decides again (registrable domain, https, dialog).
   */
  async addOrigin(tab: RotationTab): Promise<ResultMap["rotationAddOrigin"]> {
    const origin = httpsOrigin(tab);
    const record = await this.#record(tab.id);
    if (record === null || origin === record.fromOrigin || !sameSite(origin, record.fromOrigin)) {
      throw new ExtensionError("originNotRelated");
    }
    return this.#deps.native.request("rotationAddOrigin", {
      vaultId: record.vaultId,
      itemId: record.itemId,
      attemptId: record.attemptId,
      fromOrigin: record.fromOrigin,
      origin,
      topOrigin: origin,
    });
  }
}
