/**
 * The data layer (full tier): faint labels on the tallest towers near the pointer, links between
 * them with travelling dots, and a pulse ring on clicks. It reads the same tower height as the
 * shader (towers.ts) and draws on a 2D canvas above the field. Cosmetic only.
 */
import { fieldColors } from "@mosaic/design-tokens";
import type { Click, FieldLayer } from "./bootField.ts";
import type { FieldGl } from "./gl.ts";
import type { Lens } from "./lens.ts";
import { apart, arc, bez, isFarCell, type Arc, type Cell } from "./linkGeometry.ts";
import { fr, h21, ss, towerH, type TowerParams } from "./towers.ts";

const MAX_NEAR = 12;
/** Far nodes are rare, fixed cells out past the raised area; they always link inward. */
const MAX_FAR = 3;
const PICK_EVERY_MS = 250;

const LINK = fieldColors.link.join(",");
const NODE = fieldColors.node.join(",");
const LABEL = fieldColors.label.join(",");
const RING = fieldColors.ring.join(",");

interface Item extends Cell {
  a: number;
  on: boolean;
  wait: number;
  far: boolean;
  kind: 0 | 1 | 2;
  /** undefined: not chosen yet; null: chosen, none. */
  parent?: Item | null | undefined;
  kids?: number;
  prog?: number;
  lwait?: number;
  speed?: number;
}

interface Point {
  x: number;
  y: number;
  h: number;
  al: number;
  f: number;
  dc: number;
  it: Item;
}

/** Per-frame values in device pixels, shared by the drawing steps. */
interface Frame {
  d: number;
  W: number;
  H: number;
  now: number;
  sx: number;
  sy: number;
  scroll: number;
  heroY: number;
  tp: TowerParams;
}

/** The text of a node: a short hash with a level, a delta or a count. */
export function label(kind: 0 | 1 | 2, i: number, j: number, h: number): string {
  if (kind === 0) {
    const hash = ((Math.imul(i, 73856093) ^ Math.imul(j, 19349663)) >>> 0).toString(16).slice(-4);
    return `${hash}  ${(h * 100).toFixed(1)}`;
  }
  if (kind === 1) {
    return `Δ +${(h * 12).toFixed(2)}`;
  }
  return `n=${String(Math.round(h * 512))}`;
}

/** A far node stays while it is on and within 1.3 R to 2.9 R of the lens. */
function keepFar(it: Item, dd: number, R: number): boolean {
  return it.on && it.far && dd > R * 1.3 && dd < R * 2.9;
}

/** `q` may become the parent of `p`: closer to the pointer, a free slot, not a loop, never far to far. */
function canParent(p: Point, q: Point): boolean {
  const blocked = q === p || q.dc > p.dc * 0.82 || (q.it.kids ?? 0) >= 2 || q.it.parent === p.it;
  return !blocked && !(p.it.far && q.it.far);
}

class DataLayer implements FieldLayer {
  private items: Item[] = [];
  private nextPick = 0;
  private prevLens: { x: number; y: number } | null = null;
  private prevNow = 0;
  private click: Click | null = null;

  constructor(
    private readonly canvas: HTMLCanvasElement,
    private readonly ctx: CanvasRenderingContext2D,
  ) {}

  setClick(c: Click | null): void {
    this.click = c;
  }

  dispose(): void {
    this.canvas.remove();
    this.items = [];
  }

  draw(now: number, lens: Lens, gl: FieldGl): void {
    const { canvas, ctx } = this;
    const d = gl.dpr;
    const W = gl.width;
    const H = gl.height;
    if (canvas.width !== W || canvas.height !== H) {
      canvas.width = W;
      canvas.height = H;
    }
    ctx.clearRect(0, 0, W, H);
    const tp: TowerParams = { cs: gl.u.cell * d, t: gl.u.time, lx: lens.x * d, ly: H - lens.y * d, R: gl.u.lensR * d, click: this.click };
    const scroll = (gl.canvas.ownerDocument.defaultView?.scrollY ?? 0) * d;
    const f: Frame = { d, W, H, now, sx: lens.x * d, sy: lens.y * d, scroll, heroY: gl.u.heroY * d, tp };
    if (now > this.nextPick) {
      this.pick(f);
      this.nextPick = now + PICK_EVERY_MS;
    }
    const pts = this.project(f);
    this.prune();
    this.drawLinks(f, pts);
    this.drawNodes(f, pts);
    this.drawPulse(f);
  }

