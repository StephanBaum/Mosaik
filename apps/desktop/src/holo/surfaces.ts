/**
 * Static foil surfaces (ADR-0027): decorative accents painted once from the shared renderer's
 * WebGL2 context into a small canvas, never animated. Pixels are cached per size and theme.
 * Cosmetic only: no secret ever reaches this module.
 */
export interface SurfaceHost {
  live: () => boolean;
  dark: () => boolean;
  dpr: () => number;
  /** Draws a bare foil of `width` x `height` device px; returns the canvas holding it at its bottom-left, or null. */
  draw: (width: number, height: number) => HTMLCanvasElement | null;
}

interface Surface {
  canvas: HTMLCanvasElement;
  release: () => void;
}

export const SURFACE_LIVE = "holo-surface--live";
const MAX_CACHED = 32;

export class SurfaceSet {
  private readonly surfaces = new Set<Surface>();
  private readonly cache = new Map<string, HTMLCanvasElement>();

  constructor(private readonly host: SurfaceHost) {}

  /** Paints `canvas` (sized by CSS) and keeps it painted across resizes and theme changes. */
  register(canvas: HTMLCanvasElement): () => void {
    const observer = new ResizeObserver(() => {
      this.paint(canvas);
    });
    observer.observe(canvas);
    const surface: Surface = {
      canvas,
      release: () => {
        observer.disconnect();
        canvas.classList.remove(SURFACE_LIVE);
      },
    };
    this.surfaces.add(surface);
    this.paint(canvas);
    return () => {
      surface.release();
      this.surfaces.delete(surface);
    };
  }

  /** The theme changed: repaint everything (the cache is keyed by theme). */
  repaintAll(): void {
    for (const { canvas } of this.surfaces) {
      this.paint(canvas);
    }
  }

  /** Context lost: fall back to the flat fill. */
  dropLive(): void {
    this.cache.clear();
    for (const { canvas } of this.surfaces) {
      canvas.classList.remove(SURFACE_LIVE);
    }
  }

  dispose(): void {
    for (const surface of this.surfaces) {
      surface.release();
    }
    this.surfaces.clear();
    this.cache.clear();
  }

  private paint(canvas: HTMLCanvasElement): void {
    const dpr = this.host.dpr();
    const width = Math.round(canvas.clientWidth * dpr);
    const height = Math.round(canvas.clientHeight * dpr);
    if (!this.host.live() || width === 0 || height === 0) {
      canvas.classList.remove(SURFACE_LIVE);
      return;
    }
    const pixels = this.pixels(width, height);
    const context = canvas.getContext("2d");
    if (pixels === null || context === null) {
      canvas.classList.remove(SURFACE_LIVE);
      return;
    }
    canvas.width = width;
    canvas.height = height;
    context.clearRect(0, 0, width, height);
    context.drawImage(pixels, 0, 0);
    canvas.classList.add(SURFACE_LIVE);
  }

  private pixels(width: number, height: number): HTMLCanvasElement | null {
    const key = `${String(width)}x${String(height)}x${this.host.dark() ? "d" : "l"}`;
    const cached = this.cache.get(key);
    if (cached !== undefined) {
      return cached;
    }
    const source = this.host.draw(width, height);
    if (source === null) {
      return null;
    }
    const copy = document.createElement("canvas");
    copy.width = width;
    copy.height = height;
    const context = copy.getContext("2d");
    if (context === null) {
      return null;
    }
    context.drawImage(source, 0, source.height - height, width, height, 0, 0, width, height);
    if (this.cache.size >= MAX_CACHED) {
      const oldest = this.cache.keys().next();
      if (oldest.done !== true) {
        this.cache.delete(oldest.value);
      }
    }
    this.cache.set(key, copy);
    return copy;
  }
}
