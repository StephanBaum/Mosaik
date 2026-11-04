/**
 * The phase machine of the reveal (ADR-0032), as a pure reducer so every transition can be
 * tested without a DOM. See the table in the ADR: event-driven locks and every non-vault
 * screen switch instantly; only a sealed in-app lock and a locked-to-vault unlock animate.
 */
import type { SessionState } from "../ipc/contract/index.ts";
import type { SealedDisplay } from "./seal.ts";

export type LockedSession = Extract<SessionState, { state: "locked" }>;

export type RevealTarget =
  | { kind: "locked"; session: LockedSession }
  | { kind: "vault" }
  | { kind: "other"; key: string };

export type Phase = "other" | "idle-locked" | "unlocking" | "idle-unlocked" | "sealing" | "locking";

export interface RevealState {
  key: string;
  phase: Phase;
  locked: LockedSession | null;
  sealed: SealedDisplay | null;
  /** Identifies the current run; a late `done` from an earlier run is ignored. */
  run: number;
}

export type RevealEvent =
  | { type: "target"; target: RevealTarget; animate: boolean }
  | { type: "seal"; sealed: SealedDisplay }
  | { type: "abortSeal" }
  | { type: "done"; run: number };

export const keyOf = (target: RevealTarget): string =>
  target.kind === "other" ? `other:${target.key}` : target.kind;

export function initReveal(target: RevealTarget): RevealState {
  const phase: Phase = target.kind === "other" ? "other" : target.kind === "locked" ? "idle-locked" : "idle-unlocked";
  return { key: keyOf(target), phase, locked: target.kind === "locked" ? target.session : null, sealed: null, run: 0 };
}

function onTarget(state: RevealState, target: RevealTarget, animate: boolean): RevealState {
  const key = keyOf(target);
  const locked = target.kind === "locked" ? target.session : state.locked;
  const base = { ...state, key, locked };
  if (target.kind === "other") {
    return { ...base, phase: "other", sealed: null };
  }
  if (target.kind === "locked") {
    switch (state.phase) {
      case "sealing":
        return { ...base, phase: "locking", run: state.run + 1 };
      case "locking":
      case "idle-locked":
        return base;
      default:
        // A lock the app did not start (idle, sleep, tray), or the first paint: instant.
        return { ...base, phase: "idle-locked", sealed: null };
    }
  }
  switch (state.phase) {
    case "idle-locked":
    case "locking":
      return animate
        ? { ...base, phase: "unlocking", sealed: null, run: state.run + 1 }
        : { ...base, phase: "idle-unlocked", sealed: null };
    case "unlocking":
    case "idle-unlocked":
      return base;
    default:
      return { ...base, phase: "idle-unlocked", sealed: null };
  }
}

export function revealReducer(state: RevealState, event: RevealEvent): RevealState {
  switch (event.type) {
    case "target":
      return onTarget(state, event.target, event.animate);
    case "seal":
      return state.phase === "idle-unlocked" || state.phase === "unlocking"
        ? { ...state, phase: "sealing", sealed: event.sealed }
        : state;
    case "abortSeal":
      return state.phase === "sealing" ? { ...state, phase: "idle-unlocked", sealed: null } : state;
    case "done":
      if (event.run !== state.run) {
        return state;
      }
      if (state.phase === "unlocking") {
        return { ...state, phase: "idle-unlocked" };
      }
      return state.phase === "locking" ? { ...state, phase: "idle-locked", sealed: null } : state;
  }
}
