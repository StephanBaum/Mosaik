/** The key lens: the point the towers rise around. It follows the pointer and orbits when idle. */
export const IDLE_MS = 2600;

export interface Lens {
  x: number;
  y: number;
  tx: number;
  ty: number;
  lastPointer: number;
  idleT: number;
  wasIdle: boolean;
  a0: number;
  s0: number;
}

export function createLens(w: number, h: number): Lens {
  return { x: w * 0.72, y: h * 0.4, tx: w * 0.72, ty: h * 0.4, lastPointer: -1e9, idleT: 0, wasIdle: false, a0: 0, s0: 1 };
}

export function lensRadius(w: number, h: number): number {
  return Math.max(180, Math.min(360, Math.min(w, h) * 0.38));
}

/**
 * One step of `dt` seconds. After the pointer rests, the lens starts a slow orbit around the
 * hero copy, never through it: it begins where the pointer was left (same angle and distance)
 * and widens to the orbit over a few seconds.
 */
export function stepLens(lens: Lens, dt: number, now: number, view: { w: number; h: number }): void {
  const { w, h } = view;
  const idle = now - lens.lastPointer > IDLE_MS;
  if (idle) {
    const rx = w * 0.36;
    const ry = h * 0.34;
    if (!lens.wasIdle) {
      lens.wasIdle = true;
      lens.idleT = 0;
      const nx = (lens.x - w * 0.5) / rx;
      const ny = (lens.y - h * 0.5) / ry;
      lens.a0 = Math.atan2(ny, nx);
      lens.s0 = Math.min(1.5, Math.hypot(nx, ny));
    }
    lens.idleT += dt;
    const a = lens.a0 + lens.idleT * 0.17;
    const e = Math.min(1, lens.idleT / 6);
    const sc = lens.s0 + (1 - lens.s0) * e * e * (3 - 2 * e);
    lens.tx = w * 0.5 + Math.cos(a) * rx * sc * (1 + 0.11 * Math.sin(lens.idleT * 0.5));
    lens.ty = h * 0.5 + Math.sin(a) * ry * sc * (1 + 0.12 * Math.sin(lens.idleT * 0.4));
  } else {
    lens.wasIdle = false;
  }
  const k = Math.min(1, dt * (idle ? 2.2 : 7));
  lens.x += (lens.tx - lens.x) * k;
  lens.y += (lens.ty - lens.y) * k;
}
