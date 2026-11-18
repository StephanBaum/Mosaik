/**
 * Shared pieces of the interface demos: the bounded story timeline, text effects (a headline
 * resolving out of hex, a typewriter), small DOM helpers and a fixed noise source. Demo words
 * live in the rendered markup, never here, and the noise is a fixed sequence, not randomness:
 * the site never produces anything secret-looking, even as decoration.
 */
import { linear, power1InOut } from "../anim/ease.ts";
import type { Ticker } from "../anim/ticker.ts";
import { timeline, type Position, type Timeline } from "../anim/timeline.ts";

/** A demo plays this many loops at most, then rests on its final frame (WCAG 2.2.2). */
export const MAX_LOOPS = 3;

let clock: Ticker | undefined;

/** Tests drive demos on their own ticker; pass `undefined` to go back to the page's. */
export function useClock(next: Ticker | undefined): void {
  clock = next;
}

/** A story: `MAX_LOOPS` iterations with a pause between, then it stops on its last frame. */
export function story(repeatDelay: number, loops = MAX_LOOPS): Timeline {
  return timeline({ repeat: loops - 1, repeatDelay, ...(clock === undefined ? {} : { ticker: clock }) });
}

const HEX = "0123456789abcdef";
let state = 0x2f6e2b1;

/** The next value of a fixed linear congruential sequence (below `n`). */
export function nextInt(n: number): number {
  state = (Math.imul(state, 1664525) + 1013904223) >>> 0;
  return (state >>> 8) % n;
}

export const hexChar = (): string => HEX[nextInt(16)] ?? "0";

/** `text` with the characters from `done` on replaced by hex noise (spaces stay). */
export function resolved(text: string, v: number): string {
  const done = Math.floor(v * text.length * 1.08);
  return Array.from(text, (c, i) => (i < done || c === " " ? c : hexChar())).join("");
}

/** When and for how long a text effect runs; `at` defaults to the end of the timeline. */
export interface Timing {
  duration: number;
  at?: Position;
}

/** `+=seconds`: that long after the timeline end. */
export const after = (seconds: number): Position => `+=${String(seconds)}` as `+=${number}`;

/** Characters resolve left to right out of hex noise, ending on `text`. */
export function decodeTo(tl: Timeline, el: Element, text: string, { duration, at }: Timing): void {
  const p = { v: 0 };
  tl.to(
    p,
    { v: 1 },
    {
      duration,
      ease: power1InOut,
      onUpdate: () => {
        el.textContent = p.v >= 1 ? text : resolved(text, p.v);
      },
    },
    at,
  );
}

/** Typewriter. */
export function typeTo(tl: Timeline, el: Element, text: string, { duration, at }: Timing): void {
  const p = { n: 0 };
  tl.to(
    p,
    { n: text.length },
    {
      duration,
      ease: linear,
      onUpdate: () => {
        el.textContent = text.slice(0, Math.round(p.n));
      },
    },
    at,
  );
}

/** Holds the story still for `seconds`. */
export function wait(tl: Timeline, seconds: number, at?: Position): void {
  tl.to({ v: 0 }, { v: 1 }, { duration: seconds, ease: linear }, at);
}

/** A required element of a demo; a missing hook makes the demo's builder give up (the markup stays static). */
export function pick(root: ParentNode, selector: string): HTMLElement {
  const el = root.querySelector<HTMLElement>(selector);
  if (el === null) {
    throw new Error(`demo: missing ${selector}`);
  }
  return el;
}

/** The same for an SVG element. */
export function pickSvg(root: ParentNode, selector: string): SVGElement {
  const el = root.querySelector<SVGElement>(selector);
  if (el === null) {
    throw new Error(`demo: missing ${selector}`);
  }
  return el;
}

export const pickAll = (root: ParentNode, selector: string): HTMLElement[] => [...root.querySelectorAll<HTMLElement>(selector)];

/** Centre (or the fraction `fx`, `fy`) of `el` in the coordinates of `page`. */
export function centerIn(el: Element, page: Element, fx = 0.5, fy = 0.5): { x: number; y: number } {
  const a = el.getBoundingClientRect();
  const b = page.getBoundingClientRect();
  return { x: a.left - b.left + a.width * fx, y: a.top - b.top + a.height * fy };
}

/** A call that switches a class on or off: `state` is `[class name, on]`. */
export function flag(tl: Timeline, el: Element, state: readonly [name: string, on: boolean], at?: Position): void {
  tl.call(() => {
    el.classList.toggle(...state);
  }, at);
}
