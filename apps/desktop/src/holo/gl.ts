/** WebGL2 setup for the holo foil: context, program and uniform locations. Cosmetic only (ADR-0014). */
import { FRAG, UNIFORMS, VERT, type Uniform } from "./shader.ts";

export type HoloGl = WebGL2RenderingContext;

/** A shader that failed to compile or link. The message carries the GL log, never app data. */
export class HoloError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "HoloError";
  }
}

export function hasWebGl2(): boolean {
  return typeof WebGL2RenderingContext !== "undefined";
}

/** A WebGL2 context for the offscreen canvas, or `null` when the platform has none. */
export function createContext(canvas: HTMLCanvasElement): HoloGl | null {
  try {
    return canvas.getContext("webgl2", {
      alpha: true,
      premultipliedAlpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      preserveDrawingBuffer: true,
    });
  } catch {
    return null;
  }
}

function compile(gl: HoloGl, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (shader === null) {
    throw new HoloError("holo: could not create a shader");
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (gl.getShaderParameter(shader, gl.COMPILE_STATUS) !== true) {
    const log = gl.getShaderInfoLog(shader) ?? "";
    gl.deleteShader(shader);
    throw new HoloError(`holo: shader compile failed: ${log}`);
  }
  return shader;
}

export interface HoloProgram {
  program: WebGLProgram;
  loc: Record<Uniform, WebGLUniformLocation | null>;
}

export function createProgram(gl: HoloGl): HoloProgram {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(program);
  if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true) {
    throw new HoloError(`holo: link failed: ${gl.getProgramInfoLog(program) ?? ""}`);
  }
  const loc = {} as Record<Uniform, WebGLUniformLocation | null>;
  for (const name of UNIFORMS) {
    loc[name] = gl.getUniformLocation(program, name);
  }
  return { program, loc };
}