  /** Near nodes are the tallest towers around the lens, kept while they stay tall. */
  private nearNodes(f: Frame): Cell[] {
    const { cs, lx, ly, R } = f.tp;
    const ci = Math.floor(lx / cs);
    const cj = Math.floor(ly / cs);
    const rad = Math.ceil((R * 1.7) / cs);
    const found: (Cell & { h: number })[] = [];
    for (let j = cj - rad; j <= cj + rad; j += 1) {
      for (let i = ci - rad; i <= ci + rad; i += 1) {
        const h = towerH(i, j, f.tp);
        if (h > 0.14) {
          found.push({ i, j, h });
        }
      }
    }
    found.sort((a, b) => b.h - a.h);
    const keep: Cell[] = this.items.filter((it) => it.on && !it.far && towerH(it.i, it.j, f.tp) > 0.1).reduce<Cell[]>((acc, it) => {
      return acc.length < MAX_NEAR && apart(acc, it) ? [...acc, it] : acc;
    }, []);
    for (const c of found) {
      if (keep.length < MAX_NEAR && apart(keep, c)) {
        keep.push(c);
      }
    }
    return keep;
  }

  /** Far nodes: fixed rare cells between 1.5 R and 2.7 R from the lens. */
  private farNodes(f: Frame): (Cell & { far?: boolean })[] {
    const { cs, lx, ly, R } = f.tp;
    const dist = (c: Cell): number => Math.hypot((c.i + 0.5) * cs - lx, (c.j + 0.5) * cs - ly);
    const far: (Cell & { far?: boolean })[] = [];
    for (const it of this.items) {
      if (keepFar(it, dist(it), R) && far.length < MAX_FAR && apart(far, it, 10)) {
        far.push(it);
      }
    }
    const ci = Math.floor(lx / cs);
    const cj = Math.floor(ly / cs);
    const reach = Math.ceil((R * 2.7) / cs);
    for (let j = cj - reach; j <= cj + reach && far.length < MAX_FAR; j += 1) {
      for (let i = ci - reach; i <= ci + reach && far.length < MAX_FAR; i += 1) {
        if (isFarCell(i, j, dist({ i, j }), R) && apart(far, { i, j }, 10)) {
          far.push({ i, j, far: true });
        }
      }
    }
    return far;
  }

  private pick(f: Frame): void {
    const all = [...this.nearNodes(f), ...this.farNodes(f)];
    const keys = new Set(all.map((k) => `${String(k.i)},${String(k.j)}`));
    for (const it of this.items) {
      it.on = keys.has(`${String(it.i)},${String(it.j)}`);
    }
    for (const k of all) {
      if (!this.items.some((it) => it.i === k.i && it.j === k.j)) {
        this.items.push({
          i: k.i,
          j: k.j,
          a: 0,
          on: true,
          wait: h21(k.i + 8.8, k.j + 1.4) * 0.35,
          far: "far" in k && k.far === true,
          kind: Math.min(2, Math.floor(h21(k.i + 5.1, k.j + 2.3) * 3)) as 0 | 1 | 2,
        });
      }
    }
  }

  /** The shader's fades: quieter below the hero, gone behind the hero copy and under the header. */
  private fade(f: Frame, gx: number, gy: number): number {
    const { W, H, d, scroll, heroY } = f;
    const hero = 1 - ss(H * 0.8, H * 1.6, H - gy + scroll);
    const center = 1 - ss(0.1, 0.85, Math.hypot(((gx - W / 2) / H) * 0.72, (gy - (H - (heroY - scroll))) / H));
    const header = 1 - ss(H - 150 * d, H - 60 * d, gy);
    return (0.5 + 0.5 * hero) * (1 - 0.92 * center * hero) * header;
  }

  /** Fades nodes in and out and projects each onto the screen as the camera below the lens sees it. */
  private project(f: Frame): Point[] {
    const { tp } = f;
    const camx = tp.lx;
    const camy = tp.ly - tp.R * 0.9;
    const pts: Point[] = [];
    for (const it of this.items) {
      const holding = (it.prog ?? 0) > 0.002 || this.items.some((c) => c.parent === it && (c.prog ?? 0) > 0.002);
      const up = it.on || holding;
      if (up && it.wait > 0) {
        it.wait -= 1 / 60;
      } else {
        it.a += ((up ? 1 : 0) - it.a) * (up ? 0.08 : 0.04);
      }
      const h = towerH(it.i, it.j, tp);
      const cx = (it.i + 0.5) * tp.cs;
      const cy = (it.j + 0.5) * tp.cs;
      const dx = cx - camx;
      const dy = cy - camy;
      const dd = Math.max(1, Math.hypot(dx, dy));
      const k = (Math.min((dd / tp.R) * h * 2.4, 1.95) * tp.cs) / dd;
      const gx = cx + dx * k;
      const gy = cy + dy * k;
      const f0 = this.fade(f, gx, gy);
      if (f0 > 0.01) {
        pts.push({ x: gx, y: f.H - gy, h, al: it.a * f0, f: f0, dc: 0, it });
      }
    }
    return pts;
  }

