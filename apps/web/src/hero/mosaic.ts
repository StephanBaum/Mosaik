/**
 * The living mosaic as a stage target (design concept §3 M1). Reads the tile grid from the
 * `.mosaic` box's custom properties (the same values lay out the flat fallback tiles), sizes
 * its canvas within the GPU budget, and paints one full-canvas pass per frame.
 *
 * Budgets (concept §5): DPR capped at 2 and the canvas at 2.1e6 px; 60 fps while interacting,
 * 30 fps for the idle sheen, a stop after 20 s without a pointer; three frames over 50 ms step
 * the resolution down 1 → 0.75 → 0.5 → still.
 */
import { linkProgram, type LinkedProgram } from "../fx/gl.ts";
import { screenPeriod } from "../fx/draw.ts";
import type { FrameInfo, Stage, Target } from "../fx/stage.ts";
import { ATLAS_CELL, createAtlasTexture } from "./atlas.ts";
import { MOSAIC_FRAG, MOSAIC_UNIFORMS, type MosaicUniform } from "./mosaic-shader.ts";
import { hot, introEnd, newMotion, stepMotion, type MosaicMotion } from "./motion.ts";
import { rippleWeight, type MosaicParams } from "./params.ts";
import { VERT } from "../fx/shader.ts";

const MAX_DPR = 2;
const MAX_PIXELS = 2.1e6;
const SLOW_MS = 50;
const SLOW_FRAMES = 3;
const GRACE_MS = 1500;
const SCALES = [1, 0.75, 0.5] as const;

export interface Layout {
  /** CSS px of the box. */
  width: number;
  height: number;
  pitch: number;
  tile: number;
  cols: number;
  rows: number;
  /** Device px per CSS px actually rendered. */
  density: number;
}

export interface MosaicStats {
  fps: number;
  scale: number;
  pixels: number;
  still: boolean;
}

function cssNumber(style: CSSStyleDeclaration, name: string): number {
  return Number.parseFloat(style.getPropertyValue(name));
}

/** The grid from the box's `--tile-n`, `--gap-n` and `--rows`, and the render density. */
export function readLayout(box: HTMLElement, scale: number): Layout {
  const style = getComputedStyle(box);
  const tile = cssNumber(style, "--tile-n") || 72;
  const pitch = tile + (cssNumber(style, "--gap-n") || 8);
  const rows = Math.max(1, Math.round(cssNumber(style, "--rows") || 7));
  const { width, height } = box.getBoundingClientRect();
  const cols = Math.max(1, Math.ceil(width / pitch));
  let density = Math.min(window.devicePixelRatio || 1, MAX_DPR) * scale;
  if (width * height * density * density > MAX_PIXELS) {
    density = Math.sqrt(MAX_PIXELS / (width * height));
  }
  return { width, height, pitch, tile, cols, rows, density };
}

export interface MosaicOptions {
  box: HTMLElement;
  canvas: HTMLCanvasElement;
  params: MosaicParams;
  /** Reduced motion or Save-Data: one designed still, no motion, no pointer. */
  still: boolean;
  dark: () => boolean;
  onLive: () => void;
  onLost: () => void;
}

export class MosaicTarget implements Target {
  readonly motion: MosaicMotion = newMotion(performance.now());
  layout: Layout;
  private program: LinkedProgram<MosaicUniform> | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private dirty = true;
  private shown = true;
  private live = false;
  private level = 0;
  private slowCount = 0;
  private guardFrom = performance.now() + GRACE_MS;
  private lastStep = 0;
  private readonly painted: number[] = [];
  private stillMode: boolean;

  constructor(private readonly o: MosaicOptions) {
    this.stillMode = o.still;
    this.layout = readLayout(o.box, 1);
    if (o.still) {
      this.motion.intro = 99;
    }
  }

  get params(): MosaicParams {
    return this.o.params;
  }

  setVisible(visible: boolean): void {
    this.shown = visible;
  }

  /** Re-reads the grid (resize, breakpoint) and repaints. */
  relayout(): void {
    this.layout = readLayout(this.o.box, SCALES[this.level] ?? 0.5);
    this.guardFrom = performance.now() + GRACE_MS;
    this.slowCount = 0;
    this.invalidate();
  }

  invalidate(): void {
    this.dirty = true;
  }

  visible(): boolean {
    return this.shown;
  }

  interval(): number {
    return this.stillMode || hot(this.motion, this.o.params, performance.now()) ? 16 : 33;
  }

  step(frame: FrameInfo): boolean {
    if (this.stillMode) {
      return false;
    }
    this.guardSlowFrames(frame.now);
    const active = stepMotion(this.motion, this.o.params, frame);
    this.dirty ||= active;
    return active;
  }

  lost(): void {
    this.live = false;
    this.o.onLost();
  }

  stats(): MosaicStats {
    const n = this.painted.length;
    const first = this.painted[0] ?? 0;
    const last = this.painted[n - 1] ?? 0;
    const fps = n > 1 && last > first ? ((n - 1) * 1000) / (last - first) : 0;
    const { width, height, density } = this.layout;
    return {
      fps,
      scale: SCALES[this.level] ?? 0,
      pixels: Math.round(width * density) * Math.round(height * density),
      still: this.stillMode,
    };
  }

