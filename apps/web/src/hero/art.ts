/**
 * The hero art renderer: two passes on the canvas's own WebGL2 context. Pass 1 paints the
 * clear mesh gradient into a small texture; pass 2 composites mosaic, key, streaks, glitches
 * and grain at full resolution. Takes a fully computed frame; motion lives in motion.ts.
 */
import { COMPOSITE_FRAG, COMPOSITE_UNIFORMS, type CompositeUniform } from "./composite.ts";
import {
  createContext,
  createTarget,
  finishProgram,
  programDone,
  resizeTarget,
  startProgram,
  type PendingProgram,
  type Program,
  type Target,
} from "./gl.ts";
import { SCENE_FRAG, VERT } from "./glsl.ts";
import type { ArtParams } from "./params.ts";

const SCENE_UNIFORMS = ["uAspect", "uTime", "uPal", "uFlow"] as const;
type SceneUniform = (typeof SCENE_UNIFORMS)[number];

/** The scene texture is this share of the canvas (the gradient is low-frequency). */
const SCENE_SCALE = 0.34;

export interface Key {
  /** CSS px within the canvas. */
  x: number;
  y: number;
  radius: number;
  strength: number;
}

export interface ArtFrame {
  params: ArtParams;
  palette: Float32Array;
  /** Gradient clock (s). */
  clock: number;
  /** Block size in CSS px. */
  block: number;
  key: Key;
  ambient: Key;
  /** Extra sort strength and the streaks' downward pull (CSS px). */
  sortBoost: number;
  pull: number;
  sweep: number;
  glitchTick: number;
  grainTick: number;
  /** Still frame: the encrypted band instead of the keys. */
  band: boolean;
}

export class ArtRenderer {
  private readonly scene: Program<SceneUniform>;
  private readonly composite: Program<CompositeUniform>;
  private readonly target: Target;
  private width = 1;
  private height = 1;
  private density = 1;

  private constructor(
    readonly canvas: HTMLCanvasElement,
    readonly gl: WebGL2RenderingContext,
    programs: { scene: PendingProgram; composite: PendingProgram },
  ) {
    this.scene = finishProgram(gl, programs.scene, SCENE_UNIFORMS);
    this.composite = finishProgram(gl, programs.composite, COMPOSITE_UNIFORMS);
    this.target = createTarget(gl);
    gl.bindVertexArray(gl.createVertexArray());
    gl.disable(gl.BLEND);
    gl.disable(gl.DEPTH_TEST);
  }

  /**
   * A renderer once the shaders are ready, or `null` without WebGL2 or when they fail (the CSS
   * still stays). The driver compiles in parallel where it can, so the headline decode and the
   * page never stall on shader compilation.
   */
  static create(canvas: HTMLCanvasElement): Promise<ArtRenderer | null> {
    const gl = createContext(canvas);
    if (gl === null) {
      return Promise.resolve(null);
    }
    const parallel = gl.getExtension("KHR_parallel_shader_compile") !== null;
    let programs: { scene: PendingProgram; composite: PendingProgram };
    try {
      programs = {
        scene: startProgram(gl, { vert: VERT, frag: SCENE_FRAG }),
        composite: startProgram(gl, { vert: VERT, frag: COMPOSITE_FRAG }),
      };
    } catch {
      return Promise.resolve(null);
    }
    return new Promise((resolve) => {
      const poll = (): void => {
        if (!programDone(gl, programs.scene, parallel) || !programDone(gl, programs.composite, parallel)) {
          window.setTimeout(poll, 32);
          return;
        }
        try {
          resolve(new ArtRenderer(canvas, gl, programs));
        } catch {
          resolve(null);
        }
      };
      poll();
    });
  }

  get pixels(): number {
    return this.canvas.width * this.canvas.height;
  }

  /** Sizes the canvas to `width` x `height` CSS px at `density` device px per CSS px. */
  resize(width: number, height: number, density: number): void {
    this.width = Math.max(1, width);
    this.height = Math.max(1, height);
    this.density = density;
    const w = Math.max(1, Math.round(this.width * density));
    const h = Math.max(1, Math.round(this.height * density));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    resizeTarget(this.gl, this.target, Math.max(2, Math.round(w * SCENE_SCALE)), Math.max(2, Math.round(h * SCENE_SCALE)));
  }

  render(frame: ArtFrame): void {
    this.drawScene(frame);
    this.drawComposite(frame);
  }

  private drawScene(frame: ArtFrame): void {
    const { gl, scene, target } = this;
    const g = frame.params.gradient;
    gl.bindFramebuffer(gl.FRAMEBUFFER, target.framebuffer);
    gl.viewport(0, 0, target.width, target.height);
    gl.useProgram(scene.program);
    gl.uniform2f(scene.loc.uAspect, this.width / this.height, 1);
    gl.uniform1f(scene.loc.uTime, frame.clock);
    gl.uniform3fv(scene.loc.uPal, frame.palette);
    gl.uniform3f(scene.loc.uFlow, g.warp, g.chroma, ((g.hueShift % 5) + 5) % 5);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  private drawComposite(frame: ArtFrame): void {
    const { gl, composite: c, density: d } = this;
    const p = frame.params;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    gl.useProgram(c.program);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.target.texture);
    gl.uniform1i(c.loc.uScene, 0);
    gl.uniform2f(c.loc.uView, this.canvas.width, this.canvas.height);
    gl.uniform1f(c.loc.uDpr, d);
    gl.uniform3fv(c.loc.uPal, frame.palette);
    gl.uniform4f(c.loc.uBlock, Math.max(4, Math.round(frame.block * d)), p.block.split, p.block.snap, p.block.dither);
    gl.uniform4f(c.loc.uKey, frame.key.x * d, frame.key.y * d, frame.key.radius * d, frame.key.strength);
    gl.uniform4f(c.loc.uAmb, frame.ambient.x * d, frame.ambient.y * d, frame.ambient.radius * d, frame.ambient.strength);
    gl.uniform4f(c.loc.uMask, p.mask.softness, p.mask.lens, p.mask.ring * d, p.mask.ringAmount);
    gl.uniform4f(c.loc.uSort, p.sort.length * d, p.sort.threshold, p.sort.density, p.sort.strength + frame.sortBoost);
    gl.uniform2f(c.loc.uSortX, frame.pull * d, frame.sweep);
    gl.uniform2f(c.loc.uGlitch, p.glitch.amount, frame.glitchTick);
    gl.uniform3f(c.loc.uGrain, p.grain.amount, frame.grainTick, p.block.bevel);
    gl.uniform1f(c.loc.uBand, frame.band ? 1 : 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }
}