  private prune(): void {
    const removed = this.items.filter((it) => !(it.on || it.a > 0.02));
    this.items = this.items.filter((it) => it.on || it.a > 0.02);
    for (const r of removed) {
      if (r.parent?.kids !== undefined) {
        r.parent.kids -= 1;
      }
    }
    for (const it of this.items) {
      if (it.parent !== undefined && it.parent !== null && removed.includes(it.parent)) {
        it.parent = undefined;
        it.prog = 0;
      }
    }
  }

  /** The nearest node that lies closer to the pointer and roughly toward it, or null. */
  private bestParent(p: Point, pts: readonly Point[], f: Frame): Item | null {
    const it = p.it;
    let best: Item | null = null;
    let bestD = Infinity;
    for (const q of pts) {
      if (!canParent(p, q)) {
        continue;
      }
      const vx = q.x - p.x;
      const vy = q.y - p.y;
      const dist = Math.hypot(vx, vy) || 1;
      const toward = (vx * (f.sx - p.x) + vy * (f.sy - p.y)) / (dist * (p.dc || 1));
      if (toward >= (it.far ? 0.2 : 0.35) && dist < bestD) {
        bestD = dist;
        best = q.it;
      }
    }
    return best;
  }

  /**
   * Links are chosen once per node and kept: the first time a node is clearly visible it picks
   * its nearest neighbour that lies closer to the pointer and roughly toward it (at most two
   * children each; about a third of the nodes stay unlinked).
   */
  private chooseParent(p: Point, pts: readonly Point[], f: Frame): void {
    const it = p.it;
    it.prog = 0;
    it.parent = null;
    it.lwait = 0.06 + 0.4 * h21(it.i + 2.6, it.j + 9.1);
    it.speed = 0.75 + 0.5 * h21(it.i + 6.2, it.j + 3.7);
    if (!it.far && h21(it.i + 3.3, it.j + 7.7) > 0.68) {
      return;
    }
    it.parent = this.bestParent(p, pts, f);
    if (it.parent !== null) {
      it.parent.kids = (it.parent.kids ?? 0) + 1;
    }
  }

  private drawLinks(f: Frame, pts: Point[]): void {
    const { ctx } = this;
    const { d } = f;
    ctx.lineWidth = Math.max(1, d * 0.7);
    const moved = this.prevLens === null ? 0 : Math.hypot(f.sx - this.prevLens.x, f.sy - this.prevLens.y);
    this.prevLens = { x: f.sx, y: f.sy };
    const dts = this.prevNow > 0 ? Math.min(0.05, (f.now - this.prevNow) / 1000) : 0;
    this.prevNow = f.now;
    const byItem = new Map(pts.map((p) => [p.it, p]));
    for (const p of pts) {
      p.dc = Math.hypot(p.x - f.sx, p.y - f.sy);
    }
    for (const p of pts) {
      if (p.it.parent === undefined && p.it.a >= 0.12) {
        this.chooseParent(p, pts, f);
      }
    }
    for (const p of pts) {
      const q = p.it.parent === undefined || p.it.parent === null ? undefined : byItem.get(p.it.parent);
      if (q !== undefined) {
        this.drawLink(f, p, q, { dts, moved });
      }
    }
  }

  /** Grows a live link toward 1 (faster while the pointer moves) and retracts a dying one. */
  private advance(p: Point, q: Point, step: { dts: number; moved: number }, d: number): number {
    const it = p.it;
    const len = Math.hypot(q.x - p.x, q.y - p.y) || 1;
    const prog = it.prog ?? 0;
    it.prog = it.on && q.it.on
      ? Math.min(1, prog + (step.dts * (it.speed ?? 1)) / (0.25 + len / (2600 * d)) + step.moved / (len * 1.4))
      : Math.max(0, prog - step.dts / 0.55);
    return it.prog;
  }

  private drawLink(f: Frame, p: Point, q: Point, step: { dts: number; moved: number }): void {
    const it = p.it;
    if (it.on && q.it.on && (it.lwait ?? 0) > 0) {
      it.lwait = (it.lwait ?? 0) - step.dts;
      return;
    }
    const prog = this.advance(p, q, step, f.d);
    if (prog <= 0.002) {
      return;
    }
    const c = arc(p, q, (it.far ? 0.3 : 0.16) + 0.12 * h21(it.i + 4.1, it.j + 0.3));
    const al = Math.min(p.f, q.f);
    this.strokeCurve(p, c, q, { prog, al });
    this.drawDot(f, { p, c, q }, { prog, al });
  }

