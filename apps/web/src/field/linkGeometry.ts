/** Pure geometry of the data layer: cell spacing, far cells and the upward-bowed link curves. */
import { h21 } from "./towers.ts";

export interface Cell {
  i: number;
  j: number;
}

export interface Xy {
  x: number;
  y: number;
}

export const apart = (keep: readonly Cell[], c: Cell, k0 = 6): boolean =>
  keep.every((k) => Math.abs(k.i - c.i) + Math.abs(k.j - c.j) >= k0);

export interface Arc {
  ax: number;
  ay: number;
  bx: number;
  by: number;
}

/** Control points of a link: every arc bows upward. */
export function arc(p: Xy, q: Xy, bend: number): Arc {
  const dx = q.x - p.x;
  const dy = q.y - p.y;
  const len = Math.hypot(dx, dy) || 1;
  let nx = -dy / len;
  let ny = dx / len;
  if (ny > 0) {
    nx = -nx;
    ny = -ny;
  }
  const b = len * bend;
  return { ax: p.x + dx * 0.28 + nx * b, ay: p.y + dy * 0.28 + ny * b, bx: p.x + dx * 0.72 + nx * b, by: p.y + dy * 0.72 + ny * b };
}

export function bez(p: Xy, c: Arc, q: Xy, k: number): { x: number; y: number } {
  const u = 1 - k;
  return {
    x: u * u * u * p.x + 3 * u * u * k * c.ax + 3 * u * k * k * c.bx + k * k * k * q.x,
    y: u * u * u * p.y + 3 * u * u * k * c.ay + 3 * u * k * k * c.by + k * k * k * q.y,
  };
}

/** A rare cell in the far ring (1.5 R to 2.7 R from the lens). */
export function isFarCell(i: number, j: number, dd: number, R: number): boolean {
  return h21(i + 11.1, j + 5.7) >= 0.9965 && dd > R * 1.5 && dd < R * 2.7;
}
