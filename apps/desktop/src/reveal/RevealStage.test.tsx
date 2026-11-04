import { act, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { SecretsProvider } from "../app/SecretsContext.tsx";
import { lockSchedule } from "./choreography.ts";
import { useLandmarkId, useReveal } from "./context.ts";
import { detectSupport, type RevealDeps } from "./env.ts";
import type { LockedSession } from "./phase.ts";
import { RevealProvider } from "./RevealProvider.tsx";
import { RevealStage, type RevealTarget } from "./RevealStage.tsx";
import { sceneFor } from "./scene.ts";
import type { SealedDisplay } from "./seal.ts";

const session: LockedSession = { state: "locked", email: "a@b.c", secretKeyStored: true, quickUnlock: [] };
const locked: RevealTarget = { kind: "locked", session };
const locked2: RevealTarget = { kind: "locked", session: { ...session, email: "other@b.c" } };
const vault: RevealTarget = { kind: "vault" };

/** Frames and time that the test advances by hand, inside `act`. */
function makeClock() {
  let now = 0;
  let next = 1;
  const queue = new Map<number, FrameRequestCallback>();
  return {
    raf: (cb: FrameRequestCallback) => {
      queue.set(next, cb);
      return next++;
    },
    caf: (id: number) => {
      queue.delete(id);
    },
    now: () => now,
    advance: (ms: number) => {
      act(() => {
        for (let t = 0; t < ms; t += 50) {
          now += 50;
          const frames = [...queue.values()];
          queue.clear();
          for (const cb of frames) {
            cb(now);
          }
        }
      });
    },
  };
}

function fakeSeal(): SealedDisplay {
  const cv = document.createElement("canvas");
  const scene = sceneFor({ width: 1000, height: 700 }, [500, 350], 1, 1e6);
  return {
    display: { W: 1000, H: 700, S: 1, groups: [], cv, baseCv: null, ctx: null },
    cv,
    bandSrc: null,
    scene,
    mode: "fade",
    schedule: lockSchedule([], scene, true),
  };
}

function FakeUnlock() {
  const mainId = useLandmarkId("main");
  return (
    <main id={mainId}>
      <label>
        Master password
        <input type="password" defaultValue="" autoFocus />
      </label>
      <button type="button">Unlock</button>
    </main>
  );
}

let lockResult: boolean | null = null;

function FakeVault() {
  const mainId = useLandmarkId("main");
  const reveal = useReveal();
  return (
    <main id={mainId} tabIndex={-1}>
      <span>top-secret</span>
      <button
        type="button"
        onClick={() => {
          lockResult = reveal.sealVault();
        }}
      >
        Lock
      </button>
    </main>
  );
}

function Harness({ target, deps }: { target: RevealTarget; deps: Partial<RevealDeps> }) {
  return (
    <SecretsProvider>
      <RevealProvider dark={false} deps={deps}>
        <RevealStage
          target={target}
          renderUnlock={() => <FakeUnlock />}
          renderVault={() => <FakeVault />}
          other={<p>other screen</p>}
        />
      </RevealProvider>
    </SecretsProvider>
  );
}

function setup(patch: Partial<RevealDeps> = {}) {
  const clock = makeClock();
  const deps: Partial<RevealDeps> = {
    support: { animate: true, gl: false },
    reduced: () => true,
    raf: clock.raf,
    caf: clock.caf,
    now: clock.now,
    dpr: () => 1,
    prepareSeal: fakeSeal,
    ...patch,
  };
  return { clock, deps };
}

const phase = (): string | null => document.querySelector(".reveal")?.getAttribute("data-phase") ?? null;

afterEach(() => {
  lockResult = null;
  vi.useRealTimers();
});

describe("RevealStage", () => {
  it("unlocks through a crossfade: the unlock layer is inert, focus moves to the vault", () => {
    const { clock, deps } = setup();
    const { rerender } = render(<Harness target={locked} deps={deps} />);
    expect(phase()).toBe("idle-locked");
    expect(screen.getByLabelText("Master password")).toBeTruthy();

    rerender(<Harness target={vault} deps={deps} />);
    expect(phase()).toBe("unlocking");
    expect(document.querySelector(".reveal")?.getAttribute("data-mode")).toBe("fade");
    const unlock = document.querySelector('[data-reveal-layer="unlock"]');
    expect(unlock?.hasAttribute("inert")).toBe(true);
    expect(unlock?.getAttribute("aria-hidden")).toBe("true");
    expect(document.activeElement).toBe(document.querySelector('[data-reveal-layer="vault"] main'));
    // Only the active layer carries the landmark id.
    expect(document.querySelectorAll("#main")).toHaveLength(1);

    clock.advance(300);
    expect(phase()).toBe("idle-unlocked");
    expect(screen.queryByLabelText("Master password")).toBeNull();
    expect(document.querySelector('[data-reveal-layer="unlock"]')).toBeNull();
    expect(document.querySelector(".reveal")?.hasAttribute("data-mode")).toBe(false);
  });

  it("seals the vault on lock: the secret leaves the DOM at once, the unlock screen takes focus", () => {
    const { clock, deps } = setup();
    const { rerender } = render(<Harness target={vault} deps={deps} />);
    expect(phase()).toBe("idle-unlocked");
    expect(screen.getByText("top-secret")).toBeTruthy();

    fireEvent.click(screen.getByRole("button", { name: "Lock" }));
    expect(lockResult).toBe(true);
    expect(phase()).toBe("sealing");
    expect(screen.queryByText("top-secret")).toBeNull();
    expect(document.querySelector('[data-reveal-layer="vault"]')).toBeNull();
    const sealed = document.querySelector('[data-reveal-layer="sealed"]');
    expect(sealed?.getAttribute("aria-hidden")).toBe("true");
    expect(sealed?.querySelector("canvas")).not.toBeNull();

    rerender(<Harness target={locked2} deps={deps} />);
    expect(phase()).toBe("locking");
    expect(document.activeElement).toBe(screen.getByLabelText("Master password"));
    expect(screen.queryByText("top-secret")).toBeNull();

    clock.advance(300);
    expect(phase()).toBe("idle-locked");
    expect(document.querySelectorAll('[data-reveal-layer="sealed"]')).toHaveLength(0);
  });

  it("puts the vault back when the locked session never comes", () => {
    vi.useFakeTimers();
    const { deps } = setup();
    render(<Harness target={vault} deps={deps} />);
    fireEvent.click(screen.getByRole("button", { name: "Lock" }));
    expect(phase()).toBe("sealing");
    act(() => {
      vi.advanceTimersByTime(4000);
    });
    expect(phase()).toBe("idle-unlocked");
    expect(screen.getByText("top-secret")).toBeTruthy();
    expect(document.querySelectorAll('[data-reveal-layer="sealed"]')).toHaveLength(0);
  });

  it("switches instantly when the app did not start the lock", () => {
    const { deps } = setup();
    const { rerender } = render(<Harness target={vault} deps={deps} />);
    rerender(<Harness target={locked} deps={deps} />);
    expect(phase()).toBe("idle-locked");
    expect(screen.queryByText("top-secret")).toBeNull();
    expect(document.querySelector('[data-reveal-layer="sealed"]')).toBeNull();
  });

  it("does not animate without support: sealVault says no and every swap is instant", () => {
    const { deps } = setup({ support: { animate: false, gl: false } });
    const { rerender } = render(<Harness target={locked} deps={deps} />);
    rerender(<Harness target={vault} deps={deps} />);
    expect(phase()).toBe("idle-unlocked");
    expect(document.querySelector('[data-reveal-layer="unlock"]')).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Lock" }));
    expect(lockResult).toBe(false);
    expect(phase()).toBe("idle-unlocked");
    rerender(<Harness target={locked} deps={deps} />);
    expect(phase()).toBe("idle-locked");
  });

  it("falls back to an instant lock when the snapshot cannot be made", () => {
    const { deps } = setup({ prepareSeal: () => null });
    render(<Harness target={vault} deps={deps} />);
    fireEvent.click(screen.getByRole("button", { name: "Lock" }));
    expect(lockResult).toBe(false);
    expect(phase()).toBe("idle-unlocked");
  });

  it("shows other screens without any layer of the reveal", () => {
    const { deps } = setup();
    render(<Harness target={{ kind: "other", key: "kit" }} deps={deps} />);
    expect(phase()).toBe("other");
    expect(screen.getByText("other screen")).toBeTruthy();
  });
});

describe("detectSupport", () => {
  it("reports no animation under jsdom, so existing tests keep the instant path", () => {
    expect(detectSupport(window).animate).toBe(false);
    expect(detectSupport(window).gl).toBe(false);
  });
});
