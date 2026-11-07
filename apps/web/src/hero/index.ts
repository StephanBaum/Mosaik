/**
 * The hero island: decryption you can see. Starts after first paint and paints the art panel
 * on its own WebGL2 context. The cursor (or a finger) is the key; scrolling out re-encrypts.
 * Without WebGL2 (or after a lost context) the CSS still stays; under reduced motion or
 * Save-Data the shader paints one designed still. Rendering runs only while the hero is on
 * screen and the tab is visible.
 */
import { ArtRenderer } from "./art.ts";
import { ArtLoop, type LoopStats } from "./loop.ts";
import { ArtMotion } from "./motion.ts";
import { defaultParams, type ArtParams } from "./params.ts";
import { scrollOut } from "./timeline.ts";

export interface HeroController {
  readonly params: ArtParams;
  /** Repaints after a parameter change. */
  refresh(): void;
  /** Replays the load decode. */
  replay(): void;
  stats(): LoopStats | null;
}

const LIVE = "art--live";

function prefersStill(): boolean {
  const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData === true;
  return saveData || matchMedia("(prefers-reduced-motion: reduce)").matches;
}

interface Parts {
  hero: HTMLElement;
  box: HTMLElement;
  canvas: HTMLCanvasElement;
}

function wirePointer({ hero, canvas }: Parts, motion: ArtMotion, loop: ArtLoop): void {
  const move = (event: PointerEvent): void => {
    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;
    const inside = x >= 0 && y >= 0 && x <= rect.width && y <= rect.height;
    motion.point(x, y, inside, performance.now());
    loop.kick();
  };
  const leave = (event: PointerEvent): void => {
    const rect = canvas.getBoundingClientRect();
    motion.point(event.clientX - rect.left, event.clientY - rect.top, false, performance.now());
    loop.kick();
  };
  hero.addEventListener("pointermove", move, { passive: true });
  hero.addEventListener("pointerdown", move, { passive: true });
  hero.addEventListener("pointerleave", leave, { passive: true });
  hero.addEventListener("pointercancel", leave, { passive: true });
  // A finger lifts: the key fades back into the breathing reveal.
  hero.addEventListener("pointerup", (event) => {
    if (event.pointerType !== "mouse") {
      leave(event);
    }
  });
}

function wireScroll({ hero }: Parts, motion: ArtMotion, loop: ArtLoop): void {
  const update = (): void => {
    const rect = hero.getBoundingClientRect();
    motion.setScroll(scrollOut(rect.top, rect.height), performance.now());
    loop.kick();
  };
  addEventListener("scroll", update, { passive: true });
  update();
}

function wire(parts: Parts, loop: ArtLoop, motion: ArtMotion): void {
  const { hero, box, canvas } = parts;
  new ResizeObserver(() => {
    loop.relayout();
  }).observe(box);
  new IntersectionObserver((entries) => {
    for (const entry of entries) {
      loop.setVisible(entry.isIntersecting);
    }
  }).observe(hero);
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    loop.repalette();
  });
  canvas.addEventListener("webglcontextlost", (event) => {
    event.preventDefault();
    loop.setVisible(false);
    box.classList.remove(LIVE);
  });
  if (!loop.still) {
    wirePointer(parts, motion, loop);
    wireScroll(parts, motion, loop);
  }
}

export async function bootHero(root: Document): Promise<HeroController | null> {
  const hero = root.querySelector<HTMLElement>(".hero");
  const box = hero?.querySelector<HTMLElement>(".art");
  const canvas = box?.querySelector<HTMLCanvasElement>(".art__canvas");
  if (!hero || !box || !canvas) {
    return null;
  }
  const params = defaultParams();
  const renderer = await ArtRenderer.create(canvas);
  if (renderer === null) {
    return { params, refresh: () => undefined, replay: () => undefined, stats: () => null };
  }
  const motion = new ArtMotion(params);
  const loop = new ArtLoop(box, renderer, motion, {
    still: prefersStill(),
    onPaint: () => {
      box.classList.add(LIVE);
    },
  });
  wire({ hero, box, canvas }, loop, motion);
  loop.relayout();
  return {
    params,
    refresh: () => {
      loop.kick(true);
    },
    replay: () => {
      motion.replay(performance.now());
      loop.kick(true);
    },
    stats: () => loop.stats(),
  };
}
