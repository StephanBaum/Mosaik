import { afterEach, describe, expect, it, vi } from "vitest";
import type { Band } from "./band.ts";
import { lockSchedule } from "./choreography.ts";
import { startLock, startUnlock, type RunDeps } from "./driver.ts";
import type { Display } from "./paint.ts";
import { sceneFor } from "./scene.ts";
import type { SealedDisplay } from "./seal.ts";
import { BAND_PIXEL_BUDGET } from "./timing.ts";

const ORIGIN = [300, 200] as const;
const scene = sceneFor({ width: 1280, height: 800 }, ORIGIN, 1, BAND_PIXEL_BUDGET);

/** A clock and frame queue the test steps by hand. */
function fakeDeps(): { deps: RunDeps; step: (ms: number) => void; pending: () => number } {
  let now = 1000;
  let next = 1;
  const queue = new Map<number, FrameRequestCallback>();
  const deps: RunDeps = {
    raf: (cb) => {
      queue.set(next, cb);
      return next++;
    },
    caf: (id) => {
      queue.delete(id);
    },
    now: () => now,
    dark: () => false,
  };
  return {
    deps,
    step: (ms) => {
      now += ms;
      const frames = [...queue.values()];
      queue.clear();
      for (const cb of frames) {
        cb(now);
      }
    },
    pending: () => queue.size,
  };
}

function emptyDisplay(): Display {
  return { W: 1280, H: 800, S: 1, groups: [], cv: document.createElement("canvas"), baseCv: null, ctx: null };
}

interface FakeBand extends Band {
  resize: ReturnType<typeof vi.fn<Band["resize"]>>;
  upload: ReturnType<typeof vi.fn<Band["upload"]>>;
  draw: ReturnType<typeof vi.fn<Band["draw"]>>;
  clear: ReturnType<typeof vi.fn<Band["clear"]>>;
  warmUp: ReturnType<typeof vi.fn<Band["warmUp"]>>;
  dispose: ReturnType<typeof vi.fn<Band["dispose"]>>;
}

function fakeBand(): FakeBand {
  return {
    resize: vi.fn<Band["resize"]>(),
    upload: vi.fn<Band["upload"]>(),
    draw: vi.fn<Band["draw"]>(),
    clear: vi.fn<Band["clear"]>(),
    warmUp: vi.fn<Band["warmUp"]>(),
    dispose: vi.fn<Band["dispose"]>(),
    maxTexture: 4096,
    lost: false,
  };
}

/** Steps 50 ms frames until `done` holds, up to a cap. */
function runUntil(step: (ms: number) => void, done: () => boolean, cap = 200): void {
  for (let i = 0; i < cap && !done(); i++) {
    step(50);
  }
}

