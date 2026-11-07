/** WebGL2 plumbing for the hero art: context, programs and the scene render target. Cosmetic only. */

export class ArtGlError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ArtGlError";
  }
}

export function createContext(canvas: HTMLCanvasElement): WebGL2RenderingContext | null {
  if (typeof WebGL2RenderingContext === "undefined") {
    return null;
  }
  try {
    return canvas.getContext("webgl2", {
      alpha: false,
      antialias: false,
      depth: false,
      stencil: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
      powerPreference: "default",
    });
  } catch {
    return null;
  }
}

function compile(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (shader === null) {
    throw new ArtGlError("art: could not create a shader");
  }
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  return shader;
}

export interface Program<U extends string> {
  program: WebGLProgram;
  loc: Record<U, WebGLUniformLocation | null>;
}

/** A program whose compile and link may still run on the driver's threads. */
export interface PendingProgram {
  program: WebGLProgram;
  shaders: readonly WebGLShader[];
}

/** Starts compiling and linking without waiting for the result (no main-thread stall). */
export function startProgram(gl: WebGL2RenderingContext, sources: { vert: string; frag: string }): PendingProgram {
  const program = gl.createProgram();
  const shaders = [compile(gl, gl.VERTEX_SHADER, sources.vert), compile(gl, gl.FRAGMENT_SHADER, sources.frag)];
  for (const shader of shaders) {
    gl.attachShader(program, shader);
  }
  gl.linkProgram(program);
  return { program, shaders };
}

const COMPLETION_STATUS_KHR = 0x91b1;

/**
 * Whether the driver has finished the program. With KHR_parallel_shader_compile this never
 * blocks; without it the answer is always yes and the status check below waits once.
 */
export function programDone(gl: WebGL2RenderingContext, pending: PendingProgram, parallel: boolean): boolean {
  return !parallel || gl.getProgramParameter(pending.program, COMPLETION_STATUS_KHR) === true;
}

/** Checks the finished program and looks up its uniforms; throws with the GL log on failure. */
export function finishProgram<U extends string>(
  gl: WebGL2RenderingContext,
  pending: PendingProgram,
  uniforms: readonly U[],
): Program<U> {
  const { program, shaders } = pending;
  if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true) {
    const logs = shaders.map((shader) => gl.getShaderInfoLog(shader) ?? "").join(" ");
    throw new ArtGlError(`art: link failed: ${gl.getProgramInfoLog(program) ?? ""} ${logs}`);
  }
  const loc = {} as Record<U, WebGLUniformLocation | null>;
  for (const name of uniforms) {
    loc[name] = gl.getUniformLocation(program, name);
  }
  return { program, loc };
}

/** An RGBA8 texture with a framebuffer, bilinear and clamped: the scene pass renders here. */
export interface Target {
  texture: WebGLTexture;
  framebuffer: WebGLFramebuffer;
  width: number;
  height: number;
}

export function createTarget(gl: WebGL2RenderingContext): Target {
  const texture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
  const framebuffer = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, framebuffer);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, texture, 0);
  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  return { texture, framebuffer, width: 1, height: 1 };
}

export function resizeTarget(gl: WebGL2RenderingContext, target: Target, width: number, height: number): void {
  if (target.width === width && target.height === height) {
    return;
  }
  gl.bindTexture(gl.TEXTURE_2D, target.texture);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, width, height, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
  target.width = width;
  target.height = height;
}
