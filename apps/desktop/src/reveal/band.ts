/**
 * The WebGL2 glass band (ADR-0032). Ported from reveal-studies.html L655-685: a full-screen
 * triangle whose fragment shader paints only the bevel band and rim line, sampling one
 * texture, the vault snapshot. Cosmetic only: no secret ever reaches this module, the
 * canvas never receives pointer events, and it is empty when a run ends.
 */
import { createContext, linkProgram, type HoloGl, type LinkedProgram } from "../holo/gl.ts";
import type { Point } from "./scene.ts";
import { BAND_FRAG, BAND_UNIFORMS, BAND_VERT, type BandUniform } from "./shader.ts";

export interface BandUniforms {
  uSize: Point;
  uO: Point;
  uT: number;
  uR: number;
  uWob: number;
  uShape: number;
  uH: number;
  uPres: number;
  uDark: number;
  uRefr: number;
  uBevelK: number;
  uBevelMax: number;
  uSpread: number;
  uFilm: number;
  uMag: number;
}

export interface Band {
  /** Sizes the drawing buffer in device px. */
  resize(w: number, h: number): void;
  /** Uploads the snapshot into the one texture (reallocated only when its size changes). */
  upload(source: HTMLCanvasElement): void;
  draw(uniforms: BandUniforms): void;
  /** Empties the canvas. */
  clear(): void;
  /** ANGLE compiles lazily at the first draw: draw once while idle (study L952-954). */
  warmUp(): void;
  readonly maxTexture: number;
  /** The GL context was lost; the band cannot draw any more. */
  readonly lost: boolean;
  dispose(): void;
}

const SCALARS = [
  "uT",
  "uR",
  "uWob",
  "uShape",
  "uH",
  "uPres",
  "uDark",
  "uRefr",
  "uBevelK",
  "uBevelMax",
  "uSpread",
  "uFilm",
  "uMag",
] as const;

const WARM_UP: BandUniforms = {
  uSize: [2, 2],
  uO: [1, 1],
  uT: 0,
  uR: 1,
  uWob: 0,
  uShape: 0,
  uH: 1,
  uPres: 1,
  uDark: 0,
  uRefr: 1,
  uBevelK: 0.5,
  uBevelMax: 10,
  uSpread: 0.1,
  uFilm: 0.1,
  uMag: 0,
};

function createTexture(gl: HoloGl): WebGLTexture {
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  return texture;
}

class GlBand implements Band {
  readonly maxTexture: number;
  private readonly program: LinkedProgram<BandUniform>;
  private readonly vao: WebGLVertexArrayObject;
  private readonly texture: WebGLTexture;
  private width = 1;
  private height = 1;
  private textureW = 0;
  private textureH = 0;
  private contextLost = false;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly gl: HoloGl,
  ) {
    this.program = linkProgram(gl, BAND_VERT, BAND_FRAG, BAND_UNIFORMS);
    this.vao = gl.createVertexArray();
    this.texture = createTexture(gl);
    this.maxTexture = Number(gl.getParameter(gl.MAX_TEXTURE_SIZE));
    canvas.addEventListener("webglcontextlost", (event) => {
      event.preventDefault();
      this.contextLost = true;
    });
  }

  get lost(): boolean {
    return this.contextLost || this.gl.isContextLost();
  }

  resize(w: number, h: number): void {
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
    }
    this.width = w;
    this.height = h;
  }

  upload(source: HTMLCanvasElement): void {
    const { gl } = this;
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
    gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
    if (this.textureW !== source.width || this.textureH !== source.height) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, gl.RGBA, gl.UNSIGNED_BYTE, source);
      this.textureW = source.width;
      this.textureH = source.height;
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, gl.RGBA, gl.UNSIGNED_BYTE, source);
    }
  }

  draw(u: BandUniforms): void {
    const { gl } = this;
    const { loc } = this.program;
    gl.viewport(0, 0, this.width, this.height);
    gl.disable(gl.BLEND);
    this.clear();
    gl.useProgram(this.program.program);
    gl.bindVertexArray(this.vao);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.texture);
    gl.uniform1i(loc.uVault, 0);
    gl.uniform2f(loc.uSize, u.uSize[0], u.uSize[1]);
    gl.uniform2f(loc.uO, u.uO[0], u.uO[1]);
    for (const name of SCALARS) {
      gl.uniform1f(loc[name], u[name]);
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3);
  }

  clear(): void {
    const { gl } = this;
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
  }

  warmUp(): void {
    this.resize(2, 2);
    this.draw(WARM_UP);
    this.gl.finish();
    this.clear();
  }

  dispose(): void {
    this.gl.deleteTexture(this.texture);
    this.gl.deleteProgram(this.program.program);
    this.gl.getExtension("WEBGL_lose_context")?.loseContext();
  }
}

/** The band on `canvas`, or `null` without WebGL2 or when its shader does not compile. */
export function createBand(canvas: HTMLCanvasElement): Band | null {
  try {
    const gl = createContext(canvas, { preserveDrawingBuffer: false });
    return gl === null ? null : new GlBand(canvas, gl);
  } catch {
    return null;
  }
}
