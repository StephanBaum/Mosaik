/**
 * The WebGL2 side of the hero pixel sort: one context on the hero canvas, the painting as a
 * mipmapped texture, one full-screen triangle per frame. Cosmetic only; any failure leaves the
 * static painting and its baked band in place.
 */
import { createContext, type LinkedProgram } from "../fx/gl.ts";
import type { Cover } from "./geometry.ts";
import { FRAG, UNIFORMS, VERT, type SortUniform } from "./shader.ts";

/** Uniform values of one frame. Lengths are canvas px; `cursor` is `[x, y, radius, relax]`. */
export interface Frame {
  cover: Cover;
  time: number;
  /** `[threshold, length, drift, strength]`. */
  sort: readonly [number, number, number, number];
  /** `[position, width, sky, edge]`. */
  band: readonly [number, number, number, number];
  /** The copy's box as view uv `[x0, y0, x1, y1]`. */
  text: readonly [number, number, number, number];
  /** `[calm, softness]`. */
  calm: readonly [number, number];
  cursor: readonly [number, number, number, number];
  gain: number;
  /** Clean ellipse in image uv: `[cx, cy, rx, ry]` (radius 0 keeps nothing clean). */
  keep: readonly [number, number, number, number];
}

export interface SortRenderer {
  readonly canvas: HTMLCanvasElement;
  /** Sets the drawing buffer size in device px. */
  resize(width: number, height: number): void;
  draw(frame: Frame): void;
  /** Whether the context is gone. */
  lost(): boolean;
}

function uploadTexture(gl: WebGL2RenderingContext, image: TexImageSource): void {
  const texture = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, texture);
  gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, false);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, false);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
  gl.generateMipmap(gl.TEXTURE_2D);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR_MIPMAP_LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}

const COMPLETION_STATUS_KHR = 0x91b1;

function nextFrame(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => {
      resolve();
    });
  });
}

/**
 * Compiles and links without blocking the main thread where the driver can compile in parallel
 * (KHR_parallel_shader_compile): the status queries wait until the program is done.
 */
async function linkAsync(gl: WebGL2RenderingContext): Promise<LinkedProgram<SortUniform> | null> {
  const program = gl.createProgram();
  for (const [type, source] of [
    [gl.VERTEX_SHADER, VERT],
    [gl.FRAGMENT_SHADER, FRAG],
  ] as const) {
    const shader = gl.createShader(type);
    if (shader === null) {
      return null;
    }
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    gl.attachShader(program, shader);
  }
  gl.linkProgram(program);
  if (gl.getExtension("KHR_parallel_shader_compile") !== null) {
    while (gl.getProgramParameter(program, COMPLETION_STATUS_KHR) !== true && !gl.isContextLost()) {
      await nextFrame();
    }
  }
  if (gl.getProgramParameter(program, gl.LINK_STATUS) !== true) {
    return null;
  }
  const loc = {} as Record<SortUniform, WebGLUniformLocation | null>;
  for (const name of UNIFORMS) {
    loc[name] = gl.getUniformLocation(program, name);
  }
  return { program, loc };
}

function setUniforms(gl: WebGL2RenderingContext, loc: LinkedProgram<SortUniform>["loc"], f: Frame, view: [number, number]): void {
  gl.uniform1i(loc.uImage, 0);
  gl.uniform2f(loc.uView, view[0], view[1]);
  gl.uniform4f(loc.uCover, ...f.cover);
  gl.uniform1f(loc.uTime, f.time);
  gl.uniform4f(loc.uSort, ...f.sort);
  gl.uniform4f(loc.uBand, ...f.band);
  gl.uniform4f(loc.uText, ...f.text);
  gl.uniform2f(loc.uCalm, ...f.calm);
  gl.uniform4f(loc.uCursor, ...f.cursor);
  gl.uniform1f(loc.uGain, f.gain);
  gl.uniform4f(loc.uKeep, ...f.keep);
}

/**
 * A renderer for `canvas` that samples `image` (a decoded, same-origin `<img>`), or `null`
 * without WebGL2 or when the shader does not compile.
 */
export async function createSortRenderer(
  canvas: HTMLCanvasElement,
  image: TexImageSource,
  options: { preserveDrawingBuffer?: boolean } = {},
): Promise<SortRenderer | null> {
  const gl = createContext(canvas, { preserveDrawingBuffer: options.preserveDrawingBuffer ?? false });
  if (gl === null) {
    return null;
  }
  const program = await linkAsync(gl);
  if (program === null) {
    return null;
  }
  uploadTexture(gl, image);
  gl.bindVertexArray(gl.createVertexArray());
  gl.disable(gl.BLEND);
  gl.disable(gl.DEPTH_TEST);
  let isLost = false;
  canvas.addEventListener("webglcontextlost", (event) => {
    event.preventDefault();
    isLost = true;
  });
  return {
    canvas,
    resize(width, height) {
      if (canvas.width !== width || canvas.height !== height) {
        canvas.width = width;
        canvas.height = height;
      }
    },
    draw(frame) {
      if (isLost) {
        return;
      }
      gl.viewport(0, 0, canvas.width, canvas.height);
      gl.useProgram(program.program);
      setUniforms(gl, program.loc, frame, [canvas.width, canvas.height]);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    },
    lost: () => isLost || gl.isContextLost(),
  };
}
