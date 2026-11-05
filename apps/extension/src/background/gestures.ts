/**
 * The trusted gestures that are not a click in the popup: the `fill-login` shortcut and the
 * two context-menu items. The browser delivers them only for a real key press or click in its
 * own UI, never for events a page dispatches. No inline UI exists in M4 (ADR-0035).
 */
import { COMMAND_FILL } from "../shared/ids.ts";
import { fillGeneratedPassword, runFill, type FillDeps, type FillOutcome, type FillRequest, type GeneratedHold } from "./fill.ts";

export { COMMAND_FILL };
export const MENU_FILL = "fill";
export const MENU_GENERATE = "generate";

export interface GestureDeps {
  fill: FillDeps;
  hold: GeneratedHold;
  /** The tab in front, for a shortcut that arrives without one. */
  activeTab: () => Promise<{ id: number; url: string | undefined } | null>;
}

interface TabLike {
  id?: number | undefined;
  url?: string | undefined;
}

function request(gesture: "command" | "menu", tab: { id: number; url: string | undefined }, menuFrameId: number | null): FillRequest {
  return {
    gesture,
    tabId: tab.id,
    tabUrl: tab.url,
    menuFrameId,
    item: null,
    otpOnly: false,
    confirmFrame: null,
  };
}

/** `commands.onCommand`: fills the first matching login in the front tab. */
export async function onCommand(deps: GestureDeps, command: string, tab: TabLike | undefined): Promise<FillOutcome | null> {
  if (command !== COMMAND_FILL) {
    return null;
  }
  const target = tab?.id === undefined ? await deps.activeTab() : { id: tab.id, url: tab.url };
  return target === null ? null : runFill(deps.fill, request("command", target, null));
}

/** `contextMenus.onClicked`: "Fill login" or "Generate password" for the field under the cursor. */
export async function onMenuClicked(
  deps: GestureDeps,
  info: { menuItemId: string | number; frameId?: number | undefined },
  tab: TabLike | undefined,
): Promise<FillOutcome | null> {
  if (tab?.id === undefined || (info.menuItemId !== MENU_FILL && info.menuItemId !== MENU_GENERATE)) {
    return null;
  }
  const req = request("menu", { id: tab.id, url: tab.url }, info.frameId ?? 0);
  return info.menuItemId === MENU_FILL
    ? runFill(deps.fill, req)
    : fillGeneratedPassword(deps.fill, req, deps.hold);
}

/** `runtime.onInstalled`: the two menu items, shown on editable fields. */
export async function createMenus(): Promise<void> {
  await chrome.contextMenus.removeAll();
  chrome.contextMenus.create({ id: MENU_FILL, title: chrome.i18n.getMessage("commandFill"), contexts: ["editable"] });
  chrome.contextMenus.create({
    id: MENU_GENERATE,
    title: chrome.i18n.getMessage("menuGenerate"),
    contexts: ["editable"],
  });
}
