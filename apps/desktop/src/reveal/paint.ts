/**
 * Display lists to 2D canvases (study L476-549, no html2canvas, no foreignObject). One screen
 * becomes a static base plus one sprite per choreography group, so a frame can show exactly
 * the poses the DOM has. Cosmetic only; `collect.ts` has already left every secret out.
 */
import { GROUP_SELECTOR, POP_SELECTOR, type Pose } from "./choreography.ts";
import { collect, domMeasure } from "./collect.ts";
import type { Collected, GroupKind, Op, Rect, SvgItem } from "./ops.ts";
import { clamp } from "./timing.ts";

export interface DisplayGroup {
  el: HTMLElement;
  parent: number;
  kind: GroupKind;
  origin: boolean;
  /** Centre in stage px. */
  cx: number;
  cy: number;
  /** The sprite's rectangle: the group's plus padding for the blur. */
  b: Rect;
  cv: HTMLCanvasElement | null;
  /** The blurred sprite (items only). */
  cvB: HTMLCanvasElement | null;
}

export interface Display {
  /** Stage size in CSS px and the scale of the canvases. */
  W: number;
  H: number;
  S: number;
  groups: DisplayGroup[];
  /** The composed picture; null when only the group geometry was asked for. */
  cv: HTMLCanvasElement | null;
  baseCv: HTMLCanvasElement | null;
  ctx: CanvasRenderingContext2D | null;
}

export const BLUR_MAX = 4;
const SPRITE_PAD = 12;

const newCanvas = (w: number, h: number): HTMLCanvasElement =>
  Object.assign(document.createElement("canvas"), {
    width: Math.max(1, Math.ceil(w)),
    height: Math.max(1, Math.ceil(h)),
  });

function context(canvas: HTMLCanvasElement, opaque: boolean): CanvasRenderingContext2D {
  const ctx = canvas.getContext("2d", opaque ? { alpha: false } : undefined);
  if (ctx === null) {
    throw new Error("reveal: no 2d context");
  }
  return ctx;
}

function pathOf(item: SvgItem): Path2D {
  const at = (name: string): number => Number(item.attrs[name]) || 0;
  if (item.kind === "path") {
    return new Path2D(item.attrs["d"] ?? "");
  }
  const path = new Path2D();
  if (item.kind === "rect") {
    path.roundRect(at("x"), at("y"), at("width"), at("height"), at("rx"));
  } else {
    path.arc(at("cx"), at("cy"), at("r"), 0, Math.PI * 2);
  }
  return path;
}

type Ops<K extends Op["k"]> = Extract<Op, { k: K }>;

/** Where a canvas sits on the stage (its origin, in CSS px) and how much it is scaled. */
interface View {
  ox: number;
  oy: number;
  S: number;
}

function drawText(ctx: CanvasRenderingContext2D, o: Ops<"text">, ox: number, oy: number): void {
  ctx.font = o.font;
  if ("letterSpacing" in ctx) {
    ctx.letterSpacing = o.ls === "normal" ? "0px" : o.ls;
  }
  ctx.textBaseline = "alphabetic";
  const m = ctx.measureText(o.text || "x");
  const x = o.x - ox;
  const y = o.y - oy + (o.h - (m.fontBoundingBoxAscent + m.fontBoundingBoxDescent)) / 2 + m.fontBoundingBoxAscent;
  if (o.sh !== null) {
    ctx.fillStyle = o.sh.c;
    ctx.fillText(o.text, x + o.sh.x, y + o.sh.y);
  }
  ctx.fillStyle = o.c;
  ctx.fillText(o.text, x, y);
}

function drawSvg(ctx: CanvasRenderingContext2D, o: Ops<"svg">, ox: number, oy: number): void {
  ctx.save();
  ctx.translate(o.r.x - ox, o.r.y - oy);
  ctx.scale(o.r.w / o.vb[2], o.r.h / o.vb[3]);
  ctx.translate(-o.vb[0], -o.vb[1]);
  ctx.lineCap = "round";
  ctx.lineJoin = "round";
  for (const item of o.items) {
    const path = pathOf(item);
    if (item.fill !== null) {
      ctx.fillStyle = item.fill;
      ctx.fill(path);
    }
    if (item.stroke !== null) {
      ctx.strokeStyle = item.stroke;
      ctx.lineWidth = item.lw;
      ctx.stroke(path);
    }
  }
  ctx.restore();
}

