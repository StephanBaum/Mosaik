/**
 * Micro-interactions: nothing hard-flips. Line buttons fill their rim from the side the pointer
 * entered with one travelling glint, raised surfaces carry a soft glow that follows the pointer,
 * and nav underlines grow from the side the pointer came from. Everything is a CSS variable
 * eased by the in-house timeline. Touch pointers are ignored; without this chunk, or with motion
 * off, the stylesheet's own hover states still work.
 */
import { power2InOut, power2Out, power3Out } from "../anim/ease.ts";
import { timeline, type Timeline } from "../anim/timeline.ts";
import { MOTION_EVENT } from "../runtime/motion.ts";

export const RING = ".button--line";
export const GLOW = ".button--line, .button--solid, .tile, .plan, .card, .platform__link, .step-card, .see__card, .ds, .di, .db, .da, .terminal";
export const UNDERLINE = ".site-nav__links a, .lang a";

/** Conic angle in degrees (0 = top, clockwise) from the element's centre to the pointer. */
export function angleTo(rect: Pick<DOMRect, "left" | "top" | "width" | "height">, x: number, y: number): number {
  const a = (Math.atan2(y - (rect.top + rect.height / 2), x - (rect.left + rect.width / 2)) * 180) / Math.PI + 90;
  return (a + 360) % 360;
}

export function bootMicro(doc: Document): void {
  const running = new Map<Element, Timeline>();
  const cleanups: (() => void)[] = [];

  /** Starts a run on `el`, replacing the one in flight (GSAP's `overwrite: "auto"`). */
  const run = (el: HTMLElement, build: (tl: Timeline) => void): void => {
    running.get(el)?.kill();
    const tl = timeline();
    build(tl);
    running.set(el, tl);
    tl.play();
  };
  const on = <E extends keyof HTMLElementEventMap>(el: HTMLElement, type: E, fn: (e: HTMLElementEventMap[E]) => void): void => {
    el.addEventListener(type, fn);
    cleanups.push(() => {
      el.removeEventListener(type, fn);
    });
  };

  for (const el of doc.querySelectorAll<HTMLElement>(GLOW)) {
    const ring = el.matches(RING);
    const move = (e: PointerEvent): void => {
      const r = el.getBoundingClientRect();
      el.style.setProperty("--bl-x", `${(e.clientX - r.left).toFixed(1)}px`);
      el.style.setProperty("--bl-y", `${(e.clientY - r.top).toFixed(1)}px`);
    };
    on(el, "pointerenter", (e: PointerEvent) => {
      if (e.pointerType === "touch") {
        return;
      }
      move(e);
      if (!ring) {
        run(el, (tl) => tl.to(el, { "--bl-glow": 1 }, { duration: 0.5, ease: power3Out }));
        return;
      }
      const a = angleTo(el.getBoundingClientRect(), e.clientX, e.clientY);
      el.style.setProperty("--bl-start", a.toFixed(1));
      run(el, (tl) =>
        tl
          .to(el, { "--bl-fill": 1, "--bl-glow": 1 }, { duration: 0.55, ease: power3Out })
          .set(el, { "--bl-glint": a / 360, "--bl-glint-o": 0.85 }, 0)
          .to(el, { "--bl-glint": a / 360 + 1, "--bl-glint-o": 0 }, { duration: 1.1, ease: power2Out }, 0.12),
      );
    });
    on(el, "pointermove", (e: PointerEvent) => {
      if (e.pointerType !== "touch") {
        move(e);
      }
    });
    on(el, "pointerleave", () => {
      run(el, (tl) => tl.to(el, { "--bl-fill": 0, "--bl-glow": 0 }, { duration: 0.5, ease: power2InOut }));
    });
  }

  for (const el of doc.querySelectorAll<HTMLElement>(UNDERLINE)) {
    on(el, "pointerenter", (e: PointerEvent) => {
      const r = el.getBoundingClientRect();
      el.style.setProperty("--ul-pos", e.clientX < r.left + r.width / 2 ? "0%" : "100%");
    });
  }

  const stop = (event: Event): void => {
    if ((event as CustomEvent<{ off: boolean }>).detail.off) {
      for (const tl of running.values()) {
        tl.kill();
      }
      running.clear();
      for (const undo of cleanups) {
        undo();
      }
      doc.removeEventListener(MOTION_EVENT, stop);
    }
  };
  doc.addEventListener(MOTION_EVENT, stop);
}
