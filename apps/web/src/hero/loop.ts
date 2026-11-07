/**
 * The hero art's frame loop. Paints at 60 fps while the decode runs, the pointer moves or the
 * page scrolls, and at 30 fps for the idle flow; stops while the hero is off screen or the tab
 * is hidden. Budgets: DPR capped at 2, the canvas at 2.1e6 px, and three frames over 50 ms in
 * a row step the resolution down 1 → 0.75 → 0.5 → the designed still.
 */
import type { ArtRenderer } from "./art.ts";
import type { ArtMotion } from "./motion.ts";
import { readPalette } from "./palette.ts";

const MAX_DPR = 2;
const MAX_PIXELS = 2.1e6;
const MAX_STEP = 0.05;
const SLOW_MS = 50;
const SLOW_FRAMES = 3;
const GRACE_MS = 1500;
const SCALES = [1, 0.75, 0.5] as const;

export interface LoopStats {
  fps: number;
  scale: number;
  pixels: number;
  still: boolean;
}

export class ArtLoop {
  private raf = 0;
  private last = 0;
  private lastPaint = -1e9;
  private shown = true;
  private level = 0;
  private slow = 0;
  private guardFrom = performance.now() + GRACE_MS;
  private size = { width: 1, height: 1 };
  private palette: Float32Array;
  private readonly painted: number[] = [];

  constructor(
    private readonly box: HTMLElement,
    private readonly renderer: ArtRenderer,
    private readonly motion: ArtMotion,
    private readonly hooks: { still: boolean; onPaint: () => void },
  ) {
    this.palette = readPalette(box);
    document.addEventListener("visibilitychange", () => {
      if (document.hidden) {
        this.stop();
      } else {
        this.kick();
      }
    });
  }

  get still(): boolean {
    return this.hooks.still;
  }

  /** Re-reads the box size (resize, breakpoint) and repaints. */
  relayout(): void {
    const { width, height } = this.box.getBoundingClientRect();
    let density = Math.min(window.devicePixelRatio || 1, MAX_DPR) * (SCALES[this.level] ?? 0.5);
    if (width * height * density * density > MAX_PIXELS) {
      density = Math.sqrt(MAX_PIXELS / Math.max(1, width * height));
    }
    this.size = { width, height };
    this.renderer.resize(width, height, density);
    this.guardFrom = performance.now() + GRACE_MS;
    this.slow = 0;
    this.kick(true);
  }

  /** The theme changed: the palette lives in the stylesheet. */
  repalette(): void {
    this.palette = readPalette(this.box);
    this.kick(true);
  }

  setVisible(visible: boolean): void {
    this.shown = visible;
    if (visible) {
      this.kick();
    } else {
      this.stop();
    }
  }

  /** Queues a frame; `force` repaints even the still. */
  kick(force = false): void {
    if (force) {
      this.lastPaint = -1e9;
    }
    if (this.raf !== 0 || document.hidden || !this.shown) {
      return;
    }
    this.last = performance.now();
    this.raf = requestAnimationFrame(this.frame);
  }

  stats(): LoopStats {
    const n = this.painted.length;
    const first = this.painted[0] ?? 0;
    const last = this.painted[n - 1] ?? 0;
    return {
      fps: n > 1 && last > first ? ((n - 1) * 1000) / (last - first) : 0,
      scale: SCALES[this.level] ?? 0,
      pixels: this.renderer.pixels,
      still: this.hooks.still,
    };
  }

  private stop(): void {
    if (this.raf !== 0) {
      cancelAnimationFrame(this.raf);
      this.raf = 0;
    }
  }

  private readonly frame = (now: number): void => {
    this.raf = 0;
    if (this.hooks.still) {
      this.paintStill(now);
      return;
    }
    const interval = this.motion.hot(now) ? 16 : 33;
    if (now - this.lastPaint >= interval - 4) {
      const dt = Math.min(Math.max((now - this.lastPaint) / 1000, 0), MAX_STEP);
      this.guard(now);
      this.motion.step(dt, this.size);
      this.renderer.render(this.motion.frame(this.size, this.palette));
      this.painted.push(now);
      while (now - (this.painted[0] ?? now) > 1000) {
        this.painted.shift();
      }
      this.lastPaint = now;
      this.hooks.onPaint();
    }
    this.last = now;
    this.raf = requestAnimationFrame(this.frame);
  };

  private paintStill(now: number): void {
    if (this.lastPaint > 0) {
      return;
    }
    this.motion.intro = 1e3;
    this.renderer.render(this.motion.still(this.size, this.palette));
    this.lastPaint = now;
    this.hooks.onPaint();
  }

  /** Three slow frames in a row (after a grace period) step the resolution down, then to the still. */
  private guard(now: number): void {
    const elapsed = now - this.last;
    if (now < this.guardFrom || elapsed > 500) {
      return;
    }
    this.slow = elapsed > SLOW_MS ? this.slow + 1 : 0;
    if (this.slow < SLOW_FRAMES) {
      return;
    }
    this.level += 1;
    if (this.level >= SCALES.length) {
      this.hooks.still = true;
    }
    this.relayout();
  }
}
