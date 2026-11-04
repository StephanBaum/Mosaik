/**
 * The frame loops of the two runs (study L811-838). Every frame is a pure function of the
 * run time; the DOM (or, when locking, the sealed picture) is always what the user sees, and
 * the WebGL band only adds the glass rim on top. Both loops write inline styles through the
 * CSSOM only and restore every one of them when cancelled.
 */
import type { Band, BandUniforms } from "./band.ts";
import {
  applyPoses,
  posesAt,
  poseKey,
  REST_POSE,
  unlockSchedule,
  type Pose,
  type Schedule,
} from "./choreography.ts";
import { lockFrame, unlockFrame } from "./frames.ts";
import type { Lens } from "./lens.ts";
import { compose, releaseDisplay, type Display } from "./paint.ts";
import type { Scene } from "./scene.ts";
import type { SealedDisplay } from "./seal.ts";
import {
  clamp,
  GLASS,
  MAX_STEP_S,
  MUTATION_REBUILD_MS,
  REDUCED_FADE_S,
  SLOW_FRAME_MS,
  SLOW_FRAMES,
} from "./timing.ts";

export type RunMode = "glass" | "lens" | "fade";

export interface RunDeps {
  raf(cb: FrameRequestCallback): number;
  caf(id: number): void;
  now(): number;
  dark(): boolean;
}

export interface UnlockRun {
  /** The vault layer; the lens clips it. */
  vault: HTMLElement;
  scene: Scene;
  mode: RunMode;
  band: Band | null;
  bandEl: HTMLCanvasElement | null;
  /** Builds the vault display (painted for glass, group geometry only otherwise). */
  build(paint: boolean): Display | null;
}

export interface LockRun {
  /** The host of the sealed picture; the lens clips it. */
  host: HTMLElement;
  sealed: SealedDisplay;
  band: Band | null;
  bandEl: HTMLCanvasElement | null;
}

export function bandUniforms(lens: Lens, scene: Scene, dark: boolean): BandUniforms {
  return {
    uSize: [scene.w, scene.h],
    uO: scene.o,
    uT: lens.t,
    uR: lens.R,
    uWob: lens.wob,
    uShape: lens.shape,
    uH: lens.H,
    uPres: lens.pres,
    uDark: dark ? 1 : 0,
    uRefr: GLASS.refraction,
    uBevelK: GLASS.bevel,
    uBevelMax: GLASS.bevelMax,
    uSpread: GLASS.dispersion,
    uFilm: GLASS.film,
    uMag: GLASS.magnify,
  };
}

/** The run time, advanced by real frame times (each capped), and a count of slow frames. */
class Clock {
  u = 0;
  private last: number;
  private slow = 0;

  constructor(private readonly now: () => number) {
    this.last = now();
  }

  step(): void {
    const t = this.now();
    const ms = t - this.last;
    this.last = t;
    this.u += Math.min(ms / 1000, MAX_STEP_S);
    this.slow = ms > SLOW_FRAME_MS ? this.slow + 1 : 0;
  }

  /** Too many slow frames in a row: the glass is costing more than it is worth. */
  get sluggish(): boolean {
    return this.slow >= SLOW_FRAMES;
  }
}

/** Calls `handler` when the window is hidden or resized: a run that cannot be watched ends at once. */
function onInterrupt(handler: () => void): () => void {
  const hidden = () => {
    if (document.visibilityState === "hidden") {
      handler();
    }
  };
  document.addEventListener("visibilitychange", hidden);
  window.addEventListener("resize", handler);
  return () => {
    document.removeEventListener("visibilitychange", hidden);
    window.removeEventListener("resize", handler);
  };
}

/** The band is cosmetic: a failure to clear it must never stop a run. */
function hideBand(band: Band | null, bandEl: HTMLCanvasElement | null): void {
  bandEl?.toggleAttribute("data-on", false);
  try {
    band?.clear();
  } catch {
    // Nothing to do: the band is gone and the lens continues without it.
  }
}

abstract class Driver {
  protected readonly clock: Clock;
  protected rafId = 0;
  protected ended = false;
  protected bandVisible = false;
  /** The pose key of the picture last uploaded; null until the first upload. */
  protected poseKey: string | null = null;
  private readonly detach: () => void;

  protected mode: RunMode;
  protected readonly band: Band | null;
  protected readonly bandEl: HTMLCanvasElement | null;

