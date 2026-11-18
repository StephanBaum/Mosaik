/**
 * Boots the tower-city field behind the page (lazy chunk, loaded after `load` plus idle on a
 * capable tier). A fixed canvas runs the field shader; layers (data layer) draw on top of it.
 * It degrades in steps: the governor lowers the cost ("lite"), then stops ("still"); a lost
 * WebGL context, the motion toggle and a failed context all end on the CSS still. It never
 * throws into the page. Cosmetic only.
 */
import type { FxTier } from "../runtime/capability.ts";
import { setVeil } from "../runtime/fxRegistry.ts";
import { MOTION_EVENT } from "../runtime/motion.ts";
import { makeAtlas } from "./atlas.ts";
import { createDataLayer } from "./dataLayer.ts";
import { createFieldGl, type FieldGl } from "./gl.ts";
import { createGovernor } from "./governor.ts";
import { createLens, lensRadius, stepLens, type Lens } from "./lens.ts";
import { createPanelRings } from "./panelRings.ts";
import { createVeil } from "./veil.ts";

export interface Click {
  x: number;
  y: number;
  age: number;
}

export interface FieldLayer {
  draw(now: number, lens: Lens, gl: FieldGl): void;
  setClick(c: Click | null): void;
  dispose(): void;
}

export interface FieldHandle {
  readonly state: "running" | "still";
  addLayer(l: FieldLayer): void;
  stop(): void;
}

export interface BootDeps {
  createGl?: typeof createFieldGl;
  raf?: (cb: FrameRequestCallback) => number;
}

/** Scrolled past this many viewport heights, the field runs at half rate. */
const DIM_AFTER_VH = 1.9;
const CLICK_LIFE_S = 1.6;
const FONT = "500 38px 'IBM Plex Mono'";
const PRESS_TARGETS = "a, button, input, textarea, select, label, [role='button'], .holo";

const STILL: FieldHandle = {
  state: "still",
  addLayer: (l) => {
    l.dispose();
  },
  stop: () => undefined,
};

interface ClickAt {
  x: number;
  y: number;
  t0: number;
}

interface RunnerEnv {
  doc: Document;
  win: Window;
  canvas: HTMLCanvasElement;
  gl: FieldGl;
  raf: (cb: FrameRequestCallback) => number;
}

/** One running field: owns the frame loop, the input wiring and the degrade steps. */
class FieldRunner implements FieldHandle {
  state: "running" | "still" = "running";
  private lite: boolean;
  private shown = false;
  private frameId = 0;
  private last = 0;
  private skip = false;
  private click: ClickAt | null = null;
  private readonly lens: Lens;
  private readonly layers: FieldLayer[] = [];
  private readonly governor = createGovernor();
  private readonly cleanups: (() => void)[] = [];
  private readonly root: HTMLElement;

  private readonly doc: Document;
  private readonly win: Window;
  private readonly canvas: HTMLCanvasElement;
  private readonly gl: FieldGl;
  private readonly raf: (cb: FrameRequestCallback) => number;

  constructor(env: RunnerEnv, tier: Exclude<FxTier, "none">) {
    const { doc, win, canvas, gl, raf } = env;
    this.doc = doc;
    this.win = win;
    this.canvas = canvas;
    this.gl = gl;
    this.raf = raf;
    this.lite = tier === "lite";
    this.root = doc.documentElement;
    this.lens = createLens(win.innerWidth, win.innerHeight);
    this.root.dataset["fxState"] = tier;
    gl.resize();
    this.bindInput();
    this.bindLifecycle();
    this.frameId = raf(this.frame);
  }

  addLayer(l: FieldLayer): void {
    if (this.state === "running" && !this.lite) {
      this.layers.push(l);
    } else {
      l.dispose();
    }
  }

  stop = (): void => {
    if (this.state === "still") {
      return;
    }
    this.state = "still";
    for (const off of this.cleanups.splice(0)) {
      off();
    }
    for (const l of this.layers.splice(0)) {
      l.dispose();
    }
    setVeil(null);
    this.canvas.classList.remove("is-on");
    this.doc.querySelector(".field-still")?.removeAttribute("hidden");
    this.root.dataset["fxState"] = "still";
  };

  /** Registers something to release when the field stops. */
  own(release: () => void): void {
    this.cleanups.push(release);
  }

  /** The veil needs the glyph atlas, which needs the mono font. */
  attachVeil(createGl: typeof createFieldGl): void {
    const loaded = this.doc.fonts.load(FONT).catch(() => []);
    void loaded.then(() => {
      if (this.state === "running") {
        setVeil(createVeil(this.doc, makeAtlas(this.doc), createGl));
      }
    });
  }

  private on<K extends keyof WindowEventMap>(type: K, fn: (e: WindowEventMap[K]) => void): void {
    this.win.addEventListener(type, fn, { passive: true });
    this.cleanups.push(() => {
      this.win.removeEventListener(type, fn);
    });
  }

  private track(e: PointerEvent): void {
    this.lens.tx = e.clientX;
    this.lens.ty = e.clientY;
    this.lens.lastPointer = performance.now();
  }