function polygonPoints(clip: string): string[] {
  return clip.slice("polygon(".length, -1).split(",");
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("startUnlock, lens", () => {
  it("opens as a point, grows, and ends with no inline style", () => {
    const { deps, step } = fakeDeps();
    const vault = document.createElement("div");
    const onDone = vi.fn();
    startUnlock({ vault, scene, mode: "lens", band: null, bandEl: null, build: () => emptyDisplay() }, deps, onDone);

    const first = polygonPoints(vault.style.clipPath);
    expect(first).toHaveLength(120);
    expect(new Set(first)).toEqual(new Set([`${ORIGIN[0].toFixed(2)}px ${ORIGIN[1].toFixed(2)}px`]));

    // Half a second in, the lens is still short of the far corner.
    for (let i = 0; i < 10; i++) {
      step(50);
    }
    expect(vault.style.clipPath).toMatch(/^polygon\(/);
    expect(onDone).not.toHaveBeenCalled();

    runUntil(step, () => onDone.mock.calls.length > 0);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(vault.style.clipPath).toBe("");
    expect(vault.style.opacity).toBe("");
  });

  it("restores every style on cancel, once, and never calls back", () => {
    const { deps, step, pending } = fakeDeps();
    const vault = document.createElement("div");
    const onDone = vi.fn();
    const cancel = startUnlock(
      { vault, scene, mode: "lens", band: null, bandEl: null, build: () => emptyDisplay() },
      deps,
      onDone,
    );
    step(50);
    expect(vault.style.clipPath).not.toBe("");
    cancel();
    cancel();
    expect(vault.style.clipPath).toBe("");
    expect(pending()).toBe(0);
    step(50);
    expect(onDone).not.toHaveBeenCalled();
  });
});

describe("startUnlock, fade", () => {
  it("fades the vault in within 0.25 s and never touches the band", () => {
    const { deps, step } = fakeDeps();
    const vault = document.createElement("div");
    const band = fakeBand();
    const onDone = vi.fn();
    startUnlock({ vault, scene, mode: "fade", band, bandEl: null, build: () => null }, deps, onDone);
    expect(vault.style.opacity).toBe("0");
    for (let i = 0; i < 5; i++) {
      step(50);
    }
    expect(vault.style.opacity).toBe("1");
    runUntil(step, () => onDone.mock.calls.length > 0);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(vault.style.opacity).toBe("");
    expect(band.resize).not.toHaveBeenCalled();
    expect(band.upload).not.toHaveBeenCalled();
    expect(band.draw).not.toHaveBeenCalled();
    expect(band.clear).not.toHaveBeenCalled();
  });
});

describe("startUnlock, glass", () => {
  function setup() {
    const clock = fakeDeps();
    const vault = document.createElement("div");
    const bandEl = document.createElement("canvas");
    const band = fakeBand();
    const build = vi.fn<(paint: boolean) => Display | null>(() => emptyDisplay());
    const onDone = vi.fn();
    startUnlock({ vault, scene, mode: "glass", band, bandEl, build }, clock.deps, onDone);
    return { ...clock, vault, bandEl, band, build, onDone };
  }

  it("draws the band while the lens grows and leaves it empty at the end", () => {
    const { step, band, bandEl, onDone } = setup();
    expect(band.upload).toHaveBeenCalledTimes(1);
    expect(band.draw).toHaveBeenCalledTimes(1);
    expect(bandEl.hasAttribute("data-on")).toBe(true);
    runUntil(step, () => onDone.mock.calls.length > 0);
    expect(onDone).toHaveBeenCalledTimes(1);
    expect(bandEl.hasAttribute("data-on")).toBe(false);
    expect(band.clear).toHaveBeenCalled();
  });

  it("falls back to the plain lens after three slow frames", () => {
    const { step, band, bandEl, vault } = setup();
    step(100);
    step(100);
    expect(bandEl.hasAttribute("data-on")).toBe(true);
    step(100);
    expect(bandEl.hasAttribute("data-on")).toBe(false);
    const drawn = band.draw.mock.calls.length;
    step(50);
    expect(band.draw.mock.calls.length).toBe(drawn);
    expect(vault.style.clipPath).toMatch(/^polygon\(/);
  });

  it("rebuilds the display when the vault changes while the band shows", async () => {
    const { step, vault, build } = setup();
    expect(build).toHaveBeenCalledTimes(1);
    vault.append(document.createElement("section"));
    await Promise.resolve();
    step(50);
    expect(build).toHaveBeenCalledTimes(1);
    for (let i = 0; i < 4; i++) {
      step(50);
    }
    expect(build).toHaveBeenCalledTimes(2);
  });

  it("drops to the lens when the display cannot be built", () => {
    const clock = fakeDeps();
    const vault = document.createElement("div");
    const band = fakeBand();
    const build = vi.fn((paint: boolean): Display | null => (paint ? null : emptyDisplay()));
    startUnlock({ vault, scene, mode: "glass", band, bandEl: null, build }, clock.deps, vi.fn());
    expect(build).toHaveBeenCalledWith(false);
    expect(band.draw).not.toHaveBeenCalled();
    expect(vault.style.clipPath).toMatch(/^polygon\(/);
  });
});

describe("startLock", () => {
  function sealedOf(mode: SealedDisplay["mode"]): SealedDisplay {
    return {
      display: emptyDisplay(),
      cv: document.createElement("canvas"),
      bandSrc: null,
      scene,
      mode,
      schedule: lockSchedule([], scene, mode === "fade"),
    };
  }

  it("closes the picture, hides it once the lens is gone and calls back once", () => {
    const { deps, step } = fakeDeps();
    const host = document.createElement("div");
    const onDone = vi.fn();
    startLock({ host, sealed: sealedOf("lens"), band: null, bandEl: null }, deps, onDone);
    expect(host.style.clipPath).toMatch(/^polygon\(/);
    expect(host.style.visibility).toBe("");
    runUntil(step, () => host.style.visibility === "hidden");
    expect(host.style.visibility).toBe("hidden");
    runUntil(step, () => onDone.mock.calls.length > 0);
    expect(onDone).toHaveBeenCalledTimes(1);
    // The picture stays hidden until React removes it: no flash of the whole picture.
    expect(host.style.visibility).toBe("hidden");
  });

  it("restores the host when cancelled", () => {
    const { deps, step } = fakeDeps();
    const host = document.createElement("div");
    const cancel = startLock({ host, sealed: sealedOf("lens"), band: null, bandEl: null }, deps, vi.fn());
    step(50);
    cancel();
    expect(host.style.clipPath).toBe("");
    expect(host.style.opacity).toBe("");
    expect(host.style.visibility).toBe("");
  });

  it("fades the picture out when motion is reduced", () => {
    const { deps, step } = fakeDeps();
    const host = document.createElement("div");
    const onDone = vi.fn();
    startLock({ host, sealed: sealedOf("fade"), band: null, bandEl: null }, deps, onDone);
    expect(host.style.opacity).toBe("1");
    for (let i = 0; i < 5; i++) {
      step(50);
    }
    expect(host.style.opacity).toBe("0");
    runUntil(step, () => onDone.mock.calls.length > 0);
    expect(onDone).toHaveBeenCalledTimes(1);
  });
});
