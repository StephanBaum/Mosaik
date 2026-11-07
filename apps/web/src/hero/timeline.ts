/**
 * Pure timing of the hero art: the load decrypt sequence, the key's inertia and the scroll
 * re-encryption. No DOM, so the tests pin the choreography.
 */
import type { ArtParams } from "./params.ts";

/** Block sizes (CSS px) the decode steps through before the image goes clear. */
export const DECODE_STEPS = [64, 32, 16, 8] as const;

/** Share of the decode spent on the block steps; the rest dissolves 8 px blocks into the clear image. */
const STEP_SHARE = 0.8;

export interface IntroState {
  /** Global block size override (CSS px), or 0 for the settled size. */
  block: number;
  /** 1 = the whole image is clear; 0 = only the key reveals. */
  reveal: number;
  /** Position of the pixel-sort sweep, 0 (top) to 1 (bottom); negative when no sweep runs. */
  sweep: number;
  /** Whether the sequence still runs. */
  running: boolean;
}

const smooth = (x: number): number => {
  const t = Math.min(Math.max(x, 0), 1);
  return t * t * (3 - 2 * t);
};

export function introEnd(decode: ArtParams["decode"]): number {
  return decode.duration + decode.hold + decode.collapse;
}

export function introState(t: number, decode: ArtParams["decode"]): IntroState {
  const steps = decode.duration * STEP_SHARE;
  if (t < steps) {
    const index = Math.min(Math.floor((t / steps) * DECODE_STEPS.length), DECODE_STEPS.length - 1);
    return { block: DECODE_STEPS[index] ?? 8, reveal: 0, sweep: t / steps, running: true };
  }
  if (t < decode.duration) {
    return { block: 8, reveal: smooth((t - steps) / (decode.duration - steps)), sweep: -1, running: true };
  }
  const collapseFrom = decode.duration + decode.hold;
  if (t < collapseFrom) {
    return { block: 0, reveal: 1, sweep: -1, running: true };
  }
  const end = collapseFrom + decode.collapse;
  if (t < end) {
    return { block: 0, reveal: 1 - smooth((t - collapseFrom) / decode.collapse), sweep: -1, running: true };
  }
  return { block: 0, reveal: 0, sweep: -1, running: false };
}

/** Exponential follow toward `target` at `rate` per second: frame-rate independent inertia. */
export function follow(current: number, target: number, rate: number, dt: number): number {
  return current + (target - current) * (1 - Math.exp(-rate * dt));
}

/** How far the hero has scrolled out: 0 while its top is at the viewport top, 1 once it has left. */
export function scrollOut(top: number, height: number): number {
  if (height <= 0) {
    return 0;
  }
  return Math.min(Math.max(-top / height, 0), 1);
}

export interface ScrollLook {
  /** Multiplier on the block size. */
  blockScale: number;
  /** Multiplier on the key radius. */
  keyScale: number;
  /** Extra pixel-sort strength and the downward pull of the streaks (share of the view height). */
  sortBoost: number;
  pull: number;
}

/** Scrolling out re-encrypts: blocks grow, the key closes and the streaks pull downward. */
export function scrollLook(progress: number): ScrollLook {
  const p = smooth(progress * 1.4);
  return { blockScale: 1 + 2.2 * p, keyScale: 1 - p, sortBoost: 1.1 * p, pull: 0.6 * p };
}
