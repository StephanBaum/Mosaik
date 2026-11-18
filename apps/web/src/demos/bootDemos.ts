/**
 * Boots the interface demos. Every `[data-demo]` is rendered in its final frame, so without this
 * chunk (no JS, reduced motion, motion off) the page already shows the finished state. With
 * motion allowed a demo is built once it nears the viewport, starts from its first frame, plays
 * while it is on screen and pauses when it is not; its timeline stops after `MAX_LOOPS` loops on
 * the last frame. Turning motion off jumps every demo to its final frame.
 */
import type { Timeline } from "../anim/timeline.ts";
import { MOTION_EVENT, motionAllowed, safeStorage } from "../runtime/motion.ts";
import { build as agent } from "./agent.ts";
import { build as cipher } from "./cipher.ts";
import { build as cli } from "./cli.ts";
import { build as fill } from "./fill.ts";
import { build as importer } from "./importer.ts";
import { build as nextSteps } from "./nextSteps.ts";
import { build as phone } from "./phone.ts";
import { build as sync } from "./sync.ts";

export const DEMOS: Readonly<Record<string, (root: HTMLElement) => Timeline | null>> = {
  fill,
  sync,
  agent,
  import: importer,
  phone,
  cli,
  cipher,
  nextSteps,
};

function safely(build: (root: HTMLElement) => Timeline | null, root: HTMLElement): Timeline | null {
  try {
    return build(root);
  } catch {
    // A demo that cannot build stays on its static markup.
    return null;
  }
}

/** Rests a timeline on its last frame, from any point of any loop. */
function rest(tl: Timeline): void {
  tl.seek(Number.POSITIVE_INFINITY);
  tl.kill();
}

export function bootDemos(doc: Document, IO: typeof IntersectionObserver = IntersectionObserver): void {
  const win = doc.defaultView;
  // Without motion the markup already shows every final frame: there is nothing to build.
  if (win === null || !motionAllowed(win, safeStorage(win))) {
    return;
  }
  const roots = [...doc.querySelectorAll<HTMLElement>("[data-demo]")].filter((el) => DEMOS[el.dataset["demo"] ?? ""] !== undefined);
  if (roots.length === 0) {
    return;
  }
  const live = new Map<Element, Timeline>();
  const player = new IO(
    (entries) => {
      for (const entry of entries) {
        const tl = live.get(entry.target);
        if (entry.isIntersecting) {
          tl?.play();
        } else {
          tl?.pause();
        }
      }
    },
    { rootMargin: "80px" },
  );
  // A demo is built, and reset to its first frame, only once it nears the viewport: building
  // measures layout, and measuring a section the browser still skips (content-visibility) would
  // force the whole rest of the page to render at once.
  const builder = new IO(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) {
          continue;
        }
        const el = entry.target as HTMLElement;
        builder.unobserve(el);
        const build = DEMOS[el.dataset["demo"] ?? ""];
        const tl = build === undefined ? null : safely(build, el);
        if (tl !== null) {
          tl.progress(0);
          live.set(el, tl);
          player.observe(el);
        }
      }
    },
    { rootMargin: "100% 0px" },
  );
  for (const el of roots) {
    builder.observe(el);
  }
  doc.addEventListener(MOTION_EVENT, (event) => {
    if ((event as CustomEvent<{ off: boolean }>).detail.off) {
      builder.disconnect();
      player.disconnect();
      for (const tl of live.values()) {
        rest(tl);
      }
      live.clear();
    }
  });
}