  constructor(
    protected readonly deps: RunDeps,
    glass: { mode: RunMode; band: Band | null; bandEl: HTMLCanvasElement | null },
    private readonly onDone: () => void,
  ) {
    this.mode = glass.mode;
    // Only a glass run touches the band; the lens and fade runs leave it alone.
    this.band = glass.mode === "glass" ? glass.band : null;
    this.bandEl = glass.mode === "glass" ? glass.bandEl : null;
    this.clock = new Clock(() => deps.now());
    this.detach = onInterrupt(() => {
      this.finish();
    });
  }

  protected abstract total(): number;
  protected abstract frame(u: number): void;
  protected abstract restore(): void;

  /** Applies the first frame now, before the browser paints, and schedules the rest. */
  start(): () => void {
    this.frame(0);
    this.rafId = this.deps.raf(this.tick);
    return this.cancel;
  }

  private readonly tick = (): void => {
    if (this.ended) {
      return;
    }
    this.clock.step();
    if (this.mode === "glass" && this.clock.sluggish) {
      this.degrade();
    }
    this.beforeFrame();
    this.frame(this.clock.u);
    if (this.clock.u >= this.total()) {
      this.finish();
    } else {
      this.rafId = this.deps.raf(this.tick);
    }
  };

  protected beforeFrame(): void {
    // Hook for the unlock loop's rebuilds.
  }

  /** Drops the glass band for the plain lens. */
  protected degrade(): void {
    this.mode = "lens";
    this.bandVisible = false;
    hideBand(this.band, this.bandEl);
  }

  protected teardown(): void {
    this.ended = true;
    this.deps.caf(this.rafId);
    this.detach();
    hideBand(this.band, this.bandEl);
  }

  readonly cancel = (): void => {
    if (!this.ended) {
      this.teardown();
      this.restore();
    }
  };

  protected finish(): void {
    if (!this.ended) {
      this.teardown();
      this.restoreAtEnd();
      this.onDone();
    }
  }

  protected restoreAtEnd(): void {
    this.restore();
  }

  /** Draws the band for one frame; any failure drops to the plain lens. */
  protected drawBand(draw: (band: Band) => void): void {
    if (this.band === null) {
      return;
    }
    try {
      draw(this.band);
      if (!this.bandVisible) {
        this.bandEl?.toggleAttribute("data-on", true);
        this.bandVisible = true;
      }
    } catch {
      this.degrade();
    }
  }
}

class UnlockDriver extends Driver {
  private display: Display | null = null;
  private els: HTMLElement[] = [];
  private schedule: Schedule;
  private length: number;
  private dirty = false;
  private lastBuild: number;
  private readonly observer: MutationObserver | null;

  constructor(
    private readonly run: UnlockRun,
    deps: RunDeps,
    onDone: () => void,
  ) {
    super(deps, run, onDone);
    if (run.mode !== "fade") {
      this.display = run.build(run.mode === "glass");
      if (this.display === null && this.mode === "glass") {
        this.mode = "lens";
        this.display = run.build(false);
      }
    }
    this.els = this.display?.groups.map((g) => g.el) ?? [];
    this.schedule = unlockSchedule(this.display?.groups ?? [], run.scene, run.mode === "fade");
    this.length = this.schedule.total;
    this.lastBuild = deps.now();
    if (this.mode === "glass") {
      run.band?.resize(Math.round(run.scene.w * run.scene.S), Math.round(run.scene.h * run.scene.S));
    }
    this.observer = this.mode === "glass" ? this.observe() : null;
  }

  /** The vault fills in while it runs (lists load): note changes so the band can be rebuilt. */
  private observe(): MutationObserver {
    const observer = new MutationObserver(() => {
      this.dirty = true;
    });
    observer.observe(this.run.vault, {
      childList: true,
      subtree: true,
      characterData: true,
      attributes: true,
      attributeFilter: ["class", "hidden", "aria-current", "aria-expanded"],
    });
    return observer;
  }

  protected total(): number {
    return this.length;
  }

  protected override beforeFrame(): void {
    const due = this.clock.u > 0 && this.deps.now() - this.lastBuild >= MUTATION_REBUILD_MS;
    if (this.dirty && due && this.mode === "glass" && this.bandVisible) {
      this.rebuild();
    }
  }

  private rebuild(): void {
    const next = this.run.build(true);
    this.dirty = false;
    this.lastBuild = this.deps.now();
    if (next === null) {
      return;
    }
    const old = this.display;
    this.display = next;
    this.els = next.groups.map((g) => g.el);
    this.schedule = unlockSchedule(next.groups, this.run.scene, false);
    this.length = Math.max(this.length, this.schedule.total);
    this.poseKey = null;
    if (old !== null) {
      releaseDisplay(old);
    }
  }

