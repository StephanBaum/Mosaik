/**
 * Raw WebGL2 for the field and the veil: one program, one full-screen triangle, an atlas
 * texture. The context refuses software renderers (`failIfMajorPerformanceCaveat`). When the
 * driver offers `KHR_parallel_shader_compile`, compiling does not block: `ready()` stays false
 * until the link finishes (checked once per frame, at most 60 frames, then the link status
 * decides). A failed link, a refused context or a lost context all end in `null` or `lost()`,
 * never in a thrown error. Cosmetic only.
 */
import { canvasSize } from "./size.ts";
import { FIELD_FRAG, FIELD_UNIFORMS, FIELD_VERT } from "./shaders.ts";

export interface FieldUniforms {
  time: number;
  lensX: number;
  lensY: number;
  lensR: number;
  scroll: number;
  wipe: number;
  page: number;
  heroY: number;
  cell: number;
  clickX: number;
  clickY: number;
  clickAge: number;
}

export interface FieldGl {
  canvas: HTMLCanvasElement;
  u: FieldUniforms;
  /** Device pixels per CSS pixel actually used (after the cap and the pixel ceiling). */
  dpr: number;
  /** The cap the next `resize()` applies (the governor lowers it). */
  dprCap: number;
  width: number;
  height: number;
  resize(): void;
  setAtlas(src: TexImageSource): void;
  render(): void;
  ready(): boolean;
  lost(): boolean;
}

interface ParallelCompile {
  COMPLETION_STATUS_KHR: number;
}

const MAX_WAIT_FRAMES = 60;

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader | null {
  const s = gl.createShader(type);
  if (s === null) {
    return null;
  }
  gl.shaderSource(s, src);
  gl.compileShader(s);
  return s;
}

function upload(gl: WebGL2RenderingContext, tex: WebGLTexture, source: TexImageSource): void {
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.pixelStorei(gl.UNPACK_PREMULTIPLY_ALPHA_WEBGL, true);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, source);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
}

function openContext(canvas: HTMLCanvasElement): WebGL2RenderingContext | null {
  try {
    return canvas.getContext("webgl2", {
      alpha: true,
      premultipliedAlpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      preserveDrawingBuffer: false,
      powerPreference: "high-performance",
      failIfMajorPerformanceCaveat: true,
    });
  } catch {
    return null;
  }
}

/** Starts compiling and linking the field program (the link may finish in the background). */
function startLink(ctx: WebGL2RenderingContext): WebGLProgram | null {
  const program = ctx.createProgram();
  const vs = compile(ctx, ctx.VERTEX_SHADER, FIELD_VERT);
  const fs = compile(ctx, ctx.FRAGMENT_SHADER, FIELD_FRAG);
  if (vs === null || fs === null) {
    return null;
  }
  ctx.attachShader(program, vs);
  ctx.attachShader(program, fs);
  ctx.linkProgram(program);
  return program;
}

interface Linker {
  /** True once the program is linked and in use; false while compiling or after a failed link. */
  ready(): boolean;
  broken(): boolean;
  at(name: string): WebGLUniformLocation | null;
}

/** Waits for the link without blocking: once per `ready()` call, at most `MAX_WAIT_FRAMES` times. */
function createLinker(ctx: WebGL2RenderingContext, program: WebGLProgram, atlasTex: WebGLTexture): Linker {
  const parallel = ctx.getExtension("KHR_parallel_shader_compile") as ParallelCompile | null;
  const loc = new Map<string, WebGLUniformLocation | null>();
  let waited = 0;
  let linked = false;
  let failed = false;
  const finish = (): void => {
    if (!ctx.getProgramParameter(program, ctx.LINK_STATUS)) {
      failed = true;
      return;
    }
    for (const n of FIELD_UNIFORMS) {
      loc.set(n, ctx.getUniformLocation(program, n));
    }
    ctx.useProgram(program);
    ctx.uniform1i(loc.get("uAtlas") ?? null, 0);
    ctx.activeTexture(ctx.TEXTURE0);
    ctx.bindTexture(ctx.TEXTURE_2D, atlasTex);
    ctx.disable(ctx.DEPTH_TEST);
    ctx.disable(ctx.BLEND);
    linked = true;
  };
  if (parallel === null) {
    finish();
  }
  return {
    ready() {
      if (linked || failed) {
        return linked;
      }
      waited += 1;
      const done = parallel !== null && ctx.getProgramParameter(program, parallel.COMPLETION_STATUS_KHR) === true;
      if (done || waited > MAX_WAIT_FRAMES) {
        finish();
      }
      return linked;
    },
    broken: () => failed,
    at: (name) => loc.get(name) ?? null,
  };
}

