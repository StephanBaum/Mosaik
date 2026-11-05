/**
 * What the popup may ask the background to do (the `PopupRequest` operations). The router has
 * already checked who is asking; here each operation runs against the active tab.
 */
import { ExtensionError, isErrorCode } from "../shared/errors.ts";
import type {
  FillSummary,
  FrameProbe,
  PopupRequest,
  PopupState,
  TabInfo,
} from "../shared/messages.ts";
import { hostOf, isSecureOrigin, originOf, unicodeHostOf } from "../shared/origin.ts";
import type { LoginMatch } from "../shared/protocol.ts";
import { classifyFrame, chooseTarget } from "./frames.ts";
import {
  LAST_FILLED_TTL_MS,
  fillGeneratedPassword,
  runFill,
  type FillDeps,
  type FillOutcome,
  type FillRequest,
  type GeneratedHold,
  type ItemRef,
  type LastFilled,
} from "./fill.ts";
import type { NativeClient } from "./native.ts";
import { lastErrorKey, lastFilledKey } from "./session.ts";

export interface TabRef {
  id: number;
  url: string | undefined;
}

export interface PopupDeps {
  native: NativeClient;
  fill: FillDeps;
  hold: GeneratedHold;
}

const MAX_KNOWN_ITEMS = 200;

function tabInfo(url: string | undefined): TabInfo | null {
  const origin = originOf(url);
  if (origin === null) {
    return null;
  }
  const host = hostOf(origin);
  return { origin, host, unicodeHost: unicodeHostOf(host), secure: isSecureOrigin(origin), supported: true };
}

async function probeFrames(deps: PopupDeps, tab: TabRef): Promise<FrameProbe[]> {
  try {
    await deps.fill.tabs.inject(tab.id);
    return await deps.fill.tabs.probe(tab.id);
  } catch {
    return [];
  }
}

async function nextOtpOf(deps: PopupDeps, tab: TabRef, origin: string): Promise<PopupState["nextOtp"]> {
  const last = (await deps.fill.session.get(lastFilledKey(tab.id))) as LastFilled | undefined;
  const fresh = last !== undefined && last.origin === origin && deps.fill.now() - last.at < LAST_FILLED_TTL_MS;
  return fresh && last.hasOtp && !last.otpFilled
    ? { vaultId: last.vaultId, itemId: last.itemId, title: last.title, username: last.username }
    : null;
}

type Learnable = Pick<LoginMatch, "vaultId" | "itemId" | "title" | "username" | "hasOtp">;

export class PopupService {
  readonly #deps: PopupDeps;
  readonly #pairCode: () => PopupState["pairCode"];
  /** What the popup has listed, so a fill knows the item's title and whether it has a code. */
  readonly #known = new Map<string, ItemRef>();

  constructor(deps: PopupDeps, pairCode: () => PopupState["pairCode"]) {
    this.#deps = deps;
    this.#pairCode = pairCode;
  }

  async handle(request: PopupRequest, tab: TabRef | null): Promise<unknown> {
    const { native, fill, hold } = this.#deps;
    switch (request.op) {
      case "getState":
        return this.#state(tab);
      case "pair":
        await native.pair();
        return {};
      case "focusDesktop":
        return native.request("focus", {});
      case "search":
        return this.#search(request.query, tab);
      case "fill":
      case "fillOtp":
        return this.#fill(request.op === "fillOtp", request, tab);
      case "copy":
        return native.request("copy", { source: request.source });
      case "generate": {
        const generated = await native.request("generate", { request: request.request });
        hold.hold(generated.value);
        return generated;
      }
      case "fillGenerated":
        return finish(
          await fillGeneratedPassword(fill, fillRequest(tab, { confirmFrame: request.confirmFrame }), hold),
        );
      case "unpair":
        await native.unpair();
        return {};
    }
  }

  async #fill(
    otpOnly: boolean,
    request: { vaultId: string; itemId: string; confirmFrame: boolean },
    tab: TabRef | null,
  ): Promise<FillSummary> {
    const item = this.#known.get(`${request.vaultId}/${request.itemId}`) ?? {
      vaultId: request.vaultId,
      itemId: request.itemId,
      title: null,
      username: null,
      hasOtp: null,
    };
    return finish(
      await runFill(this.#deps.fill, fillRequest(tab, { item, otpOnly, confirmFrame: request.confirmFrame })),
    );
  }

  async #search(query: string, tab: TabRef | null): Promise<unknown> {
    const trimmed = query.trim();
    if (trimmed.length === 0) {
      throw new ExtensionError("invalidRequest");
    }
    const found = await this.#deps.native.request("search", { query: trimmed, origin: originOf(tab?.url) });
    this.#learn(found.items);
    return found;
  }

  #learn(items: readonly Learnable[]): void {
    if (this.#known.size > MAX_KNOWN_ITEMS) {
      this.#known.clear();
    }
    for (const { vaultId, itemId, title, username, hasOtp } of items) {
      this.#known.set(`${vaultId}/${itemId}`, { vaultId, itemId, title, username, hasOtp });
    }
  }

  async #state(tab: TabRef | null): Promise<PopupState> {
    const { native } = this.#deps;
    await native.ensureSession().catch(() => undefined);
    const info = tabInfo(tab?.url);
    const state: PopupState = {
      connection: native.connection,
      pairCode: this.#pairCode(),
      tab: info,
      frame: "none",
      matches: [],
      warnings: [],
      pendingSave: null,
      nextOtp: null,
      lastError: null,
      saveOffers: false,
      generatorFill: false,
    };
    if (tab !== null && info !== null) {
      await this.#fillTab(state, tab, info);
    }
    return state;
  }

  /** What the popup shows about the front tab: its frame, matches, warnings and a pending code. */
  async #fillTab(state: PopupState, tab: TabRef, info: TabInfo): Promise<void> {
    const { native, fill } = this.#deps;
    const stored = await fill.session.get(lastErrorKey(tab.id));
    state.lastError = isErrorCode(stored) ? stored : null;
    const target = chooseTarget(await probeFrames(this.#deps, tab), null);
    const origin = target?.origin ?? info.origin;
    state.frame = classifyFrame(target, info.origin);
    state.generatorFill = (target?.counts.newPassword ?? 0) > 0;
    state.nextOtp = await nextOtpOf(this.#deps, tab, origin);
    if (state.connection === "unlocked") {
      const listing = await native.request("listForOrigin", { origin, topOrigin: info.origin }).catch(() => null);
      state.matches = listing?.items ?? [];
      state.warnings = listing?.warnings ?? [];
      this.#learn(state.matches);
    }
  }
}

function finish(outcome: FillOutcome): FillSummary {
  if (!outcome.ok) {
    throw new ExtensionError(outcome.error);
  }
  return outcome.summary;
}

function fillRequest(tab: TabRef | null, extra: Partial<FillRequest>): FillRequest {
  if (tab === null) {
    throw new ExtensionError("unsupportedPage");
  }
  return {
    gesture: "popup",
    tabId: tab.id,
    tabUrl: tab.url,
    menuFrameId: null,
    item: null,
    otpOnly: false,
    confirmFrame: false,
    ...extra,
  };
}
