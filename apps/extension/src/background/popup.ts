/**
 * What the popup may ask the background to do (the `PopupRequest` operations). The router has
 * already checked who is asking; here each operation runs against the active tab.
 */
import { ExtensionError, isErrorCode } from "../shared/errors.ts";
import type {
  FillSummary,
  FrameProbe,
  FrameRef,
  PopupRequest,
  PopupResults,
  PopupState,
  SaveAction,
  TabInfo,
} from "../shared/messages.ts";
import { hostOf, isSecureOrigin, originOf, unicodeHostOf } from "../shared/origin.ts";
import type { GenerateRequest, LoginMatch } from "../shared/protocol.ts";
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
import type { PasskeySwitch } from "./passkey-registry.ts";
import { chooseChangeTarget, type RotationService } from "./rotation.ts";
import type { SaveService } from "./save.ts";
import { lastErrorKey, lastFilledKey } from "./session.ts";

export interface TabRef {
  id: number;
  url: string | undefined;
}

export interface PopupDeps {
  native: NativeClient;
  fill: FillDeps;
  hold: GeneratedHold;
  save: SaveService;
  passkeys: PasskeySwitch;
  rotation: RotationService;
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

/** A failed shortcut or menu fill is shown once: opening the popup is the user having seen it. */
async function takeLastError(deps: PopupDeps, tabId: number): Promise<PopupState["lastError"]> {
  const stored = await deps.fill.session.get(lastErrorKey(tabId));
  if (stored !== undefined) {
    await deps.fill.session.remove(lastErrorKey(tabId));
    await deps.fill.badge.clear(tabId);
  }
  return isErrorCode(stored) ? stored : null;
}

type Operations = {
  readonly [K in PopupRequest["op"]]: (request: Extract<PopupRequest, { op: K }>, tab: TabRef | null) => Promise<unknown>;
};

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
    const run = this.#operations[request.op] as (request: PopupRequest, tab: TabRef | null) => Promise<unknown>;
    return run(request, tab);
  }

  readonly #operations: Operations = {
    getState: (_request, tab) => this.#state(tab),
    pair: async () => {
      await this.#deps.native.pair();
      return {};
    },
    focusDesktop: () => this.#deps.native.request("focus", {}),
    search: (request, tab) => this.#search(request.query, tab),
    fill: (request, tab) => this.#fill(false, request, tab),
    fillOtp: (request, tab) => this.#fill(true, request, tab),
    copy: (request) => this.#deps.native.request("copy", { source: request.source }),
    generate: (request, tab) => this.#generate(request.request, tab),
    fillGenerated: async (request, tab) =>
      finish(
        await fillGeneratedPassword(
          this.#deps.fill,
          fillRequest(tab, { confirmFrame: request.confirmFrame, requireHeld: true }),
          this.#deps.hold,
        ),
      ),
    savePending: (request, tab) => this.#savePending(request.action, tab),
    saveOffersChanged: async (request) => ({ saveOffers: await this.#deps.save.setOffers(request.enabled) }),
    setPasskeys: async (request) => ({ passkeys: await this.#deps.passkeys.set(request.enabled) }),
    rotationBegin: (request, tab) => this.#deps.rotation.begin(request, rotationTab(tab)),
    rotationOpenPage: (request, tab) => this.#deps.rotation.openPage(request, rotationTab(tab)),
    fillChange: (request, tab) => this.#deps.rotation.fillChange(request, rotationTab(tab)),
    fillPending: (request, tab) => this.#deps.rotation.fillPending(request, rotationTab(tab)),
    rotationResult: (request, tab) => this.#deps.rotation.rotationResult(request, rotationTab(tab)),
    rotationAddOrigin: (_request, tab) => this.#deps.rotation.addOrigin(rotationTab(tab)),
    unpair: async () => {
      await this.#deps.native.unpair();
      return {};
    },
  };

  async #fill(
    otpOnly: boolean,
    request: { vaultId: string; itemId: string; confirmFrame: FrameRef | null },
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

  /**
   * A password from the desktop, held for a following fill on this tab and origin only. For
   * "this site" the host and the page's password rules come from the tab, never from the popup.
   */
  async #generate(request: GenerateRequest, tab: TabRef | null): Promise<unknown> {
    const origin = originOf(tab?.url);
    if (tab === null || origin === null) {
      throw new ExtensionError("unsupportedPage");
    }
    const body: GenerateRequest =
      request.kind === "forSite"
        ? { kind: "forSite", host: hostOf(origin), rules: chooseTarget(await probeFrames(this.#deps, tab), null)?.passwordRules ?? null }
        : request;
    const generated = await this.#deps.native.request("generate", { request: body });
    this.#deps.hold.hold(generated.value, { tabId: tab.id, origin });
    return generated;
  }

  async #savePending(action: SaveAction, tab: TabRef | null): Promise<PopupResults["savePending"]> {
    const origin = originOf(tab?.url);
    if (tab === null || origin === null) {
      return { outcome: "dismissed" };
    }
    return this.#deps.save.act(tab.id, origin, action);
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
      frameTarget: null,
      changeFrame: null,
      matches: [],
      warnings: [],
      pendingSave: null,
      nextOtp: null,
      lastError: null,
      saveOffers: false,
      passkeys: false,
      generatorFill: false,
      rotations: [],
      rotationTabOffer: null,
    };
    state.saveOffers = await this.#deps.save.offersEnabled().catch(() => false);
    state.passkeys = await this.#deps.passkeys.enabled().catch(() => false);
    if (tab !== null && info !== null) {
      await this.#fillTab(state, tab, info);
      state.pendingSave = await this.#deps.save.info(tab.id, info.origin).catch(() => null);
    }
    return state;
  }

  /** What the popup shows about the front tab: its frame, matches, warnings and a pending code. */
  async #fillTab(state: PopupState, tab: TabRef, info: TabInfo): Promise<void> {
    const { native } = this.#deps;
    state.lastError = await takeLastError(this.#deps, tab.id);
    const probes = await probeFrames(this.#deps, tab);
    const target = chooseTarget(probes, null);
    const origin = target?.origin ?? info.origin;
    state.frame = classifyFrame(target, info.origin);
    state.frameTarget = target === null ? null : { frameId: target.frameId, documentId: target.documentId };
    state.generatorFill = (target?.counts.newPassword ?? 0) > 0;
    state.nextOtp = await nextOtpOf(this.#deps, tab, origin);
    if (state.connection === "unlocked") {
      const listing = await native.request("listForOrigin", { origin, topOrigin: info.origin }).catch(() => null);
      state.matches = listing?.items ?? [];
      state.warnings = listing?.warnings ?? [];
      this.#learn(state.matches);
      Object.assign(state, await this.#deps.rotation.forTab(tab).catch(() => ({ rotations: [], rotationTabOffer: null })));
      if (state.rotations.length > 0) {
        state.changeFrame = changeFrameOf(probes, info.origin);
      }
    }
  }
}

/** The frame a change fill would go to, for the popup's confirmation; null when there is no change form to pick. */
function changeFrameOf(probes: readonly FrameProbe[], topOrigin: string): PopupState["changeFrame"] {
  try {
    const target = chooseChangeTarget(probes);
    return { frame: classifyFrame(target, topOrigin), target: { frameId: target.frameId, documentId: target.documentId } };
  } catch {
    return null;
  }
}

/** The tab a rotation operation is about: there must be one (the popup is open on it). */
function rotationTab(tab: TabRef | null): TabRef {
  if (tab === null) {
    throw new ExtensionError("unsupportedPage");
  }
  return tab;
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
    confirmFrame: null,
    ...extra,
  };
}
