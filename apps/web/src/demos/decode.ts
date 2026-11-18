/**
 * The hero headline resolves out of hex, line by line, on a fresh visit at the top of a wide
 * page. While it runs each line keeps its box (so nothing below moves), the heading carries its
 * full text as an accessible name, and every trace is removed when it finishes. Narrow
 * viewports, reduced motion, the motion toggle and a restored scroll position skip it.
 */
import { timeline, type Timeline } from "../anim/timeline.ts";
import { MOTION_EVENT, motionAllowed, safeStorage } from "../runtime/motion.ts";
import { decodeTo, resolved } from "./text.ts";

/** Below this viewport width the headline just shows. */
export const MIN_WIDTH = 700;

/** Resolves when the headline has settled (at once when the decode is skipped). */
export function bootDecode(doc: Document): Promise<void> {
  const win = doc.defaultView;
  const h1 = doc.querySelector<HTMLElement>("h1[data-decode]");
  if (win === null || h1 === null || win.innerWidth < MIN_WIDTH || win.scrollY !== 0 || !motionAllowed(win, safeStorage(win))) {
    return Promise.resolve();
  }
  const lines = [...h1.querySelectorAll<HTMLElement>(".line")];
  const texts = lines.map((line) => line.textContent);
  const full = texts.join(" ");
  for (const line of lines) {
    line.style.setProperty("min-height", `${String(line.getBoundingClientRect().height)}px`);
  }
  // The heading shrinks to its widest line, and hex is wider or narrower than the words: pin its
  // width so the centred lines do not slide sideways (layout shift) while the letters resolve.
  h1.style.setProperty("width", `${String(h1.getBoundingClientRect().width)}px`);
  h1.style.setProperty("max-width", "none");
  h1.classList.add("is-decoding");
  h1.setAttribute("aria-label", full);

  let finished: () => void = () => undefined;
  const done = new Promise<void>((resolve) => {
    finished = resolve;
  });
  const settle = (): void => {
    for (const line of lines) {
      line.style.removeProperty("min-height");
    }
    h1.style.removeProperty("width");
    h1.style.removeProperty("max-width");
    h1.classList.remove("is-decoding");
    h1.removeAttribute("aria-label");
    finished();
  };
  const tl: Timeline = timeline({ onComplete: settle });
  tl.call(() => {
    lines.forEach((line, i) => {
      line.textContent = resolved(texts[i] ?? "", 0);
    });
  }, 0);
  lines.forEach((line, i) => {
    decodeTo(tl, line, texts[i] ?? "", { duration: 0.5, at: 0.1 + i * 0.12 });
  });
  const stop = (event: Event): void => {
    if ((event as CustomEvent<{ off: boolean }>).detail.off) {
      tl.seek(Number.POSITIVE_INFINITY);
      doc.removeEventListener(MOTION_EVENT, stop);
    }
  };
  doc.addEventListener(MOTION_EVENT, stop);
  tl.play();
  return done;
}