  protected frame(u: number): void {
    const { vault, scene } = this.run;
    if (this.mode === "fade") {
      vault.style.opacity = String(clamp(u / REDUCED_FADE_S));
      return;
    }
    const f = unlockFrame(u, scene);
    vault.style.clipPath = f.clip ?? "";
    const poses = posesAt("unlock", u, this.display?.groups ?? [], this.schedule);
    applyPoses(this.els, poses);
    if (this.mode !== "glass") {
      return;
    }
    if (f.bandOn) {
      this.paintBand(f.lens, poses);
    } else if (this.bandVisible) {
      this.bandVisible = false;
      hideBand(this.band, this.bandEl);
    }
  }

  private paintBand(lens: Lens, poses: readonly Pose[]): void {
    const display = this.display;
    if (display === null) {
      return;
    }
    this.drawBand((band) => {
      const key = poseKey(poses);
      if (key !== this.poseKey && display.cv !== null) {
        compose(display, poses);
        band.upload(display.cv);
        this.poseKey = key;
      }
      band.draw(bandUniforms(lens, this.run.scene, this.deps.dark()));
    });
  }

  protected restore(): void {
    this.observer?.disconnect();
    this.run.vault.style.clipPath = "";
    this.run.vault.style.opacity = "";
    applyPoses(
      this.els,
      this.els.map(() => REST_POSE),
    );
    if (this.display !== null) {
      releaseDisplay(this.display);
      this.display = null;
    }
  }
}

class LockDriver extends Driver {
  private readonly bandCtx: CanvasRenderingContext2D | null;

  constructor(
    private readonly run: LockRun,
    deps: RunDeps,
    onDone: () => void,
  ) {
    super(deps, { mode: run.sealed.mode, band: run.band, bandEl: run.bandEl }, onDone);
    const src = run.sealed.bandSrc;
    this.bandCtx = this.mode === "glass" && src !== null ? src.getContext("2d", { alpha: false }) : null;
    if (src !== null && this.bandCtx !== null) {
      run.band?.resize(src.width, src.height);
    } else if (this.mode === "glass") {
      this.mode = "lens";
    }
  }

  protected total(): number {
    return this.run.sealed.schedule.total;
  }

  protected frame(u: number): void {
    const { host, sealed } = this.run;
    if (this.mode === "fade") {
      host.style.opacity = String(1 - clamp(u / REDUCED_FADE_S));
      return;
    }
    const f = lockFrame(u, sealed.scene);
    if (f.gone) {
      // The picture is wholly outside the lens: hide it, since no clip means "show all".
      host.style.visibility = "hidden";
      hideBand(this.band, this.bandEl);
      this.bandVisible = false;
      return;
    }
    host.style.clipPath = f.clip ?? "";
    const poses = posesAt("lock", u, sealed.display.groups, sealed.schedule);
    const key = poseKey(poses);
    if (key !== this.poseKey) {
      compose(sealed.display, poses);
      this.poseKey = key;
      this.fillBandSource();
    }
    if (this.mode === "glass") {
      this.drawBand((band) => {
        band.upload(this.bandSource());
        band.draw(bandUniforms(f.lens, sealed.scene, this.deps.dark()));
      });
    }
  }

  private bandSource(): HTMLCanvasElement {
    return this.run.sealed.bandSrc ?? this.run.sealed.cv;
  }

  private fillBandSource(): void {
    const { sealed } = this.run;
    if (this.bandCtx !== null && sealed.bandSrc !== null && sealed.display.cv !== null) {
      this.bandCtx.drawImage(sealed.display.cv, 0, 0, sealed.bandSrc.width, sealed.bandSrc.height);
    }
  }

  protected restore(): void {
    this.run.host.style.clipPath = "";
    this.run.host.style.opacity = "";
    this.run.host.style.visibility = "";
  }

  /** At the natural end the picture stays hidden until React removes it: no flash of the full picture. */
  protected override restoreAtEnd(): void {
    // Intentionally empty.
  }
}

/** Starts the unlock run. Returns an idempotent `cancel` that restores every inline style written. */
export function startUnlock(run: UnlockRun, deps: RunDeps, onDone: () => void): () => void {
  return new UnlockDriver(run, deps, onDone).start();
}

/** Starts the lock run over the sealed picture. Returns an idempotent `cancel`. */
export function startLock(run: LockRun, deps: RunDeps, onDone: () => void): () => void {
  return new LockDriver(run, deps, onDone).start();
}
