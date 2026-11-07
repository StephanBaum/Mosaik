/**
 * Pointer and scroll input of the painted hero. The clean circle follows the pointer with
 * inertia and relaxes in while a mouse or pen is over the hero; touch leaves the painting
 * sorted (a finger hides what it would reveal). Scrolling the hero out raises the sorting.
 */
import { follow, scrollProgress } from "./geometry.ts";

export interface InputState {
  /** Smoothed pointer position in CSS px inside the canvas. */
  x: number;
  y: number;
  /** Smoothed relax amount (0 = sorted, 1 = the clean circle is fully open). */
  relax: number;
  /** Share of the hero scrolled out (0..1). */
  scroll: number;
}

export interface HeroInput {
  readonly state: InputState;
  /** Advances the smoothing by `dt` seconds at `rate` per second; returns whether it still moves. */
  step(dt: number, rate: number): boolean;
}

const SETTLED = 0.002;

/** `hero` takes the events and scrolls; `frame` (the canvas) is the coordinate box. */
export function attachInput(hero: HTMLElement, frame: HTMLElement, onInput: () => void): HeroInput {
  const target = { x: 0, y: 0, relax: 0 };
  const state: InputState = { x: 0, y: 0, relax: 0, scroll: 0 };
  let placed = false;
  const move = (event: PointerEvent): void => {
    if (event.pointerType === "touch") {
      return;
    }
    const rect = frame.getBoundingClientRect();
    target.x = event.clientX - rect.left;
    target.y = event.clientY - rect.top;
    target.relax = 1;
    if (!placed) {
      // The first sighting starts the circle where the pointer is, closed, so it opens in place.
      state.x = target.x;
      state.y = target.y;
      placed = true;
    }
    onInput();
  };
  hero.addEventListener("pointermove", move, { passive: true });
  hero.addEventListener("pointerenter", move, { passive: true });
  hero.addEventListener("pointerleave", () => {
    target.relax = 0;
    onInput();
  });
  const readScroll = (): void => {
    const rect = hero.getBoundingClientRect();
    state.scroll = scrollProgress(rect.top, rect.height);
  };
  window.addEventListener("scroll", readScroll, { passive: true });
  readScroll();
  return {
    state,
    step(dt, rate) {
      state.x = follow(state.x, target.x, rate, dt);
      state.y = follow(state.y, target.y, rate, dt);
      // The circle opens and closes a little faster than it travels.
      state.relax = follow(state.relax, target.relax, rate * 1.4, dt);
      return Math.abs(state.relax - target.relax) > SETTLED || Math.hypot(state.x - target.x, state.y - target.y) > 0.5;
    },
  };
}
