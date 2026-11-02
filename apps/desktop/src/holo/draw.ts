/**
 * One draw call of the foil shader into the bottom-left of the GL canvas. Shared by the
 * button renderer and (later) static surfaces, so both use the same context and program.
 */
import type { HoloGl, HoloProgram } from "./gl.ts";
import { EFFECT_IDLE, FOCUS, focusWeight, RIPPLE, rippleWeight } from "./effects.ts";
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
  /** Device px of transparent margin on every side around the button (buttons only). */
  margin?: number;
  /** Press ripple and focus wave ages in seconds; omitted for stills. */
  ripple?: { x: number; y: number; age: number };
  focusAge?: number;
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
  const m = p.margin ?? 0;
  gl.uniform4f(loc.uRect, m, m, p.width - 2 * m, p.height - 2 * m);
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
  drawEffects(gl, program, p);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
}

const TAU = 2 * Math.PI;

function drawEffects(gl: HoloGl, { loc }: HoloProgram, p: DrawParams): void {
  const rippleAge = p.ripple?.age ?? EFFECT_IDLE;
  const focusAge = p.focusAge ?? EFFECT_IDLE;
  gl.uniform4f(loc.uRipP, RIPPLE.speed, TAU / RIPPLE.wavelength, RIPPLE.runout, RIPPLE.outline);
  gl.uniform2f(loc.uRipQ, RIPPLE.refraction, RIPPLE.shading);
  gl.uniform4f(loc.uFocP, FOCUS.speed, TAU / FOCUS.wavelength, FOCUS.runout, FOCUS.outline);
  gl.uniform4f(loc.uFocQ, FOCUS.refraction, FOCUS.shading, FOCUS.grow, FOCUS.gap);
  gl.uniform4f(loc.uRipple, p.ripple?.x ?? 0, p.ripple?.y ?? 0, rippleAge, rippleWeight(rippleAge));
  gl.uniform2f(loc.uFocusR, focusAge, focusWeight(focusAge));
}
