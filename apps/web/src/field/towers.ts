/** The JS mirror of the field shader's tower height, so the data layer can pin labels to towers. */
import type { Click } from "./bootField.ts";

/** Everything a tower height depends on besides its cell: cell size, time, lens and click wave (device pixels). */
export interface TowerParams {
  cs: number;
  t: number;
  lx: number;
  ly: number;
  R: number;
  click: Click | null;
}

export const fr = (x: number): number => x - Math.floor(x);

export function ss(a: number, b: number, x: number): number {
  const k = Math.min(1, Math.max(0, (x - a) / (b - a)));
  return k * k * (3 - 2 * k);
}

/** Hash of a cell to [0, 1), the same family as the shader's `h21`. */
export function h21(x: number, y: number): number {
  let a = fr(x * 0.1031);
  let b = fr(y * 0.1031);
  let c = fr(x * 0.1031);
  const d = a * (b + 33.33) + b * (c + 33.33) + c * (a + 33.33);
  a += d;
  b += d;
  c += d;
  return fr((a + b) * c);
}

const mix = (a: number, b: number, k: number): number => a + (b - a) * k;

/** Value noise on the cell grid. */
export function vn(x: number, y: number): number {
  const ix = Math.floor(x);
  const iy = Math.floor(y);
  const fx = x - ix;
  const fy = y - iy;
  const ux = fx * fx * (3 - 2 * fx);
  const uy = fy * fy * (3 - 2 * fy);
  return mix(mix(h21(ix, iy), h21(ix + 1, iy), ux), mix(h21(ix, iy + 1), h21(ix + 1, iy + 1), ux), uy);
}

/** Height 0..1 of the tower in cell (i, j): soft floors from drifting noise that rise toward the lens. */
export function towerH(i: number, j: number, p: TowerParams): number {
  const { cs, t, lx, ly, R, click } = p;
  const cx = (i + 0.5) * cs;
  const cy = (j + 0.5) * cs;
  const d = Math.hypot(cx - lx, cy - ly);
  const r = (0.72 * Math.exp(-((d / (R * 0.5)) ** 2)) + 0.28 * Math.exp(-d / (R * 1.1))) * (1 - ss(R * 2.2, R * 2.6, d));
  let wv = 0;
  if (click !== null && click.age >= 0 && click.age < 1.6) {
    const cd = Math.hypot(cx - click.x, cy - click.y);
    wv = Math.exp(-(((cd - click.age * R * 1.5) / (cs * 3.5)) ** 2)) * (1 - click.age / 1.6) * (0.45 + 0.55 * h21(i + 2.2, j + 2.2));
  }
  if (r < 0.002 && wv < 0.002) {
    return 0;
  }
  const n = vn(i * 0.17 + t * 0.09, j * 0.17 - t * 0.06);
  const n2 = vn(i * 0.045 - t * 0.03 + 3, j * 0.045 + t * 0.025 + 3);
  let x = (0.55 * n + 0.45 * h21(i + 13.7, j + 13.7)) * (0.55 + 0.65 * n2);
  x = x * x * 7;
  const floors = Math.min(1, Math.max(0, (Math.floor(x) + ss(0.7, 1, fr(x))) / 7));
  return Math.min(1, floors * r + 0.6 * wv);
}
