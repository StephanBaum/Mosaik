/**
 * Motion state of the living mosaic: cursor light, press ripple, load sweep and the foil
 * clock. Pure maths plus the pointer wiring; no GL. The easing is the app's (`approach` from
 * the copied holo/motion.ts). Positions are in tile pitches from the mosaic's top-left, y down.
 */
import { approach } from "../fx/motion.ts";
import type { MosaicParams } from "./params.ts";

export interface MosaicMotion {
  clock: number;
  /** Seconds since the load sweep started. */
  intro: number;
  cursorX: number;
  cursorY: number;
  targetX: number;
  targetY: number;
  presence: number;
  presenceT: number;
  inside: boolean;
  /** Clock time (ms) of the last pointer input. */
  lastInput: number;
  /** Press ripple origin (pitches, y up) and age in seconds. */
  ripX: number;
  ripY: number;
  ripAge: number;
}

/** Rendering stops after this long without a pointer (design concept §5). */
export const IDLE_MS = 20_000;
/** Interaction frame rate holds this long after the last pointer move. */
const HOT_MS = 900;
const SETTLE = 0.002;
const IDLE_AGE = 99;

export function newMotion(now: number): MosaicMotion {
  return {
    clock: 2,
    intro: 0,
    cursorX: 0,
    cursorY: 0,
    targetX: 0,
    targetY: 0,
    presence: 0,
    presenceT: 0,
    inside: false,
    lastInput: now,
    ripX: 0,
    ripY: 0,
    ripAge: IDLE_AGE,
  };
}

/** Seconds until the last tile of the sweep has landed. */
export function introEnd(p: MosaicParams): number {
  return 0.5 + p.intro.duration;
}

function settling(m: MosaicMotion): boolean {
  return (
    Math.abs(m.presence - m.presenceT) > SETTLE ||
    Math.abs(m.cursorX - m.targetX) > SETTLE ||
    Math.abs(m.cursorY - m.targetY) > SETTLE
  );
}

/** Whether the mosaic needs interaction-rate frames (60 fps) rather than the idle sheen (30 fps). */
export function hot(m: MosaicMotion, p: MosaicParams, now: number): boolean {
  return (
    m.intro < introEnd(p) ||
    m.ripAge < p.ripple.duration ||
    settling(m) ||
    (m.inside && now - m.lastInput < HOT_MS)
  );
}

/** Advances by `dt` seconds; returns whether the mosaic is still animating. */
export function stepMotion(m: MosaicMotion, p: MosaicParams, frame: { dt: number; now: number }): boolean {
  const { dt, now } = frame;
  const step = (rate: number) => ({ rate, dt, reduced: false });
  m.intro += dt;
  m.ripAge = Math.min(m.ripAge + dt, IDLE_AGE);
  m.presence = approach(m.presence, m.presenceT, step(p.cursor.presenceRate));
  const rate = m.inside ? p.cursor.followRate : p.cursor.settleRate;
  m.cursorX = approach(m.cursorX, m.targetX, step(rate));
  m.cursorY = approach(m.cursorY, m.targetY, step(rate));
  const active = hot(m, p, now) || now - m.lastInput < IDLE_MS;
  if (active) {
    m.clock += dt * p.foil.timeScale;
  }
  return active;
}

export interface PointerHooks {
  /** The mosaic box in CSS px and its pitch. */
  geometry: () => { rect: DOMRect; pitch: number; inset: number };
  poke: () => void;
  now: () => number;
}

function isControl(target: EventTarget | null): boolean {
  return target instanceof Element && target.closest("a, button, input, select, textarea") !== null;
}

/** Wires the hero's pointer to the motion state; returns the function that removes it. */
export function attachPointer(zone: HTMLElement, m: MosaicMotion, hooks: PointerHooks): () => void {
  const local = (event: PointerEvent): { x: number; y: number; within: boolean } => {
    const { rect, pitch, inset } = hooks.geometry();
    const x = (event.clientX - rect.left + inset) / pitch;
    const y = (event.clientY - rect.top + inset) / pitch;
    const within =
      event.clientX >= rect.left &&
      event.clientX <= rect.right &&
      event.clientY >= rect.top &&
      event.clientY <= rect.bottom;
    return { x, y, within };
  };
  const track = (event: PointerEvent): void => {
    const point = local(event);
    if (m.presence < 0.05) {
      m.cursorX = point.x;
      m.cursorY = point.y;
    }
    m.targetX = point.x;
    m.targetY = point.y;
    m.inside = true;
    m.presenceT = 1;
    m.lastInput = hooks.now();
    hooks.poke();
  };
  const leave = (): void => {
    m.inside = false;
    m.presenceT = 0;
    m.lastInput = hooks.now();
    hooks.poke();
  };
  const down = (event: PointerEvent): void => {
    const point = local(event);
    track(event);
    if (point.within && event.button === 0 && !isControl(event.target)) {
      m.ripX = point.x;
      m.ripY = -point.y;
      m.ripAge = 0;
    }
  };
  const up = (event: PointerEvent): void => {
    if (event.pointerType !== "mouse") {
      leave();
    }
  };
  const listeners: [keyof HTMLElementEventMap, (event: PointerEvent) => void][] = [
    ["pointermove", track],
    ["pointerenter", track],
    ["pointerleave", leave],
    ["pointerdown", down],
    ["pointerup", up],
    ["pointercancel", leave],
  ];
  for (const [type, handler] of listeners) {
    zone.addEventListener(type, handler as EventListener, { passive: true });
  }
  return () => {
    for (const [type, handler] of listeners) {
      zone.removeEventListener(type, handler as EventListener);
    }
  };
}
