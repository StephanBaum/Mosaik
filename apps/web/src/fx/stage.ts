/**
 * The page's single WebGL2 context (design concept §3, ADR-0027 item 6). It lives on an
 * offscreen canvas that is never attached to the DOM. Each target (the mosaic, the holo CTA,
 * the logo's foil tile) paints into the bottom-left of it and copies the pixels into its own
 * 2D canvas, so every surface scrolls, clips and stacks with its element, as in the desktop
 * renderer. One rAF loop drives all targets and runs only while one of them is visible and
 * animating; it stops when the tab is hidden. Cosmetic only.
 */
import { createContext, createProgram, hasWebGl2, type HoloGl, type HoloProgram } from "./gl.ts";

export interface FrameInfo {
  /** Seconds since the previous frame, capped. */
  dt: number;
  /** Clock time in ms. */
  now: number;
}

export interface Target {
  /** Advances the target; returns whether it wants more frames. */
  step(frame: FrameInfo): boolean;
  /** Paints when its state changed; called after `step`. */
  paint(stage: Stage): void;
  /** Minimum ms between paints while animating (16 for interaction, 33 for idle sheen). */
  interval(): number;
  /** Whether the target is on screen. Hidden targets neither step nor paint. */
  visible(): boolean;
  /** The context is gone for good: show the flat fallback. */
  lost(): void;
}

export interface Stage {
  readonly gl: HoloGl;
  /** The app's holo program (buttons and stills). */
  readonly holo: HoloProgram;
  /** Grows the GL canvas to at least `width` x `height` device px. */
  ensure(width: number, height: number): void;
  /** Copies the bottom-left `width` x `height` of the GL canvas into `ctx`. */
  copy(ctx: CanvasRenderingContext2D, width: number, height: number): void;
  add(target: Target): () => void;
  /** Queues a frame (after input, a resize or a theme change). */
  kick(): void;
}

const MAX_STEP = 0.05;

class GlStage implements Stage {
  private readonly targets = new Set<Target>();
  private raf = 0;
  private last = 0;
  private readonly lastPaint = new Map<Target, number>();

  constructor(
    readonly gl: HoloGl,
    readonly holo: HoloProgram,
    private readonly canvas: HTMLCanvasElement,
  ) {
    canvas.addEventListener("webglcontextlost", this.onLost);
    document.addEventListener("visibilitychange", this.onVisibility);
  }

  ensure(width: number, height: number): void {
    if (this.canvas.width < width) {
      this.canvas.width = width;
    }
    if (this.canvas.height < height) {
      this.canvas.height = height;
    }
  }

  copy(ctx: CanvasRenderingContext2D, width: number, height: number): void {
    ctx.clearRect(0, 0, width, height);
    ctx.drawImage(this.canvas, 0, this.canvas.height - height, width, height, 0, 0, width, height);
  }

  add(target: Target): () => void {
    this.targets.add(target);
    this.kick();
    return () => {
      this.targets.delete(target);
      this.lastPaint.delete(target);
    };
  }

  kick(): void {
    if (this.raf !== 0 || document.hidden) {
      return;
    }
    this.last = performance.now();
    this.raf = requestAnimationFrame(this.frame);
  }

  private readonly frame = (now: number): void => {
    this.raf = 0;
    const dt = Math.min(Math.max((now - this.last) / 1000, 0), MAX_STEP);
    this.last = now;
    let more = false;
    for (const target of this.targets) {
      if (!target.visible()) {
        continue;
      }
      const due = now - (this.lastPaint.get(target) ?? -1e9) >= target.interval() - 4;
      if (!due) {
        more = true;
        continue;
      }
      const elapsed = (now - (this.lastPaint.get(target) ?? now)) / 1000;
      more = target.step({ dt: Math.min(Math.max(elapsed, dt), MAX_STEP), now }) || more;
      target.paint(this);
      this.lastPaint.set(target, now);
    }
    if (more && !document.hidden) {
      this.raf = requestAnimationFrame(this.frame);
    }
  };

  private readonly onLost = (event: Event): void => {
    event.preventDefault();
    if (this.raf !== 0) {
      cancelAnimationFrame(this.raf);
      this.raf = 0;
    }
    for (const target of this.targets) {
      target.lost();
    }
    this.targets.clear();
  };

  private readonly onVisibility = (): void => {
    if (document.hidden) {
      if (this.raf !== 0) {
        cancelAnimationFrame(this.raf);
        this.raf = 0;
      }
      return;
    }
    // Restart from now: a long hidden pause must not arrive as one giant step.
    this.lastPaint.clear();
    this.kick();
  };
}

/** The shared stage, or `null` without WebGL2 (the page keeps its flat fallbacks). */
export function createStage(): Stage | null {
  if (!hasWebGl2()) {
    return null;
  }
  try {
    const canvas = document.createElement("canvas");
    canvas.width = 1;
    canvas.height = 1;
    const gl = createContext(canvas);
    if (gl === null) {
      return null;
    }
    const holo = createProgram(gl);
    gl.bindVertexArray(gl.createVertexArray());
    gl.disable(gl.BLEND);
    gl.disable(gl.DEPTH_TEST);
    return new GlStage(gl, holo, canvas);
  } catch {
    return null;
  }
}
