/**
 * The shared holo-foil renderer (ADR-0014). One WebGL2 context on an offscreen canvas that
 * is never attached to the DOM paints every registered button, then copies the pixels into
 * the button's own canvas. The foil therefore scrolls, clips and stacks with its button
 * (also inside a native `<dialog>`), and the module never touches `style` or injects CSS.
 * Cosmetic only: no secret ever reaches this module.
 */
import { drawFoil } from "./draw.ts";
import { attachEvents } from "./events.ts";
import {
  createContext,
  createProgram,
  hasWebGl2,
  type HoloGl,
  type HoloProgram,
} from "./gl.ts";
import { newState, update, type FoilState } from "./motion.ts";

export interface RendererDeps {
  createCanvas?: () => HTMLCanvasElement;
  raf?: (cb: FrameRequestCallback) => number;
  caf?: (id: number) => void;
  now?: () => number;
  matchMedia?: (query: string) => MediaQueryList;
  dpr?: () => number;
  createGl?: (canvas: HTMLCanvasElement) => HoloGl | null;
}

export interface RegisterOptions {
  /** Keeps the sheen moving without interaction. Use on at most one button per screen. */
  ambient?: boolean;
  seed?: number;
}

export interface HoloRenderer {
  /** Starts painting `foil` for `button`; returns the unregister function. No-op when not live. */
  register(button: HTMLButtonElement, foil: HTMLCanvasElement, options?: RegisterOptions): () => void;
  setDark(dark: boolean): void;
  readonly live: boolean;
  dispose(): void;
}

interface Entry {
  button: HTMLButtonElement;
  foil: HTMLCanvasElement;
  state: FoilState;
  context: CanvasRenderingContext2D | null;
  release: () => void;
}

const LIVE = "holo--live";
const MAX_DPR = 2;
const MAX_STEP = 0.05;
const GRACE_MS = 2500;
const SLOW_FRAME_MS = 24;
const STALL_MS = 500;
const SLOW_FRAMES = 3;
const REDUCED_QUERY = "(prefers-reduced-motion: reduce)";

class FoilRenderer implements HoloRenderer {
  live = false;
  private readonly deps: Required<RendererDeps>;
  private readonly entries = new Set<Entry>();
  private gl: HoloGl | null = null;
  private program: HoloProgram | null = null;
  private glCanvas: HTMLCanvasElement | null = null;
  private initialised = false;
  private dark = false;
  private reduced = false;
  private slow = false;
  private slowCount = 0;
  private guardFrom = 0;
  private raf = 0;
  private last = 0;
  private reducedQuery: MediaQueryList | null = null;

  constructor(deps: RendererDeps) {
    this.deps = {
      createCanvas: deps.createCanvas ?? (() => document.createElement("canvas")),
      raf: deps.raf ?? ((cb) => window.requestAnimationFrame(cb)),
      caf: deps.caf ?? ((id) => {
        window.cancelAnimationFrame(id);
      }),
      now: deps.now ?? (() => performance.now()),
      matchMedia: deps.matchMedia ?? ((query) => window.matchMedia(query)),
      dpr: deps.dpr ?? (() => window.devicePixelRatio || 1),
      createGl: deps.createGl ?? createContext,
    };
  }

  register(button: HTMLButtonElement, foil: HTMLCanvasElement, options: RegisterOptions = {}): () => void {
    this.init();
    if (!this.live) {
      return () => undefined;
    }
    const state = newState({ dark: this.dark, ambient: options.ambient === true, seed: options.seed });
    const entry: Entry = { button, foil, state, context: null, release: () => undefined };
    const poke = (): void => {
      state.dirty = true;
      this.kick();
    };
    const detach = attachEvents(button, state, {
      poke,
      now: this.deps.now,
      reduced: () => this.reduced,
    });
    const observer = new ResizeObserver(() => {
      this.resize(entry);
      this.kick();
    });
    observer.observe(button);
    entry.release = () => {
      detach();
      observer.disconnect();
      button.classList.remove(LIVE);
    };
    this.entries.add(entry);
    button.classList.add(LIVE);
    this.resize(entry);
    this.kick();
    return () => {
      entry.release();
      this.entries.delete(entry);
    };
  }

  setDark(dark: boolean): void {
    this.dark = dark;
    for (const entry of this.entries) {
      entry.state.dark = dark ? 1 : 0;
      entry.state.dirty = true;
    }
    this.kick();
  }

  dispose(): void {
    this.stop();
    for (const entry of this.entries) {
      entry.release();
    }
    this.entries.clear();
    this.reducedQuery?.removeEventListener("change", this.onReducedChange);
    this.reducedQuery = null;
    document.removeEventListener("visibilitychange", this.onVisibility);
    this.gl?.getExtension("WEBGL_lose_context")?.loseContext();
    this.live = false;
    this.gl = null;
    this.program = null;
  }