function drawField(ctx: WebGL2RenderingContext, link: Linker, gl: FieldGl, overlay: boolean): void {
  const { u } = gl;
  const d = gl.dpr;
  ctx.viewport(0, 0, gl.width, gl.height);
  ctx.uniform2f(link.at("uRes"), gl.width, gl.height);
  ctx.uniform1f(link.at("uTime"), u.time);
  ctx.uniform2f(link.at("uLens"), u.lensX * d, gl.height - u.lensY * d);
  ctx.uniform1f(link.at("uLensR"), u.lensR * d);
  ctx.uniform3f(link.at("uClick"), u.clickX * d, gl.height - u.clickY * d, u.clickAge);
  ctx.uniform1f(link.at("uScroll"), u.scroll * d);
  ctx.uniform1f(link.at("uVh"), gl.height);
  ctx.uniform1f(link.at("uWipe"), u.wipe);
  ctx.uniform1f(link.at("uOverlay"), overlay ? 1 : 0);
  ctx.uniform1f(link.at("uCell"), u.cell * d);
  ctx.uniform1f(link.at("uPage"), u.page);
  ctx.uniform1f(link.at("uHeroY"), u.heroY * d);
  ctx.uniform1f(link.at("uDpr"), d);
  ctx.drawArrays(ctx.TRIANGLES, 0, 3);
}

function initialUniforms(overlay: boolean): FieldUniforms {
  return {
    time: 6.5,
    lensX: 0,
    lensY: 0,
    lensR: 260,
    scroll: 0,
    wipe: 0,
    page: 1,
    heroY: 0,
    cell: overlay ? 17 : 12,
    clickX: 0,
    clickY: 0,
    clickAge: -1,
  };
}

/** `overlay` paints only the transition wipe (the veil). Returns null without a usable WebGL2. */
export function createFieldGl(canvas: HTMLCanvasElement, opts: { overlay: boolean; dprCap: number }): FieldGl | null {
  const ctx = openContext(canvas);
  const program = ctx === null ? null : startLink(ctx);
  if (ctx === null || program === null) {
    return null;
  }
  const atlasTex = ctx.createTexture();
  upload(ctx, atlasTex, canvas.ownerDocument.createElement("canvas"));
  const link = createLinker(ctx, program, atlasTex);
  if (link.broken()) {
    return null;
  }
  let contextLost = false;
  canvas.addEventListener("webglcontextlost", (e) => {
    e.preventDefault();
    contextLost = true;
  });
  const api: FieldGl = {
    canvas,
    u: initialUniforms(opts.overlay),
    dpr: 1,
    dprCap: opts.dprCap,
    width: 0,
    height: 0,
    resize() {
      const size = canvasSize(canvas.clientWidth, canvas.clientHeight, canvas.ownerDocument.defaultView?.devicePixelRatio ?? 1, api.dprCap);
      api.dpr = size.scale;
      if (size.w !== api.width || size.h !== api.height) {
        api.width = size.w;
        api.height = size.h;
        canvas.width = size.w;
        canvas.height = size.h;
      }
    },
    setAtlas(src) {
      if (!contextLost) {
        ctx.activeTexture(ctx.TEXTURE0);
        upload(ctx, atlasTex, src);
      }
    },
    ready: () => link.ready(),
    lost: () => contextLost || link.broken() || ctx.isContextLost(),
    render() {
      if (link.ready() && !contextLost) {
        drawField(ctx, link, api, opts.overlay);
      }
    },
  };
  api.resize();
  return api;
}
