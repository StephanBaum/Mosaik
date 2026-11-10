/**
 * The content script. It is injected on demand (after a click in the popup, the shortcut or
 * the context menu), reports what fields a frame has, and fills one credential that the
 * background hands it. It adds nothing to the page, listens to no page message, and keeps no
 * secret: values live in locals of one call and the message's fields are set to null after.
 */
import type { ErrorCode } from "../shared/errors.ts";
import {
  isContentMessage,
  type ContentMessage,
  type FillReply,
  type ProbeResult,
} from "../shared/messages.ts";
import { classifyChangeForm, sameClassification, type ChangeClassification } from "./change.ts";
import { countFields, detect, hasActiveField, type Detection } from "./detect.ts";
import { fillChangeFields, fillCredentials, fillNewPasswords } from "./fill.ts";
import { frameAncestry } from "./frame.ts";

const MAX_RULES_LENGTH = 4096;

/** The `passwordrules` attribute of the new-password field, if the page sets one. */
function passwordRulesOf(detection: Detection): string | null {
  const rules = detection.newPassword[0]?.getAttribute("passwordrules") ?? null;
  return rules !== null && rules.length <= MAX_RULES_LENGTH ? rules : null;
}

/**
 * The document's own origin: "null" when it is opaque (a CSP `sandbox` header, a sandboxed
 * frame), where `location.origin` would still name the site of the URL.
 */
const documentOrigin = (): string => window.origin;

/**
 * What the last probe made of the change form. A change fill goes only to the form (and the very
 * fields) the probe classified, so a page that rearranged itself between the probe and the fill
 * (swapped `autocomplete` values, reordered or renamed fields) gets nothing. No secret is held.
 */
let probedChange: ChangeClassification | null = null;

export function probe(): ProbeResult {
  const ancestry = frameAncestry(window);
  const detection = detect(document);
  const change = classifyChangeForm(document);
  probedChange = change;
  return {
    origin: documentOrigin(),
    ancestorsSameOrigin: ancestry.ancestorsSameOrigin,
    frameVisible: ancestry.frameVisible,
    depth: ancestry.depth,
    activeField: hasActiveField(detection),
    counts: countFields(detection),
    passwordRules: passwordRulesOf(detection),
    visible: document.visibilityState === "visible",
    changeShape: change.shape,
  };
}

/** The checks that must hold again at fill time: the page may have changed since the probe. */
function refusal(expectedOrigin: string): ErrorCode | null {
  if (documentOrigin() !== expectedOrigin) {
    return "originChanged";
  }
  if (document.visibilityState !== "visible") {
    return "pageHidden";
  }
  if (window !== window.top) {
    const ancestry = frameAncestry(window);
    if (!ancestry.ancestorsSameOrigin || !ancestry.frameVisible) {
      return "crossOriginFrame";
    }
  }
  return null;
}

/** The text of the first username field, for a save that follows a generated password. */
function usernameValueOf(detection: Detection): string | null {
  const value = detection.username[0]?.value;
  return value === undefined || value === "" ? null : value;
}

/** A change fill: both values of one login, into the form the probe classified, only if it is unchanged. */
function handleFillChange(message: Extract<ContentMessage, { type: "fillChange" }>): FillReply {
  const probed = probedChange;
  probedChange = null;
  const now = classifyChangeForm(document);
  let reply: FillReply;
  if (now.shape === "none") {
    reply = { ok: false, error: "changeFormNotFound" };
  } else if (
    now.shape === "ambiguous" ||
    probed === null ||
    now.shape !== message.shape ||
    !sameClassification(probed, now)
  ) {
    reply = { ok: false, error: "changeFormAmbiguous" };
  } else {
    const filled = fillChangeFields(now, { current: message.current, next: message.new });
    reply = { ok: true, filled, usernameValue: null };
  }
  Object.assign(message, { current: null, new: "" });
  return reply;
}

export function handleFill(message: ContentMessage): FillReply {
  const refused = refusal(message.expectedOrigin);
  if (refused !== null) {
    Object.assign(message, { username: null, password: null, otp: null, current: null, new: "" });
    return { ok: false, error: refused };
  }
  if (message.type === "fillChange") {
    return handleFillChange(message);
  }
  // Fields are found again now: ones that changed type, were renamed or appeared after the
  // probe are classified anew, and only the ones that are fillable now are used.
  const detection = detect(document);
  const filled =
    message.type === "fill"
      ? fillCredentials(detection, message)
      : fillNewPasswords(detection, message.password);
  Object.assign(message, { username: null, password: null, otp: null });
  if (filled.username + filled.password + filled.otp + filled.newPassword === 0) {
    // The fields the probe saw are gone or no longer fillable: report it instead of success.
    return { ok: false, error: "noFields" };
  }
  return {
    ok: true,
    filled,
    usernameValue: message.type === "fillGenerated" ? usernameValueOf(detection) : null,
  };
}

function onMessage(
  message: unknown,
  sender: chrome.runtime.MessageSender,
  sendResponse: (reply: FillReply) => void,
): boolean {
  // Only the extension's own background sends here: no page, no other extension, no tab.
  if (sender.id !== chrome.runtime.id || sender.tab !== undefined || !isContentMessage(message)) {
    return false;
  }
  sendResponse(handleFill(message));
  return false;
}

// An own property only: a page element with id or name "mosaicContent" also shows up as a
// global in this world (window named properties) and must not pass for the installed script.
if (!Object.hasOwn(globalThis, "mosaicContent")) {
  Object.defineProperty(globalThis, "mosaicContent", { value: { probe }, enumerable: false });
  chrome.runtime.onMessage.addListener(onMessage);
}
