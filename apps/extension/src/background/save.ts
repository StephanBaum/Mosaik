/**
 * Save and update prompts (ADR-0038). A password reaches here from the opt-in capture script
 * (a trusted submit in a login or change form) or from the generator (a generated password
 * the extension filled). It waits in `storage.session` for five minutes, bound to its tab and
 * to the page origin, until the user answers in the popup. Saving only asks the desktop,
 * which shows its own confirmation dialog and writes the item itself; nothing here writes to
 * the vault.
 */
import { ExtensionError } from "../shared/errors.ts";
import type { CapturedMessage, PendingSaveInfo, PopupResults, SaveAction } from "../shared/messages.ts";
import { hostOf, isSecureOrigin, originOf, unicodeHostOf } from "../shared/origin.ts";
import type { RequestMap, ResultMap } from "../shared/protocol.ts";
import type { SaveBadge } from "./badge.ts";
import type { CaptureRegistry } from "./capture-registry.ts";
import type { KeyValueStore } from "./db.ts";
import type { GeneratedFill } from "./fill.ts";
import type { SenderInfo } from "./router.ts";
import { PENDING_SAVE_PREFIX, pendingSaveKey, type SessionStore } from "./session.ts";

export const PENDING_SAVE_TTL_MS = 5 * 60_000;
const MAX_USERNAME = 256;
const MAX_PASSWORD = 1024;

export interface PendingSave {
  /** The origin of the frame the password was typed (or generated) in. */
  origin: string;
  /** The tab's origin; the prompt is shown, and sent to the desktop, only for this one. */
  topOrigin: string;
  username: string | null;
  password: string;
  /** `null`: the desktop has not been asked yet. */
  kind: "new" | "update" | null;
  itemTitle: string | null;
  source: "capture" | "generated";
  expiresAt: number;
}

export interface SaveDeps {
  native: {
    request<O extends "checkSave" | "proposeSave">(op: O, body: RequestMap[O]): Promise<ResultMap[O]>;
    ensureSession(): Promise<void>;
    readonly connection: string;
  };
  session: SessionStore;
  /** The extension's IndexedDB: `neverSave:<origin>`. */
  kv: KeyValueStore;
  badge: SaveBadge;
  capture: CaptureRegistry;
  now: () => number;
}

export type SaveResult = PopupResults["savePending"];

const NEVER_SAVE_PREFIX = "neverSave:";
export const neverSaveKey = (origin: string): string => `${NEVER_SAVE_PREFIX}${origin}`;

const isText = (value: unknown, max: number): value is string => typeof value === "string" && value.length <= max;

const isOneOf = (value: unknown, allowed: readonly unknown[]): boolean => allowed.includes(value);

const isOptionalText = (value: unknown, max: number): boolean => value === null || isText(value, max);

function isPendingSave(value: unknown): value is PendingSave {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const v = value as Record<string, unknown>;
  return (
    isText(v["origin"], 2048) &&
    isText(v["topOrigin"], 2048) &&
    isOptionalText(v["username"], MAX_USERNAME) &&
    isText(v["password"], MAX_PASSWORD) &&
    isOneOf(v["kind"], [null, "new", "update"]) &&
    isOptionalText(v["itemTitle"], 256) &&
    isOneOf(v["source"], ["capture", "generated"]) &&
    typeof v["expiresAt"] === "number"
  );
}

interface CaptureSite {
  tabId: number;
  origin: string;
  topOrigin: string;
}

/**
 * Where a capture came from, as the browser reports it (never from the message): the frame's
 * origin, and the tab's. The top frame, or a visible same-origin frame whose ancestors are all
 * same-origin; anything else (cross-origin, opaque) is dropped.
 */
export function captureSite(message: CapturedMessage, sender: SenderInfo): CaptureSite | null {
  const tabId = sender.tab?.id;
  const topOrigin = originOf(sender.tab?.url);
  const origin = sender.origin ?? originOf(sender.url);
  if (tabId === undefined || topOrigin === null || origin === null || origin === "null" || origin !== topOrigin) {
    return null;
  }
  const isTop = (sender.frameId ?? 0) === 0;
  return isTop || message.ancestorsSameOrigin ? { tabId, origin, topOrigin } : null;
}

