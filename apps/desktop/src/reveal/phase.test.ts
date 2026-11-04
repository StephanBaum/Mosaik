import { describe, expect, it } from "vitest";
import {
  initReveal,
  revealReducer,
  type LockedSession,
  type Phase,
  type RevealEvent,
  type RevealState,
  type RevealTarget,
} from "./phase.ts";
import type { SealedDisplay } from "./seal.ts";

const session: LockedSession = { state: "locked", email: "a@b.c", secretKeyStored: true, quickUnlock: [] };
const session2: LockedSession = { ...session, email: "other@b.c" };
const sealed = { tag: "sealed" } as unknown as SealedDisplay;

const locked: RevealTarget = { kind: "locked", session };
const vault: RevealTarget = { kind: "vault" };
const other: RevealTarget = { kind: "other", key: "kit" };

function at(phase: Phase, patch: Partial<RevealState> = {}): RevealState {
  const base = initReveal(locked);
  return { ...base, phase, locked: session, sealed: phase === "sealing" || phase === "locking" ? sealed : null, run: 3, ...patch };
}

function go(state: RevealState, target: RevealTarget, animate = true): RevealState {
  return revealReducer(state, { type: "target", target, animate });
}

describe("initReveal", () => {
  it("never animates the first paint", () => {
    expect(initReveal(other).phase).toBe("other");
    expect(initReveal(locked).phase).toBe("idle-locked");
    expect(initReveal(vault).phase).toBe("idle-unlocked");
  });
});

describe("target events", () => {
  it("other: every target is instant", () => {
    expect(go(at("other"), locked).phase).toBe("idle-locked");
    expect(go(at("other"), vault).phase).toBe("idle-unlocked");
    expect(go(at("other"), other).phase).toBe("other");
  });

  it("idle-locked: unlocks with a new run, or instantly without animation", () => {
    const state = at("idle-locked");
    const next = go(state, vault);
    expect(next.phase).toBe("unlocking");
    expect(next.run).toBe(state.run + 1);
    expect(go(state, vault, false).phase).toBe("idle-unlocked");
    expect(go(state, other).phase).toBe("other");
  });

  it("idle-locked: a new locked session only updates the stored one", () => {
    const next = go(at("idle-locked"), { kind: "locked", session: session2 });
    expect(next.phase).toBe("idle-locked");
    expect(next.locked).toBe(session2);
    expect(next.run).toBe(3);
  });

  it("unlocking: a lock the app did not start is instant", () => {
    expect(go(at("unlocking"), locked).phase).toBe("idle-locked");
    expect(go(at("unlocking"), other).phase).toBe("other");
  });

  it("idle-unlocked: an event-driven lock is instant", () => {
    const next = go(at("idle-unlocked"), locked);
    expect(next.phase).toBe("idle-locked");
    expect(next.sealed).toBeNull();
    expect(go(at("idle-unlocked"), other).phase).toBe("other");
  });

  it("sealing: the locked session starts the closing run", () => {
    const state = at("sealing");
    const next = go(state, locked);
    expect(next.phase).toBe("locking");
    expect(next.run).toBe(state.run + 1);
    expect(next.sealed).toBe(sealed);
    const away = go(state, other);
    expect(away.phase).toBe("other");
    expect(away.sealed).toBeNull();
  });

  it("locking: a locked session updates, a vault unlocks again, another screen drops the picture", () => {
    const state = at("locking");
    expect(go(state, { kind: "locked", session: session2 }).locked).toBe(session2);
    expect(go(state, { kind: "locked", session: session2 }).phase).toBe("locking");
    const again = go(state, vault);
    expect(again.phase).toBe("unlocking");
    expect(again.sealed).toBeNull();
    expect(again.run).toBe(state.run + 1);
    expect(go(state, vault, false).phase).toBe("idle-unlocked");
    const away = go(state, other);
    expect(away.phase).toBe("other");
    expect(away.sealed).toBeNull();
  });

  it("always records the key and the session of a locked target", () => {
    for (const phase of ["other", "idle-locked", "unlocking", "idle-unlocked", "sealing", "locking"] as const) {
      const next = go(at(phase), { kind: "locked", session: session2 });
      expect(next.key).toBe("locked");
      expect(next.locked).toBe(session2);
    }
  });
});

describe("seal, abortSeal and done", () => {
  const seal: RevealEvent = { type: "seal", sealed };

  it("seals only from the unlocked phases", () => {
    expect(revealReducer(at("idle-unlocked"), seal).phase).toBe("sealing");
    expect(revealReducer(at("unlocking"), seal).phase).toBe("sealing");
    for (const phase of ["other", "idle-locked", "sealing", "locking"] as const) {
      expect(revealReducer(at(phase), seal)).toEqual(at(phase));
    }
  });

  it("puts the vault back when a seal is aborted, and only then", () => {
    const back = revealReducer(at("sealing"), { type: "abortSeal" });
    expect(back.phase).toBe("idle-unlocked");
    expect(back.sealed).toBeNull();
    expect(revealReducer(at("locking"), { type: "abortSeal" }).phase).toBe("locking");
  });

  it("finishes the current run, and ignores a stale one", () => {
    expect(revealReducer(at("unlocking"), { type: "done", run: 3 }).phase).toBe("idle-unlocked");
    const done = revealReducer(at("locking"), { type: "done", run: 3 });
    expect(done.phase).toBe("idle-locked");
    expect(done.sealed).toBeNull();
    expect(revealReducer(at("unlocking"), { type: "done", run: 2 }).phase).toBe("unlocking");
    expect(revealReducer(at("locking"), { type: "done", run: 2 }).phase).toBe("locking");
    expect(revealReducer(at("idle-locked"), { type: "done", run: 3 }).phase).toBe("idle-locked");
  });
});
