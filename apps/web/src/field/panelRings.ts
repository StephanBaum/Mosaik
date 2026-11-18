/**
 * Panel glyph rings (full and lite tiers): panels turn into numbers where a ring passes. A press
 * sends a ring of hex glyphs out from the pointer with a long fading tail, and the panel gives
 * a little and springs back. One shared canvas between the content and the sticky header,
 * painted only while a ring runs. Cosmetic only.
 */
import { fieldColors } from "@mosaic/design-tokens";
import { elasticOut, power2Out } from "../anim/ease.ts";
import { ticker } from "../anim/ticker.ts";
import { timeline, type Timeline } from "../anim/timeline.ts";

const SELECTOR = ".tile, .plan, .card, .platform__link, .step-card, .see__card";
const HEX = "0123456789abcdef";
const CELL = 11;
const DURATION = 1.35;
const STRENGTH = 1.4;
const COVER = fieldColors.cover.join(",");

/**
 * Brightness of a cell at distance `dd` from the press for a ring of radius `rad`: a narrow
 * band outside the ring (`front`), and a long tail inside it (`tail`).
 */
export function ringBand(dd: number, rad: number, front: number, tail: number): number {
  if (dd > rad) {
    return Math.exp(-(((dd - rad) / front) ** 2));
  }
  return Math.exp(-(rad - dd) / tail) * (0.35 + 0.65 * Math.exp(-(((rad - dd) / front) ** 2)) + 0.25);
}

interface Ring {
  x: number;
  y: number;
  t0: number;
  strength: number;
  dur: number;
  radius: number;
}

function hash(a: number, b: number): number {
  let h = Math.imul(a | 0, 374761393) + Math.imul(b | 0, 668265263);
  h = Math.imul(h ^ (h >>> 13), 1274126177);
  return ((h ^ (h >>> 16)) >>> 0) / 4294967296;
}

/** Colour channels of the film gradient the foil uses, as `r,g,b`. */
function film(hue: number): string {
  return [0, 0.33, 0.67].map((o) => String(Math.round(255 * (0.6 + 0.4 * (0.5 + 0.5 * Math.cos(6.2831853 * (hue + o))))))).join(",");
}

class PanelRings {
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;
  private readonly active = new Map<Element, Ring>();
  private readonly springs = new Map<Element, Timeline>();
  private stopTick: (() => void) | null = null;

  constructor(private readonly doc: Document) {
    doc.addEventListener("pointerdown", this.onDown, { passive: true });
  }

  dispose(): void {
    this.doc.removeEventListener("pointerdown", this.onDown);
    this.stopTick?.();
    this.stopTick = null;
    for (const tl of this.springs.values()) {
      tl.kill();
    }
    this.springs.clear();
    this.active.clear();
    this.canvas?.remove();
    this.canvas = null;
    this.ctx = null;
  }

  private readonly onDown = (e: PointerEvent): void => {
    if (e.button !== 0 || this.doc.documentElement.dataset["motion"] === "off") {
      return;
    }
    const el = e.target instanceof Element ? e.target.closest(SELECTOR) : null;
    if (el !== null) {
      this.start(el, e);
      this.push(el, e);
    }
  };

  private ensure(): CanvasRenderingContext2D | null {
    if (this.ctx !== null) {
      return this.ctx;
    }
    const canvas = this.doc.createElement("canvas");
    canvas.className = "panel-fx";
    canvas.setAttribute("aria-hidden", "true");
    const ctx = canvas.getContext("2d");
    if (ctx === null) {
      return null;
    }
    this.doc.body.append(canvas);
    this.canvas = canvas;
    this.ctx = ctx;
    return ctx;
  }

  private start(el: Element, e: PointerEvent): void {
    if (this.ensure() === null) {
      return;
    }
    const now = performance.now();
    const prev = this.active.get(el);
    if (prev !== undefined && STRENGTH <= prev.strength && now - prev.t0 < 450) {
      return;
    }
    const r = el.getBoundingClientRect();
    const view = this.doc.defaultView;
    this.active.set(el, {
      x: e.clientX - r.left,
      y: e.clientY - r.top,
      t0: now,
      strength: STRENGTH,
      dur: DURATION,
      radius: parseFloat(view?.getComputedStyle(el).borderTopLeftRadius ?? "0") || 0,
    });
    this.stopTick ??= ticker.add((t) => {
      this.frame(t);
    });
  }