  private strokeCurve(p: Point, c: Arc, q: Point, at: { prog: number; al: number }): void {
    const { ctx } = this;
    ctx.strokeStyle = `rgba(${LINK},${(0.16 * at.al).toFixed(3)})`;
    ctx.beginPath();
    ctx.moveTo(p.x, p.y);
    const steps = Math.max(2, Math.ceil(28 * at.prog));
    for (let k = 1; k <= steps; k += 1) {
      const v = bez(p, c, q, (at.prog * k) / steps);
      ctx.lineTo(v.x, v.y);
    }
    ctx.stroke();
  }

  /** The travelling dot: the growing tip while the link draws in, then a slow pass along it. */
  private drawDot(f: Frame, link: { p: Point; c: Arc; q: Point }, at: { prog: number; al: number }): void {
    const { ctx } = this;
    const { p, c, q } = link;
    const head = at.prog < 1 ? at.prog : fr(f.tp.t * 0.18 + h21(p.it.i, p.it.j));
    const v = bez(p, c, q, head);
    const pa = at.prog < 1 ? 0.6 : 0.55 * Math.sin(Math.PI * head);
    ctx.fillStyle = `rgba(${NODE},${(pa * at.al).toFixed(3)})`;
    ctx.beginPath();
    ctx.arc(v.x, v.y, (at.prog < 1 ? 1.4 : 1.1) * f.d, 0, Math.PI * 2);
    ctx.fill();
  }

  private drawNodes(f: Frame, pts: readonly Point[]): void {
    const { ctx } = this;
    const { d } = f;
    ctx.font = `500 ${String(9 * d)}px 'IBM Plex Mono', ui-monospace, monospace`;
    ctx.textBaseline = "alphabetic";
    const labelled = new Set([...pts].filter((p) => !p.it.far && p.al > 0.01).sort((a, b) => b.h - a.h).slice(0, 4));
    for (const p of pts) {
      if (p.al < 0.01) {
        continue;
      }
      const s = (labelled.has(p) ? 4 : 2.5) * d * (0.25 + 0.75 * p.it.a);
      ctx.strokeStyle = `rgba(${NODE},${(0.42 * p.al).toFixed(3)})`;
      ctx.strokeRect(p.x - s / 2, p.y - s / 2, s, s);
      if (labelled.has(p)) {
        this.drawLabel(f, p, s);
      }
    }
  }

  private drawLabel(f: Frame, p: Point, s: number): void {
    const { ctx } = this;
    const { d } = f;
    const text = label(p.it.kind, p.it.i, p.it.j, p.h);
    const tw = ctx.measureText(text).width;
    const ex = p.x + 11 * d;
    const ey = p.y - 11 * d;
    ctx.strokeStyle = `rgba(${LINK},${(0.22 * p.al).toFixed(3)})`;
    ctx.beginPath();
    ctx.moveTo(p.x + s / 2, p.y - s / 2);
    ctx.lineTo(ex, ey);
    ctx.lineTo(ex + tw + 10 * d, ey);
    ctx.stroke();
    ctx.fillStyle = `rgba(${LINK},${(0.32 * p.al).toFixed(3)})`;
    ctx.fillRect(ex + 2 * d, ey - 3 * d - p.h * 9 * d, 2 * d, p.h * 9 * d);
    ctx.fillStyle = `rgba(${LABEL},${(0.46 * p.al).toFixed(3)})`;
    ctx.fillText(text, ex + 7 * d, ey - 3 * d);
  }

  /** The click pulse: one faint ring running with the shockwave. */
  private drawPulse(f: Frame): void {
    const { click, ctx } = this;
    if (click === null || click.age < 0 || click.age >= 1.6) {
      return;
    }
    const { d, H } = f;
    const ca = (1 - click.age / 1.6) * this.fade(f, click.x, click.y);
    ctx.strokeStyle = `rgba(${RING},${(0.22 * ca).toFixed(3)})`;
    ctx.setLineDash([3 * d, 5 * d]);
    ctx.beginPath();
    ctx.arc(click.x, H - click.y, click.age * f.tp.R * 1.5, 0, Math.PI * 2);
    ctx.stroke();
    ctx.setLineDash([]);
  }
}

/** Creates the layer, or null when the platform has no 2D context. */
export function createDataLayer(doc: Document): FieldLayer | null {
  const canvas = doc.createElement("canvas");
  canvas.className = "field-data";
  canvas.setAttribute("aria-hidden", "true");
  const ctx = canvas.getContext("2d");
  if (ctx === null) {
    return null;
  }
  doc.body.insertBefore(canvas, doc.querySelector(".field-still"));
  return new DataLayer(canvas, ctx);
}
