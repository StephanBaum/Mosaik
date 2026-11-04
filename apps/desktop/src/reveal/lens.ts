/**
 * The lens outline. One definition written twice, as in the study (reveal-studies.html
 * section 3): GLSL for the glass band and JS for the clip-path polygon. Only sums of sines,
 * so both compute the same shape.
 */
import type { Point } from "./scene.ts";

export interface Lens {
  /** Base radius in px. */
  R: number;
  /** Run time in seconds; animates the outline. */
  t: number;
  /** Irregularity as a share of the radius. */
  shape: number;
  /** Rim wobble in px. */
  wob: number;
  /** Glass thickness. */
  H: number;
  /** Presence of the glass band, 0 to 1. */
  pres: number;
}

export const LENS_N = 120;

export const LENS_GLSL = `
float edgeR(float a){
  float n1=0.6*sin(a+2.1+1.3*uT)+0.4*sin(2.0*a-0.4-1.1*uT);
  float wob=uWob*(0.65*n1+0.35*sin(3.0*a+5.0*uT));
  float n2=0.7*sin(a+3.1+0.15*uT)+0.3*sin(2.0*a+1.7-0.1*uT);
  float irr=uShape*uR*(0.55*n2+0.30*sin(2.0*a+1.3+0.4*uT)+0.15*sin(3.0*a-0.7));
  return max(uR+wob+irr,0.0);
}
float lensD(vec2 p){ vec2 q=p-uO; float r=length(q); return r-edgeR(r<1e-4?0.0:atan(q.y,q.x)); }`;

export function edgeR(a: number, lens: Lens): number {
  const t = lens.t;
  const S = Math.sin;
  const n1 = 0.6 * S(a + 2.1 + 1.3 * t) + 0.4 * S(2 * a - 0.4 - 1.1 * t);
  const wob = lens.wob * (0.65 * n1 + 0.35 * S(3 * a + 5 * t));
  const n2 = 0.7 * S(a + 3.1 + 0.15 * t) + 0.3 * S(2 * a + 1.7 - 0.1 * t);
  const irr = lens.shape * lens.R * (0.55 * n2 + 0.3 * S(2 * a + 1.3 + 0.4 * t) + 0.15 * S(3 * a - 0.7));
  return Math.max(lens.R + wob + irr, 0);
}

/** The outline as a CSS `polygon()`; clips the layer and its hit testing. */
export function lensClip(lens: Lens, o: Point): string {
  const points: string[] = [];
  for (let i = 0; i < LENS_N; i++) {
    const a = -Math.PI + (i / LENS_N) * 2 * Math.PI;
    const e = edgeR(a, lens);
    points.push(`${(o[0] + e * Math.cos(a)).toFixed(2)}px ${(o[1] + e * Math.sin(a)).toFixed(2)}px`);
  }
  return `polygon(${points.join(",")})`;
}

/** The radii the outline can never leave: wholly inside `inner`, wholly outside `outer`. */
export function lensBounds(lens: Lens): { inner: number; outer: number } {
  return {
    inner: lens.R * (1 - lens.shape) - lens.wob,
    outer: lens.R * (1 + lens.shape) + lens.wob,
  };
}
