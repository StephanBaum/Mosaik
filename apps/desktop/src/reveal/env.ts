/** What the reveal needs from the platform, injectable for tests. No JSX. */
import { hasWebGl2 } from "../holo/gl.ts";
import { createBand, type Band } from "./band.ts";
import { prepareSeal } from "./seal.ts";

export interface RevealSupport {
  /** The snapshot painter and clip-path work: animate at all. Otherwise every swap is instant. */
  animate: boolean;
  /** WebGL2 is there: the glass band may be tried. */
  gl: boolean;
}

/** `win.CSS` is missing in some test environments. */
function supportsClipPath(css: Partial<typeof CSS> | undefined): boolean {
  return typeof css?.supports === "function" && css.supports("clip-path", "polygon(0px 0px, 1px 0px, 0px 1px)");
}

export function detectSupport(win: Window & typeof globalThis): RevealSupport {
  const animate =
    typeof CanvasRenderingContext2D === "function" &&
    typeof Range.prototype.getBoundingClientRect === "function" &&
    supportsClipPath(win.CSS);
  return { animate, gl: hasWebGl2() };
}

export interface RevealDeps {
  support: RevealSupport;
  /** Evaluated on every call: the user can change the setting at any time. */
  reduced(): boolean;
  raf(cb: FrameRequestCallback): number;
  caf(id: number): void;
  now(): number;
  dpr(): number;
  createBand(canvas: HTMLCanvasElement): Band | null;
  prepareSeal: typeof prepareSeal;
}

export function defaultDeps(win: Window & typeof globalThis = window): RevealDeps {
  return {
    support: detectSupport(win),
    reduced: () => win.matchMedia("(prefers-reduced-motion: reduce)").matches,
    raf: (cb) => win.requestAnimationFrame(cb),
    caf: (id) => {
      win.cancelAnimationFrame(id);
    },
    now: () => win.performance.now(),
    dpr: () => win.devicePixelRatio || 1,
    createBand,
    prepareSeal,
  };
}

export interface RevealEnv {
  deps: RevealDeps;
  /** The current theme; read by the driver while a run is active. */
  dark(): boolean;
  /** The one band, created on first use for `canvas`; null without WebGL2. */
  band(canvas: HTMLCanvasElement | null): Band | null;
}
