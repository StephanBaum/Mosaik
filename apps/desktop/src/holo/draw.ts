/**
 * One draw call of the foil shader into the bottom-left of the GL canvas. Shared by the
 * button renderer and (later) static surfaces, so both use the same context and program.
 */
import type { HoloGl, HoloProgram } from "./gl.ts";
import { BASE_PERIOD_DEV } from "./shader.ts";

export interface DrawParams {
  /** Size in device px. */
  width: number;
  height: number;
  /** Device pixel ratio the stripe period is laid out for. */
  dpr: number;
  mouse: { x: number; y: number };
  time: number;
  hover: number;
  press: number;
  focus: number;
  dark: number;
  seed: number;
  /** 1 paints the pill with its rim (buttons); 0 the bare surface (stills). */
  mask: number;
  /** Stripe frequency factor (1 for buttons). */
  stripeK: number;
  /** Reference unit size and half-aspect for stills; `[0, 0]` for buttons. */
  unit: readonly [number, number];
}

/** Stripe period in device px for a device pixel ratio. */
export function screenPeriod(dpr: number): number {
  return BASE_PERIOD_DEV * (dpr >= 2 ? 1 : 0.75);
}

export function drawFoil(gl: HoloGl, program: HoloProgram, p: DrawParams): void {
  const { loc } = program;
  gl.viewport(0, 0, p.width, p.height);
  gl.clearColor(0, 0, 0, 0);
  gl.clear(gl.COLOR_BUFFER_BIT);
  gl.uniform4f(loc.uRect, 0, 0, p.width, p.height);
  gl.uniform2f(loc.uMouse, p.mouse.x, p.mouse.y);
  gl.uniform1f(loc.uTime, p.time);
  gl.uniform1f(loc.uHover, p.hover);
  gl.uniform1f(loc.uPress, p.press);
  gl.uniform1f(loc.uFocus, p.focus);
  gl.uniform1f(loc.uDark, p.dark);
  gl.uniform1f(loc.uSeed, p.seed);
  gl.uniform1f(loc.uDPR, p.dpr);
  gl.uniform1f(loc.uScrPer, screenPeriod(p.dpr));
  gl.uniform1f(loc.uStripeK, p.stripeK);
  gl.uniform1f(loc.uMask, p.mask);
  gl.uniform2f(loc.uUnit, p.unit[0], p.unit[1]);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
}