/** A live canvas or image, snapped to whole device pixels so it stays crisp. */
function drawLive(ctx: CanvasRenderingContext2D, o: Ops<"live">, view: View): void {
  const snap = (v: number): number => Math.round(v * view.S) / view.S;
  const x = snap(o.r.x - view.ox);
  const y = snap(o.r.y - view.oy);
  const w = snap(o.r.w);
  const h = snap(o.r.h);
  if (o.rad > 0) {
    ctx.save();
    ctx.beginPath();
    ctx.roundRect(x, y, w, h, o.rad);
    ctx.clip();
    ctx.drawImage(o.el, x, y, w, h);
    ctx.restore();
  } else {
    ctx.drawImage(o.el, x, y, w, h);
  }
}

function drawShape(ctx: CanvasRenderingContext2D, o: Ops<"fill" | "rect" | "stroke">, ox: number, oy: number): void {
  if (o.k === "stroke") {
    ctx.strokeStyle = o.c;
    ctx.lineWidth = o.w;
    ctx.beginPath();
    ctx.roundRect(o.r.x - ox + o.w / 2, o.r.y - oy + o.w / 2, o.r.w - o.w, o.r.h - o.w, Math.max(0, o.rad - o.w / 2));
    ctx.stroke();
    return;
  }
  ctx.fillStyle = o.c;
  if (o.k === "rect") {
    ctx.fillRect(o.r.x - ox, o.r.y - oy, o.r.w, o.r.h);
    return;
  }
  ctx.beginPath();
  ctx.roundRect(o.r.x - ox, o.r.y - oy, o.r.w, o.r.h, o.rad);
  ctx.fill();
}

/** Paints an op list into `ctx`, whose origin is at (ox, oy) of the stage (study L476-497). */
function drawOps(ctx: CanvasRenderingContext2D, ops: readonly Op[], view: View): void {
  const { ox, oy } = view;
  for (const o of ops) {
    switch (o.k) {
      case "clip":
        ctx.save();
        ctx.beginPath();
        ctx.rect(o.r.x - ox, o.r.y - oy, o.r.w, o.r.h);
        ctx.clip();
        break;
      case "unclip":
        ctx.restore();
        break;
      case "text":
        ctx.globalAlpha = o.a;
        drawText(ctx, o, ox, oy);
        break;
      case "svg":
        ctx.globalAlpha = o.a;
        drawSvg(ctx, o, ox, oy);
        break;
      case "live":
        ctx.globalAlpha = o.a;
        drawLive(ctx, o, view);
        break;
      default:
        ctx.globalAlpha = o.a;
        drawShape(ctx, o, ox, oy);
    }
  }
  ctx.globalAlpha = 1;
}

/** Group elements are measured at rest: their poses are cleared while the page is measured. */
function atRest<T>(root: Element, fn: () => T): T {
  const els = [...root.querySelectorAll<HTMLElement>(`${GROUP_SELECTOR}, ${POP_SELECTOR}`)];
  const saved = els.map((el) => [el.style.opacity, el.style.transform, el.style.filter] as const);
  for (const el of els) {
    el.style.opacity = "";
    el.style.transform = "";
    el.style.filter = "";
  }
  try {
    return fn();
  } finally {
    els.forEach((el, i) => {
      [el.style.opacity, el.style.transform, el.style.filter] = saved[i] ?? ["", "", ""];
    });
  }
}

function spriteOf(group: Collected["groups"][number], S: number): DisplayGroup {
  const { rect } = group;
  const b = { x: rect.x - SPRITE_PAD, y: rect.y - SPRITE_PAD, w: rect.w + 2 * SPRITE_PAD, h: rect.h + 2 * SPRITE_PAD };
  const cv = newCanvas(b.w * S, b.h * S);
  const ctx = context(cv, false);
  ctx.setTransform(S, 0, 0, S, 0, 0);
  drawOps(ctx, group.ops, { ox: b.x, oy: b.y, S });
  let cvB: HTMLCanvasElement | null = null;
  if (group.kind === "item") {
    cvB = newCanvas(cv.width, cv.height);
    const blur = context(cvB, false);
    blur.filter = `blur(${(BLUR_MAX * S).toFixed(1)}px)`;
    blur.drawImage(cv, 0, 0);
  }
  return {
    el: group.el,
    parent: group.parent,
    kind: group.kind,
    origin: group.origin,
    cx: rect.x + rect.w / 2,
    cy: rect.y + rect.h / 2,
    b,
    cv,
    cvB,
  };
}