  private bindInput(): void {
    this.on("pointermove", (e) => {
      const inHero = e.target instanceof Element && e.target.closest(".hero--home") !== null;
      if (e.pointerType !== "touch" || inHero) {
        this.track(e);
      }
    });
    this.on("pointerdown", (e) => {
      if (e.pointerType === "touch") {
        this.track(e);
      }
      const hit = e.target instanceof Element ? e.target.closest(PRESS_TARGETS) : null;
      if (hit === null && e.button === 0) {
        this.click = { x: e.clientX, y: e.clientY, t0: performance.now() };
      }
    });
    this.on("resize", () => {
      this.gl.resize();
    });
  }

  private listenDoc(type: string, fn: (e: Event) => void): void {
    this.doc.addEventListener(type, fn);
    this.cleanups.push(() => {
      this.doc.removeEventListener(type, fn);
    });
  }

  private bindLifecycle(): void {
    this.listenDoc("visibilitychange", () => {
      if (!this.doc.hidden && this.state === "running" && this.frameId === 0) {
        this.last = 0;
        this.frameId = this.raf(this.frame);
      }
    });
    this.listenDoc(MOTION_EVENT, (e) => {
      if ((e as CustomEvent<{ off: boolean }>).detail.off) {
        this.stop();
      }
    });
    this.canvas.addEventListener("webglcontextlost", this.stop);
  }

  private degrade(): void {
    this.lite = true;
    this.gl.dprCap = 1;
    this.gl.resize();
    for (const l of this.layers.splice(0)) {
      l.dispose();
    }
  }

  /** Feeds the governor; returns false when it ended the field. */
  private govern(rawMs: number): boolean {
    if (rawMs >= 250) {
      return true;
    }
    const verdict = this.governor.sample(rawMs);
    if (verdict === "stop") {
      this.stop();
      return false;
    }
    if (verdict === "lite") {
      this.degrade();
    }
    return true;
  }

  private frame = (now: number): void => {
    this.frameId = 0;
    if (this.state !== "running") {
      return;
    }
    if (this.gl.lost()) {
      this.stop();
      return;
    }
    if (this.doc.hidden) {
      return;
    }
    if (!this.gl.ready()) {
      this.frameId = this.raf(this.frame);
      return;
    }
    const rawMs = this.last === 0 ? 16 : now - this.last;
    const dt = Math.min(0.05, rawMs / 1000);
    this.last = now;
    const dim = this.win.scrollY > this.win.innerHeight * DIM_AFTER_VH;
    this.skip = !this.skip;
    if (!((this.lite || dim) && this.skip)) {
      this.renderOnce(now, dt);
    }
    if (this.govern(rawMs)) {
      this.frameId = this.raf(this.frame);
    }
  };

  /** Writes this frame's uniforms; returns the click age (-1 when there is no live click). */
  private writeUniforms(now: number, dt: number): number {
    const { win, gl, lens } = this;
    const u = gl.u;
    const w = win.innerWidth;
    const h = win.innerHeight;
    u.time += dt * 0.85;
    stepLens(lens, dt, now, { w, h });
    u.lensX = lens.x;
    u.lensY = lens.y;
    const raw = this.click === null ? -1 : (now - this.click.t0) / 1000;
    const age = raw > CLICK_LIFE_S ? -1 : raw;
    u.clickAge = age;
    u.clickX = this.click?.x ?? 0;
    u.clickY = this.click?.y ?? 0;
    const home = this.doc.body.dataset["page"] === "home";
    u.lensR = lensRadius(w, h);
    u.scroll = win.scrollY;
    u.page = home ? 1 : 0.62;
    u.heroY = home ? h * 0.5 : h * 0.42;
    return age;
  }

  private renderOnce(now: number, dt: number): void {
    const age = this.writeUniforms(now, dt);
    const { gl, click } = this;
    gl.render();
    const at = age < 0 || click === null ? null : { x: click.x * gl.dpr, y: gl.height - click.y * gl.dpr, age };
    for (const l of this.layers) {
      l.setClick(at);
      l.draw(now, this.lens, gl);
    }
    if (!this.shown) {
      this.shown = true;
      this.canvas.classList.add("is-on");
    }
  }
}

export function bootField(doc: Document, tier: Exclude<FxTier, "none">, deps: BootDeps = {}): FieldHandle {
  const win = doc.defaultView;
  if (win === null) {
    return STILL;
  }
  const createGl = deps.createGl ?? createFieldGl;
  const raf = deps.raf ?? ((cb: FrameRequestCallback): number => win.requestAnimationFrame(cb));
  const canvas = doc.createElement("canvas");
  canvas.className = "field";
  canvas.setAttribute("aria-hidden", "true");
  doc.body.insertBefore(canvas, doc.querySelector(".field-still") ?? doc.body.firstChild);
  const gl = createGl(canvas, { overlay: false, dprCap: tier === "full" ? 1.5 : 1 });
  if (gl === null) {
    canvas.remove();
    doc.documentElement.dataset["fxState"] = "still";
    return STILL;
  }
  const runner = new FieldRunner({ doc, win, canvas, gl, raf }, tier);
  if (tier === "full") {
    const data = createDataLayer(doc);
    if (data !== null) {
      runner.addLayer(data);
    }
    if ("fonts" in doc) {
      runner.attachVeil(createGl);
    }
  }
  const rings = createPanelRings(doc);
  runner.own(() => {
    rings.dispose();
  });
  return runner;
}