export class SaveService {
  readonly #deps: SaveDeps;
  readonly #saving = new Set<number>();

  constructor(deps: SaveDeps) {
    this.#deps = deps;
  }

  /** `runtime.onMessage` for a capture the router accepted (sender checked, rate limited). */
  onCaptured = (message: CapturedMessage, sender: SenderInfo): void => {
    void this.capture(message, sender).catch(() => undefined);
  };

  async capture(message: CapturedMessage, sender: SenderInfo): Promise<void> {
    const site = captureSite(message, sender);
    if (site === null || !(await this.offersEnabled()) || (await this.isNeverSave(site.origin))) {
      return;
    }
    await this.#offer(site, message.username, message.newPassword ?? message.password, "capture");
  }

  /** A generated password the extension filled: the same prompt as a captured one. */
  storeGenerated = async (filled: GeneratedFill): Promise<void> => {
    if (await this.isNeverSave(filled.origin)) {
      return;
    }
    const username = filled.username?.trim() ?? "";
    await this.#offer(
      { tabId: filled.tabId, origin: filled.origin, topOrigin: filled.topOrigin },
      username === "" || username.length > MAX_USERNAME ? null : username,
      filled.password,
      "generated",
    );
  };

  async #offer(site: CaptureSite, username: string | null, password: string, source: PendingSave["source"]): Promise<void> {
    const name = username === null ? null : username.trim().slice(0, MAX_USERNAME) || null;
    if (password.length < 1 || password.length > MAX_PASSWORD) {
      return;
    }
    const record: PendingSave = {
      origin: site.origin,
      topOrigin: site.topOrigin,
      username: name,
      password,
      kind: null,
      itemTitle: null,
      source,
      expiresAt: this.#deps.now() + PENDING_SAVE_TTL_MS,
    };
    if (source === "capture" && (await this.#ask(record)) === "unchanged") {
      return;
    }
    await this.#deps.session.set(pendingSaveKey(site.tabId), record);
    await this.#deps.badge.show(site.tabId);
  }

  /**
   * Asks the desktop whether the login is new, an update or already saved, when it is unlocked
   * (`kind` stays null when it cannot answer; the popup asks again when it opens).
   */
  async #ask(record: PendingSave): Promise<PendingSave["kind"] | "unchanged"> {
    const { native } = this.#deps;
    try {
      await native.ensureSession();
      if (native.connection !== "unlocked") {
        return null;
      }
      const checked = await native.request("checkSave", {
        origin: record.origin,
        topOrigin: record.topOrigin,
        username: record.username,
        password: record.password,
      });
      if (checked.kind !== "unchanged") {
        record.kind = checked.kind;
        record.itemTitle = checked.itemTitle;
      }
      return checked.kind;
    } catch {
      return null;
    }
  }

  async #load(tabId: number, topOrigin: string): Promise<PendingSave | null> {
    const stored = await this.#deps.session.get(pendingSaveKey(tabId));
    if (stored === undefined) {
      return null;
    }
    if (!isPendingSave(stored) || stored.expiresAt <= this.#deps.now() || stored.topOrigin !== topOrigin) {
      // Expired, malformed, or the tab is on another site now: the password is dropped, not offered there.
      await this.drop(tabId);
      return null;
    }
    return stored;
  }

  /** What the popup shows about a pending save of this tab, or null. */
  async info(tabId: number, topOrigin: string): Promise<PendingSaveInfo | null> {
    const record = await this.#load(tabId, topOrigin);
    if (record === null) {
      return null;
    }
    if (record.kind === null && (await this.#ask(record)) === "unchanged") {
      await this.drop(tabId);
      return null;
    }
    if (record.kind !== null) {
      await this.#deps.session.set(pendingSaveKey(tabId), record);
    }
    const host = hostOf(record.origin);
    return {
      host,
      unicodeHost: unicodeHostOf(host),
      username: record.username,
      kind: record.kind,
      itemTitle: record.itemTitle,
      source: record.source,
      insecure: !isSecureOrigin(record.origin),
    };
  }

  /** The user's answer in the popup: ask the desktop to save, forget this once, or never offer for the site. */
  async act(tabId: number, topOrigin: string, action: SaveAction): Promise<SaveResult> {
    const record = await this.#load(tabId, topOrigin);
    if (record === null) {
      return { outcome: "dismissed" };
    }
    if (action === "never") {
      await this.#deps.kv.set(neverSaveKey(record.origin), true);
      await this.drop(tabId);
      return { outcome: "never" };
    }
    if (action === "dismiss") {
      await this.drop(tabId);
      return { outcome: "dismissed" };
    }
    return this.#propose(tabId, record);
  }

  async #propose(tabId: number, record: PendingSave): Promise<SaveResult> {
    if (this.#saving.has(tabId)) {
      throw new ExtensionError("busy");
    }
    this.#saving.add(tabId);
    try {
      const answer = await this.#deps.native.request("proposeSave", {
        origin: record.origin,
        topOrigin: record.topOrigin,
        username: record.username,
        password: record.password,
      });
      // Saved, updated or declined in the desktop dialog: the password is not kept either way.
      await this.drop(tabId);
      return { outcome: answer.outcome };
    } finally {
      this.#saving.delete(tabId);
    }
  }

  /** Forgets the pending save of a tab and its badge. */
  async drop(tabId: number): Promise<void> {
    await this.#deps.session.remove(pendingSaveKey(tabId));
    await this.#deps.badge.hide(tabId).catch(() => undefined);
  }

  /** `tabs.onUpdated`: a tab that left the site drops what it was going to save. */
  async onNavigated(tabId: number, url: string): Promise<void> {
    const stored = await this.#deps.session.get(pendingSaveKey(tabId));
    if (isPendingSave(stored) && stored.topOrigin !== originOf(url)) {
      await this.drop(tabId);
    }
  }

  /** The sites saving is never offered for (M4-R10), as exact origins. */
  async neverSaveList(): Promise<string[]> {
    const keys = await this.#deps.kv.keys(NEVER_SAVE_PREFIX);
    return keys.map((key) => key.slice(NEVER_SAVE_PREFIX.length));
  }

  /** Takes one site off the list (only the popup port reaches this) and returns what is left. */
  async neverSaveRemove(origin: string): Promise<string[]> {
    await this.#deps.kv.delete(neverSaveKey(origin));
    return this.neverSaveList();
  }

  async isNeverSave(origin: string): Promise<boolean> {
    return (await this.#deps.kv.get(neverSaveKey(origin))) === true;
  }

  /** Whether capture is on: registered, and the site access is still granted. */
  async offersEnabled(): Promise<boolean> {
    const { capture } = this.#deps;
    if (!(await capture.isRegistered())) {
      return false;
    }
    if (await capture.hasAccess()) {
      return true;
    }
    // The browser took the permission back: the script cannot run, so the setting is off.
    await this.setOffers(false);
    return false;
  }

  /** The popup switch. Turning on needs the site access the popup just asked the user for. */
  async setOffers(enabled: boolean): Promise<boolean> {
    const { capture } = this.#deps;
    if (!enabled) {
      await capture.unregister();
      await this.#dropCaptured();
      return false;
    }
    if (!(await capture.hasAccess())) {
      throw new ExtensionError("permissionDenied");
    }
    if (!(await capture.isRegistered())) {
      await capture.register();
    }
    return true;
  }

  async #dropCaptured(): Promise<void> {
    for (const key of await this.#deps.session.keys(PENDING_SAVE_PREFIX)) {
      const stored = await this.#deps.session.get(key);
      if (!isPendingSave(stored) || stored.source === "capture") {
        await this.drop(Number(key.slice(PENDING_SAVE_PREFIX.length)));
      }
    }
  }
}