function geometryOf(group: Collected["groups"][number]): DisplayGroup {
  const { rect } = group;
  return {
    el: group.el,
    parent: group.parent,
    kind: group.kind,
    origin: group.origin,
    cx: rect.x + rect.w / 2,
    cy: rect.y + rect.h / 2,
    b: rect,
    cv: null,
    cvB: null,
  };
}

function build(root: HTMLElement, stage: HTMLElement, S: number, opts: { groups: boolean; paint: boolean }): Display {
  const rect = stage.getBoundingClientRect();
  const collected = atRest(root, () => collect(root, domMeasure(rect), { groups: opts.groups }));
  const d: Display = { W: rect.width, H: rect.height, S, groups: [], cv: null, baseCv: null, ctx: null };
  if (!opts.paint) {
    d.groups = collected.groups.map(geometryOf);
    return d;
  }
  const baseCv = newCanvas(d.W * S, d.H * S);
  const base = context(baseCv, true);
  base.setTransform(S, 0, 0, S, 0, 0);
  base.fillStyle = getComputedStyle(root).backgroundColor;
  base.fillRect(0, 0, d.W, d.H);
  drawOps(base, collected.base, { ox: 0, oy: 0, S });
  d.groups = collected.groups.map((g) => spriteOf(g, S));
  d.baseCv = baseCv;
  d.cv = newCanvas(d.W * S, d.H * S);
  d.ctx = context(d.cv, true);
  return d;
}

/**
 * The display of `root` as laid out inside `stage`, scaled by `S`. `null` when anything
 * fails (no 2D context, a canvas the platform refuses): the caller falls back to no glass.
 * With `paint: false` only the group geometry is built (the lens and fade unlocks).
 */
export function buildDisplay(
  root: HTMLElement,
  stage: HTMLElement,
  S: number,
  opts: { groups: boolean; paint: boolean },
): Display | null {
  try {
    return build(root, stage, S, opts);
  } catch {
    return null;
  }
}

interface Matrix {
  k: number;
  ox: number;
  oy: number;
  a: number;
  blur: number;
}

/** Composes the base and every group sprite at its pose into `d.cv` (study L525-549). */
export function compose(d: Display, poses: readonly Pose[]): void {
  const { ctx, baseCv, S } = d;
  if (ctx === null || baseCv === null) {
    return;
  }
  ctx.setTransform(1, 0, 0, 1, 0, 0);
  ctx.globalAlpha = 1;
  ctx.drawImage(baseCv, 0, 0);
  const matrices: Matrix[] = [];
  d.groups.forEach((g, i) => {
    const p = poses[i];
    const parent = g.parent >= 0 ? matrices[g.parent] : undefined;
    const par = parent ?? { k: 1, ox: 0, oy: 0, a: 1, blur: 0 };
    if (p === undefined || g.cv === null) {
      return;
    }
    const ocx = g.cx * (1 - p.s) + p.tx;
    const ocy = g.cy * (1 - p.s) + p.ty;
    const m: Matrix = {
      k: par.k * p.s,
      ox: par.k * ocx + par.ox,
      oy: par.k * ocy + par.oy,
      a: par.a * p.o,
      blur: Math.hypot(par.blur, p.blur),
    };
    matrices[i] = m;
    if (m.a <= 0.004) {
      return;
    }
    const kb = clamp(m.blur / BLUR_MAX);
    const dx = (m.k * g.b.x + m.ox) * S;
    const dy = (m.k * g.b.y + m.oy) * S;
    const dw = g.b.w * m.k * S;
    const dh = g.b.h * m.k * S;
    const sharp = g.cvB === null ? 1 : 1 - kb;
    if (kb < 0.98 || g.cvB === null) {
      ctx.globalAlpha = m.a * sharp;
      ctx.drawImage(g.cv, dx, dy, dw, dh);
    }
    if (kb > 0.02 && g.cvB !== null) {
      ctx.globalAlpha = m.a * Math.min(1, kb * 1.15);
      ctx.drawImage(g.cvB, dx, dy, dw, dh);
    }
  });
  ctx.globalAlpha = 1;
}

/** Frees the canvases at once (a canvas holds its pixels until it is collected). */
export function releaseDisplay(d: Display): void {
  const free = (canvas: HTMLCanvasElement | null): void => {
    if (canvas !== null) {
      canvas.width = 0;
      canvas.height = 0;
    }
  };
  free(d.cv);
  free(d.baseCv);
  for (const g of d.groups) {
    free(g.cv);
    free(g.cvB);
  }
  d.groups = [];
  d.cv = null;
  d.baseCv = null;
  d.ctx = null;
}
