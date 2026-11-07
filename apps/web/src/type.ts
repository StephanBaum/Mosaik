/**
 * Wires the scramble-decode type: the hero headline decodes on load (pixel blocks → glyphs →
 * letters, per character), the nav links re-decode on hover and focus. Under reduced motion
 * the headline is plain from the first frame and nothing scrambles.
 *
 * The headline paints as pixel blocks at once; its decode starts when `start` is called (the
 * entry calls it once the hero art has its GL context, whose creation can stall the main
 * thread), or after a safety delay.
 */
import { decode, prime, splitChars, type ScrambleTiming } from "./scramble.ts";

const HEADLINE: ScrambleTiming = { delay: 120, stagger: 26, cycle: 420, tick: 55, blocks: true };
const SECOND_LINE_DELAY = 240;
const HOVER: ScrambleTiming = { delay: 0, stagger: 22, cycle: 240, tick: 45, blocks: false };

const READY = "is-split";
/** The decode starts by itself after this long if nobody calls `start`. */
const START_FALLBACK_MS = 1600;

const timingFor = (index: number): ScrambleTiming => ({ ...HEADLINE, delay: HEADLINE.delay + index * SECOND_LINE_DELAY });

function headline(root: Document): () => void {
  const title = root.querySelector<HTMLElement>(".hero__title");
  if (!title || title.classList.contains(READY)) {
    return () => undefined;
  }
  // Screen readers get one clean copy; the split characters are presentation only.
  const label = root.createElement("span");
  label.className = "sr-only";
  label.textContent = (title.textContent || "").replace(/\s+/g, " ").trim();
  const lines = Array.from(title.querySelectorAll<HTMLElement>(".line"));
  for (const line of lines) {
    line.setAttribute("aria-hidden", "true");
  }
  title.append(label);
  const groups = lines.map((line) => splitChars(line));
  groups.forEach((spans, index) => {
    prime(spans, timingFor(index));
  });
  title.classList.add(READY);
  let started = false;
  const start = (): void => {
    if (started) {
      return;
    }
    started = true;
    groups.forEach((spans, index) => {
      void decode(spans, timingFor(index));
    });
  };
  window.setTimeout(start, START_FALLBACK_MS);
  return start;
}

function hover(link: HTMLElement): void {
  let spans: HTMLSpanElement[] | null = null;
  let running = false;
  const run = (): void => {
    if (running) {
      return;
    }
    running = true;
    spans ??= splitChars(link);
    void decode(spans, HOVER).then(() => {
      running = false;
    });
  };
  link.addEventListener("pointerenter", run);
  link.addEventListener("focus", run);
}

/** Splits and primes the headline; returns the function that starts its decode. */
export function wireType(root: Document): () => void {
  const title = root.querySelector<HTMLElement>(".hero__title");
  if (matchMedia("(prefers-reduced-motion: reduce)").matches) {
    title?.classList.add(READY);
    return () => undefined;
  }
  for (const link of root.querySelectorAll<HTMLElement>(".site-nav__links a, .site-header__cta")) {
    hover(link);
  }
  return headline(root);
}
