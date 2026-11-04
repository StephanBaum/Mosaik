/** The stage geometry of one reveal run: size, origin, farthest corner and snapshot scale. */
import { MAX_DPR } from "./timing.ts";

export type Point = readonly [x: number, y: number];

export interface Scene {
  /** Stage size in CSS px. */
  readonly w: number;
  readonly h: number;
  /** Where the lens grows from or closes into, in stage px. */
  readonly o: Point;
  /** Distance from the origin to the farthest stage corner. */
  readonly maxD: number;
  /** Snapshot scale: device pixels per CSS px, capped by the pixel budget. */
  readonly S: number;
}

export function sceneFor(
  size: { readonly width: number; readonly height: number },
  o: Point,
  dpr: number,
  budget: number,
): Scene {
  const w = size.width;
  const h = size.height;
  const maxD = Math.max(
    Math.hypot(o[0], o[1]),
    Math.hypot(w - o[0], o[1]),
    Math.hypot(o[0], h - o[1]),
    Math.hypot(w - o[0], h - o[1]),
  );
  const area = Math.max(w * h, 1);
  return { w, h, o, maxD, S: Math.min(dpr, MAX_DPR, Math.sqrt(budget / area)) };
}