  /** Lazy, once: any failure leaves the renderer not live for good and buttons keep the static fill. */
  private init(): void {
    if (this.initialised) {
      return;
    }
    this.initialised = true;
    try {
      if (!hasWebGl2() && this.deps.createGl === createContext) {
        return;
      }
      const canvas = this.deps.createCanvas();
      const gl = this.deps.createGl(canvas);
      if (gl === null) {
        return;
      }
      const program = createProgram(gl);
      gl.bindVertexArray(gl.createVertexArray());
      gl.disable(gl.BLEND);
      gl.disable(gl.DEPTH_TEST);
      gl.useProgram(program.program);
      canvas.addEventListener("webglcontextlost", this.onContextLost);
      this.gl = gl;
      this.program = program;
      this.glCanvas = canvas;
      this.live = true;
    } catch {
      this.live = false;
      return;
    }
    this.reducedQuery = this.deps.matchMedia(REDUCED_QUERY);
    this.reduced = this.reducedQuery.matches;
    this.reducedQuery.addEventListener("change", this.onReducedChange);
    document.addEventListener("visibilitychange", this.onVisibility);
    this.guardFrom = this.deps.now() + GRACE_MS;
  }

  private readonly onContextLost = (event: Event): void => {
    event.preventDefault();
    this.live = false;
    this.stop();
    for (const entry of this.entries) {
      entry.button.classList.remove(LIVE);
    }
  };

  private readonly onReducedChange = (): void => {
    this.reduced = this.reducedQuery?.matches ?? false;
    this.markAllDirty();
  };

  private readonly onVisibility = (): void => {
    if (document.hidden) {
      this.stop();
    } else {
      this.markAllDirty();
    }
  };

  private markAllDirty(): void {
    for (const entry of this.entries) {
      entry.state.dirty = true;
    }
    this.kick();
  }

  private dpr(): number {
    return this.slow ? 1 : Math.min(this.deps.dpr(), MAX_DPR);
  }

  /** Sizes the foil canvas to the button in device px (offset size, so the press scale does not matter). */
  private resize(entry: Entry): void {
    const dpr = this.dpr();
    const width = Math.round(entry.button.offsetWidth * dpr);
    const height = Math.round(entry.button.offsetHeight * dpr);
    if (entry.foil.width !== width) {
      entry.foil.width = width;
    }
    if (entry.foil.height !== height) {
      entry.foil.height = height;
    }
    if (entry.button.offsetHeight > 0) {
      entry.state.bx = (0.5 * entry.button.offsetWidth) / entry.button.offsetHeight;
    }
    entry.state.dirty = true;
    this.guardFrom = this.deps.now() + GRACE_MS;
    this.slowCount = 0;
  }

  private kick(): void {
    if (!this.live || this.raf !== 0 || document.hidden) {
      return;
    }
    this.last = this.deps.now();
    this.raf = this.deps.raf(this.frame);
  }

  private stop(): void {
    if (this.raf !== 0) {
      this.deps.caf(this.raf);
      this.raf = 0;
    }
  }

  private readonly frame = (): void => {
    this.raf = 0;
    if (!this.live) {
      return;
    }
    const now = this.deps.now();
    const elapsed = now - this.last;
    this.last = now;
    const dt = Math.min(Math.max(elapsed / 1000, 0), MAX_STEP);
    const env = { reduced: this.reduced, slow: this.slow };
    let animating = false;
    for (const entry of this.entries) {
      update(entry.state, dt, now, env);
      animating ||= entry.state.active;
    }
    for (const entry of this.entries) {
      if (entry.state.active || entry.state.dirty) {
        this.paint(entry);
        entry.state.dirty = false;
      }
    }
    this.guardSlowFrames(animating, elapsed, now);
    if (animating && !document.hidden) {
      this.raf = this.deps.raf(this.frame);
    }
  };

  /** Three slow frames while animating: render at 1x and drop the ambient sheen. */
  private guardSlowFrames(animating: boolean, elapsed: number, now: number): void {
    if (this.slow || !animating) {
      return;
    }
    if (now > this.guardFrom && elapsed > SLOW_FRAME_MS && elapsed < STALL_MS) {
      this.slowCount += 1;
      if (this.slowCount >= SLOW_FRAMES) {
        this.slow = true;
        for (const entry of this.entries) {
          this.resize(entry);
        }
      }
    } else if (elapsed <= SLOW_FRAME_MS) {
      this.slowCount = 0;
    }
  }

  private paint(entry: Entry): void {
    const { gl, program, glCanvas } = this;
    const { width, height } = entry.foil;
    if (gl === null || program === null || glCanvas === null || width === 0 || height === 0) {
      return;
    }
    if (glCanvas.width < width) {
      glCanvas.width = width;
    }
    if (glCanvas.height < height) {
      glCanvas.height = height;
    }
    const s = entry.state;
    drawFoil(gl, program, {
      width,
      height,
      dpr: this.dpr(),
      mouse: { x: s.mx, y: s.my },
      time: s.time,
      hover: s.hover,
      press: s.press,
      focus: s.focus,
      dark: s.dark,
      seed: s.seed,
      mask: 1,
      stripeK: 1,
      unit: [0, 0],
    });
    entry.context ??= entry.foil.getContext("2d");
    entry.context?.clearRect(0, 0, width, height);
    entry.context?.drawImage(glCanvas, 0, glCanvas.height - height, width, height, 0, 0, width, height);
  }
}

export function createHoloRenderer(deps: RendererDeps = {}): HoloRenderer {
  return new FoilRenderer(deps);
}
