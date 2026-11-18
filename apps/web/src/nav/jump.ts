/**
 * An in-page jump you can watch (`a[data-jump]` with `href="#id"`): the pressed button lifts out
 * of the page, the copy around it drifts up and dims, the page glides down after a beat and the
 * button fades back last. Nothing blocks: a wheel, touch or key ends the glide at once.
 * Keyboard activation, reduced motion and the motion toggle use the native anchor jump.
 */
import { power2In, power3InOut } from "../anim/ease.ts";
import { timeline, type Timeline } from "../anim/timeline.ts";
import { tween } from "../anim/tween.ts";
import { motionAllowed, safeStorage } from "../runtime/motion.ts";
import { isPlainPointerClick } from "./intercept.ts";

const LIFT_PROPS = ["position", "left", "top", "width", "height", "margin", "z-index", "pointer-events", "transform", "opacity"];

const clearStyles = (el: HTMLElement): void => {
  for (const p of LIFT_PROPS) {
    el.style.removeProperty(p);
  }
};

/** Lifts `button` out of the flow, leaving a placeholder; `release` puts it back at once. */
function lift(doc: Document, button: HTMLElement): { release(): void } {
  const r = button.getBoundingClientRect();
  const ph = doc.createElement("span");
  ph.style.setProperty("display", "inline-block");
  ph.style.setProperty("width", `${String(r.width)}px`);
  ph.style.setProperty("height", `${String(r.height)}px`);
  ph.style.setProperty("vertical-align", "middle");
  button.before(ph);
  const set = (k: string, v: string): void => {
    button.style.setProperty(k, v);
  };
  set("position", "fixed");
  set("left", `${String(r.left)}px`);
  set("top", `${String(r.top)}px`);
  set("width", `${String(r.width)}px`);
  set("height", `${String(r.height)}px`);
  set("margin", "0");
  set("z-index", "var(--layer-popover)");
  set("pointer-events", "none");
  doc.body.append(button);
  return {
    release() {
      if (ph.isConnected) {
        ph.replaceWith(button);
      }
      clearStyles(button);
    },
  };
}

/** The pressed button fades out while the glide runs, then fades back in place. */
function buttonTimeline(button: HTMLElement, release: () => void): Timeline {
  return timeline()
    .to(button, { scale: 1.08, opacity: 0 }, { duration: 0.42, ease: power2In }, 0.5)
    .call(release)
    .set(button, { opacity: 0, scale: 1 })
    .to(button, { opacity: 1 }, { duration: 0.4 }, "+=0.2")
    .call(() => {
      clearStyles(button);
    });
}

/** The section copy around the button drifts up and dims. */
function copyTimeline(button: HTMLElement | null): { rest: HTMLElement[]; tl: Timeline | null } {
  const section = button?.closest("section, header, .hero") ?? null;
  const rest = section === null ? [] : [...section.querySelectorAll<HTMLElement>("[data-intro], h1, p")].filter((el) => !el.contains(button));
  if (rest.length === 0) {
    return { rest, tl: null };
  }
  return { rest, tl: timeline().to(rest, { y: -14, opacity: 0.25 }, { duration: 0.5, ease: power2In, stagger: 0.03 }) };
}

function scrollTarget(win: Window, target: HTMLElement): number {
  const margin = parseFloat(win.getComputedStyle(target).scrollMarginTop) || 0;
  const max = win.document.documentElement.scrollHeight - win.innerHeight;
  return Math.max(0, Math.min(target.getBoundingClientRect().top + win.scrollY - margin, max));
}

/** Runs the whole jump; resolves true when it glided to the end, false when the user cut it short. */
function jumpTo(win: Window, target: HTMLElement, button: HTMLElement | null): Promise<boolean> {
  const doc = win.document;
  const to = scrollTarget(win, target);
  const lifted = button === null ? null : lift(doc, button);
  const btl = button === null || lifted === null ? null : buttonTimeline(button, () => {
    lifted.release();
  });
  const copy = copyTimeline(button);
  btl?.play();
  copy.tl?.play();
  const from = win.scrollY;
  const glide = tween({
    from,
    to,
    duration: Math.min(1.25, 0.7 + Math.abs(to - from) / 2400),
    delay: 0.22,
    ease: power3InOut,
    onUpdate: (y) => {
      win.scrollTo({ top: y, behavior: "instant" });
    },
  });
  const events = ["wheel", "touchstart", "keydown"] as const;
  const cut = (): void => {
    glide.cancel();
  };
  for (const ev of events) {
    win.addEventListener(ev, cut, { passive: true });
  }
  return glide.done.then((completed) => {
    for (const ev of events) {
      win.removeEventListener(ev, cut);
    }
    copy.tl?.kill();
    for (const el of copy.rest) {
      el.style.removeProperty("transform");
      el.style.removeProperty("opacity");
    }
    if (!completed) {
      btl?.kill();
      lifted?.release();
    }
    return completed;
  });
}

/** The link and target of a plain pointer click on a `data-jump` link, or null. */
function jumpOf(e: MouseEvent, doc: Document): { a: HTMLAnchorElement; target: HTMLElement } | null {
  const a = isPlainPointerClick(e) && e.target instanceof Element ? e.target.closest<HTMLAnchorElement>("a[data-jump]") : null;
  const href = a?.getAttribute("href") ?? "";
  const target = a !== null && href.length > 1 && href.startsWith("#") ? doc.getElementById(href.slice(1)) : null;
  return a === null || target === null ? null : { a, target };
}

export function wireJumps(doc: Document, win: Window): void {
  const storage = safeStorage(win);
  let running = false;
  doc.addEventListener("click", (e) => {
    const jump = jumpOf(e, doc);
    if (jump === null || running || !motionAllowed(win, storage)) {
      return;
    }
    e.preventDefault();
    running = true;
    const { a, target } = jump;
    void jumpTo(win, target, a.classList.contains("holo") ? a : null).then((completed) => {
      running = false;
      if (completed) {
        win.history.pushState(null, "", `#${target.id}`);
        target.focus({ preventScroll: true });
      }
    });
  });
}