  /** The press spring: a short give toward the press, then an elastic return. */
  private push(el: Element, e: PointerEvent): void {
    const target = el as HTMLElement;
    const r = el.getBoundingClientRect();
    const ux = Math.max(-1, Math.min(1, (e.clientX - (r.left + r.width / 2)) / (r.width / 2)));
    const uy = Math.max(-1, Math.min(1, (e.clientY - (r.top + r.height / 2)) / (r.height / 2)));
    const keep = target.style.transitionProperty;
    target.style.transitionProperty = "border-color, background-color, color";
    this.springs.get(el)?.kill();
    const tl = timeline({
      onComplete: () => {
        target.style.removeProperty("transform");
        target.style.transitionProperty = keep;
        this.springs.delete(el);
      },
    });
    tl.to(el, { x: -ux * 2.5, y: -uy * 2.5, scale: 1.012, rotateY: -ux * 1.6, rotateX: uy * 1.6 }, { duration: 0.16, ease: power2Out });
    tl.to(el, { x: 0, y: 0, scale: 1, rotateX: 0, rotateY: 0 }, { duration: 1.1, ease: elasticOut(1, 0.42) });
    this.springs.set(el, tl);
    tl.play();
  }

  private frame(now: number): void {
    const { ctx, canvas } = this;
    const view = this.doc.defaultView;
    if (ctx === null || canvas === null || view === null) {
      return;
    }
    const dpr = Math.min(view.devicePixelRatio, 1.5);
    const w = Math.round(view.innerWidth * dpr);
    const h = Math.round(view.innerHeight * dpr);
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
    }
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, view.innerWidth, view.innerHeight);
    ctx.font = "500 9.5px 'IBM Plex Mono', ui-monospace, monospace";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    for (const [el, ring] of this.active) {
      const age = Math.max(0, (now - ring.t0) / 1000);
      if (age > ring.dur || !el.isConnected) {
        this.active.delete(el);
      } else {
        this.paintRing(ctx, el.getBoundingClientRect(), ring, { age, now, vh: view.innerHeight });
      }
    }
    if (this.active.size === 0) {
      ctx.clearRect(0, 0, view.innerWidth, view.innerHeight);
      this.stopTick?.();
      this.stopTick = null;
    }
  }

  private paintRing(ctx: CanvasRenderingContext2D, r: DOMRect, s: Ring, at: { age: number; now: number; vh: number }): void {
    if (r.bottom < 0 || r.top > at.vh) {
      return;
    }
    const far = Math.max(Math.hypot(s.x, s.y), Math.hypot(r.width - s.x, s.y), Math.hypot(s.x, r.height - s.y), Math.hypot(r.width - s.x, r.height - s.y));
    const p = at.age / s.dur;
    const rad = (1 - Math.pow(1 - p, 2.4)) * (far + 24);
    const life = Math.pow(1 - p, 0.9);
    const front = 9 * s.strength;
    const tail = 70 * s.strength;
    const peak = s.strength > 1 ? 0.9 : 0.5;
    ctx.save();
    ctx.beginPath();
    ctx.roundRect(r.left, r.top, r.width, r.height, s.radius);
    ctx.clip();
    const tick = Math.floor(at.now / 55);
    const cols = Math.ceil(r.width / CELL);
    const rows = Math.ceil(r.height / CELL);
    for (let j = 0; j < rows; j += 1) {
      for (let i = 0; i < cols; i += 1) {
        const dx = (i + 0.5) * CELL - s.x;
        const dy = (j + 0.5) * CELL - s.y;
        const dd = Math.hypot(dx, dy) || 1;
        const band = ringBand(dd, rad, front, tail);
        if (band < 0.06) {
          continue;
        }
        const a = band * life * peak;
        ctx.fillStyle = `rgba(${COVER},${(a * 0.8).toFixed(3)})`;
        ctx.fillRect(r.left + i * CELL, r.top + j * CELL, CELL, CELL);
        const push = band * 2.5 * s.strength;
        ctx.fillStyle = `rgba(${film(dd / 240 + at.now / 5000)},${a.toFixed(3)})`;
        const glyph = HEX.charAt(Math.floor(hash(i + tick * 7, j * 31 + tick) * 16));
        ctx.fillText(glyph, r.left + (i + 0.5) * CELL + (dx / dd) * push, r.top + (j + 0.5) * CELL + (dy / dd) * push);
      }
    }
    ctx.restore();
  }
}

export function createPanelRings(doc: Document): { dispose(): void } {
  return new PanelRings(doc);
}
