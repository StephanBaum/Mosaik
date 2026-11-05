/**
 * Which frame gets a credential, and whether it may. Pure policy, no browser APIs: the
 * rules of docs/EXTENSION_SECURITY.md ("frames") in one place, covered by `frames.test.ts`.
 */
import type { ErrorCode } from "../shared/errors.ts";
import type { FrameClass, FrameProbe, ProbeResult } from "../shared/messages.ts";
import type { FillWant } from "../shared/protocol.ts";

export type FrameFacts = Pick<
  FrameProbe,
  "frameId" | "origin" | "ancestorsSameOrigin" | "frameVisible" | "visible"
>;

export type FrameDecision =
  | { ok: true; frame: "top" | "sameOriginFrame" }
  | { ok: false; error: ErrorCode };

function isHttpOrigin(origin: string): boolean {
  return origin.startsWith("https://") || origin.startsWith("http://");
}

/**
 * Cross-origin and opaque frames are never filled; a same-origin frame (with same-origin
 * ancestry, visible) only after the user confirmed it in the popup.
 */
export function decideFrame(target: FrameFacts, topOrigin: string, confirmFrame: boolean): FrameDecision {
  if (target.origin === "null" || !isHttpOrigin(target.origin)) {
    return { ok: false, error: "frameOpaque" };
  }
  if (!target.visible) {
    return { ok: false, error: "pageHidden" };
  }
  if (target.frameId === 0) {
    return target.origin === topOrigin ? { ok: true, frame: "top" } : { ok: false, error: "originChanged" };
  }
  if (target.origin !== topOrigin || !target.ancestorsSameOrigin || !target.frameVisible) {
    return { ok: false, error: "crossOriginFrame" };
  }
  return confirmFrame ? { ok: true, frame: "sameOriginFrame" } : { ok: false, error: "frameNeedsConfirmation" };
}

function hasFields(probe: ProbeResult): boolean {
  const { username, password, newPassword, otp } = probe.counts;
  return username + password + newPassword + otp > 0;
}

/**
 * The frame to fill: the one the user clicked (context menu), else the deepest frame with a
 * focused field, else the top frame if it has fields, else the only frame that has any.
 */
export function chooseTarget(probes: readonly FrameProbe[], menuFrameId: number | null): FrameProbe | null {
  if (menuFrameId !== null) {
    return probes.find((p) => p.frameId === menuFrameId) ?? null;
  }
  const focused = probes.filter((p) => p.activeField);
  const deepest = focused.reduce<FrameProbe | null>((best, p) => (best === null || p.depth >= best.depth ? p : best), null);
  if (deepest !== null) {
    return deepest;
  }
  const top = probes.find((p) => p.frameId === 0);
  if (top !== undefined && hasFields(top)) {
    return top;
  }
  const withFields = probes.filter(hasFields);
  return withFields.length === 1 ? (withFields[0] ?? null) : null;
}

/** The frame as the popup shows it. */
export function classifyFrame(target: FrameProbe | null, topOrigin: string): FrameClass {
  if (target === null) {
    return "none";
  }
  if (target.frameId === 0) {
    return "top";
  }
  const same =
    target.origin === topOrigin &&
    target.origin !== "null" &&
    target.ancestorsSameOrigin &&
    target.frameVisible;
  return same ? "sameOriginFrame" : "crossOriginFrame";
}

/** What to ask the desktop for. `hasOtp` is null when the item is not known to have a code. */
export function wantFor(counts: ProbeResult["counts"], hasOtp: boolean | null, otpOnly: boolean): FillWant {
  return {
    // A sign-up form (new-password fields, no current password) gets no login.
    login: !otpOnly && (counts.password > 0 || (counts.username > 0 && counts.newPassword === 0)),
    otp: counts.otp > 0 && hasOtp !== false,
  };
}