  paint(stage: Stage): void {
    if (!this.dirty) {
      return;
    }
    const w = Math.max(1, Math.round(this.layout.width * this.layout.density));
    const h = Math.max(1, Math.round(this.layout.height * this.layout.density));
    if (!this.ensureProgram(stage)) {
      return;
    }
    if (this.o.canvas.width !== w || this.o.canvas.height !== h) {
      this.o.canvas.width = w;
      this.o.canvas.height = h;
    }
    stage.ensure(w, h);
    this.draw(stage, w, h);
    this.ctx ??= this.o.canvas.getContext("2d");
    if (this.ctx !== null) {
      stage.copy(this.ctx, w, h);
    }
    this.dirty = false;
    this.record();
    if (!this.live) {
      this.live = true;
      this.o.onLive();
    }
  }

  private record(): void {
    const now = performance.now();
    this.painted.push(now);
    while (this.painted.length > 0 && now - (this.painted[0] ?? now) > 1000) {
      this.painted.shift();
    }
  }

  private ensureProgram(stage: Stage): boolean {
    if (this.program !== null) {
      return true;
    }
    try {
      this.program = linkProgram(stage.gl, VERT, MOSAIC_FRAG, MOSAIC_UNIFORMS);
      stage.gl.useProgram(this.program.program);
      createAtlasTexture(stage.gl);
      return true;
    } catch {
      this.lost();
      return false;
    }
  }

  /** Three slow frames in a row (after a grace period) step the resolution down, then to a still. */
  private guardSlowFrames(now: number): void {
    const elapsed = now - this.lastStep;
    this.lastStep = now;
    if (now < this.guardFrom || elapsed > 500) {
      return;
    }
    this.slowCount = elapsed > SLOW_MS ? this.slowCount + 1 : 0;
    if (this.slowCount < SLOW_FRAMES) {
      return;
    }
    this.level += 1;
    if (this.level >= SCALES.length) {
      this.stillMode = true;
      this.motion.intro = introEnd(this.o.params) + 1;
    }
    this.relayout();
  }

  private draw(stage: Stage, w: number, h: number): void {
    const { gl } = stage;
    const program = this.program;
    if (program === null) {
      return;
    }
    gl.useProgram(program.program);
    gl.viewport(0, 0, w, h);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    setUniforms(gl, program, { layout: this.layout, params: this.o.params, motion: this.motion, still: this.stillMode, dark: this.o.dark(), size: [w, h] });
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}

interface UniformInput {
  layout: Layout;
  params: MosaicParams;
  motion: MosaicMotion;
  still: boolean;
  dark: boolean;
  size: readonly [number, number];
}

function setUniforms(gl: WebGL2RenderingContext, { loc }: LinkedProgram<MosaicUniform>, u: UniformInput): void {
  const { layout, params: p, motion: m } = u;
  const pitch = layout.pitch * layout.density;
  gl.uniform2f(loc.uView, u.size[0], u.size[1]);
  gl.uniform1f(loc.uPitch, pitch);
  // Tiles sit centred in their pitch cell; the box starts at the first tile's edge.
  gl.uniform1f(loc.uInset, ((layout.pitch - layout.tile) / 2) * layout.density);
  gl.uniform1f(loc.uTileK, layout.tile / layout.pitch);
  gl.uniform1f(loc.uRadius, p.foil.radius);
  gl.uniform1f(loc.uTilt, p.cursor.tilt);
  gl.uniform1f(loc.uGlow, p.cursor.glow);
  gl.uniform1f(loc.uScrPer, screenPeriod(layout.density));
  gl.uniform1f(loc.uStripeK, p.foil.stripeScale);
  gl.uniform1f(loc.uDark, u.dark ? 1 : 0);
  gl.uniform2f(loc.uGrid, layout.cols, layout.rows);
  gl.uniform4f(loc.uLight, p.cursor.sigma, p.cursor.hoverGain, p.foil.vibrance, p.foil.lift);
  gl.uniform4f(loc.uHue, p.foil.hueBase, p.foil.hueSpread, p.foil.hueJitter, p.foil.sheenLag);
  const r = p.ripple;
  gl.uniform4f(loc.uRipP, r.speed, (2 * Math.PI) / r.wavelength, r.runout, r.outline);
  gl.uniform2f(loc.uRipQ, r.refraction, r.shading);
  gl.uniform4f(loc.uGlyph, p.glyph.share, p.glyph.scale, p.glyph.etch, ATLAS_CELL);
  gl.uniform1i(loc.uGlyphs, 0);
  gl.uniform3f(loc.uIntro, m.intro, p.intro.stagger, p.intro.duration);
  if (u.still) {
    gl.uniform1f(loc.uClock, p.still.clock);
    gl.uniform3f(loc.uCursor, p.still.lightX, p.still.lightY, p.still.presence);
    gl.uniform4f(loc.uRipple, 0, 0, 99, 0);
    return;
  }
  gl.uniform1f(loc.uClock, m.clock);
  gl.uniform3f(loc.uCursor, m.cursorX, m.cursorY, m.presence);
  gl.uniform4f(loc.uRipple, m.ripX, m.ripY, m.ripAge, rippleWeight(m.ripAge, r.duration));
}
