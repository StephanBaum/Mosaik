/**
 * Page links wipe the screen with the veil before navigating (capable tier only); the browser's
 * cross-document view transition (transitions.css) covers the rest. A page restored from the
 * back-forward cache never stays covered. A transition the browser aborts (the old page took too
 * long, as during a slow veil wipe, or its state changed) rejects the transition's promises with
 * an `InvalidStateError`; nobody else awaits them, so they are caught here on both pages and no
 * uncaught error reaches the console.
 */
import { getVeil, type VeilApi } from "../runtime/fxRegistry.ts";
import { motionAllowed, safeStorage } from "../runtime/motion.ts";
import { shouldWipe } from "./intercept.ts";

/** Cross-document view transitions are not in every `lib.dom`; only the bits we use. */
interface ViewTransitionLike {
  ready?: Promise<unknown>;
  finished?: Promise<unknown>;
  updateCallbackDone?: Promise<unknown>;
  skipTransition(): void;
}

interface TransitionEventLike extends Event {
  viewTransition?: ViewTransitionLike | null;
}

const ignore = (): void => undefined;

/** Marks the transition's promises handled: an aborted or skipped transition is not an error. */
export function quietTransition(vt: ViewTransitionLike | null | undefined): void {
  if (vt === null || vt === undefined) {
    return;
  }
  for (const p of [vt.ready, vt.finished, vt.updateCallbackDone]) {
    p?.catch(ignore);
  }
}

const WIPE_SECONDS = 0.32;

export function wireNav(doc: Document, win: Window, veil: () => VeilApi | null = getVeil): void {
  const storage = safeStorage(win);
  let leaving = false;

  doc.addEventListener(
    "click",
    (e) => {
      const a = e.target instanceof Element ? e.target.closest<HTMLAnchorElement>("a[data-nav]") : null;
      if (a === null) {
        return;
      }
      const link = { href: a.href, target: a.target, hasDownload: a.hasAttribute("download"), isNav: true };
      if (!shouldWipe(e, link, { origin: win.location.origin, pathname: win.location.pathname })) {
        return;
      }
      const v = veil();
      if (v === null || !motionAllowed(win, storage)) {
        return;
      }
      e.preventDefault();
      if (leaving) {
        return;
      }
      leaving = true;
      const go = (): void => {
        win.location.assign(a.href);
      };
      v.wipeIn(WIPE_SECONDS).then(go, go);
    },
    true,
  );

  win.addEventListener("pageshow", (e) => {
    if (e.persisted) {
      leaving = false;
      veil()?.reset();
    }
  });

  win.addEventListener("pageswap", (e) => {
    const vt = (e as TransitionEventLike).viewTransition;
    quietTransition(vt);
    if (!motionAllowed(win, storage)) {
      vt?.skipTransition();
    }
  });

  // The incoming side: `pagereveal` fires before the first frame, which may come before this
  // module runs, so the transition still in flight is also read from the document.
  win.addEventListener("pagereveal", (e) => {
    quietTransition((e as TransitionEventLike).viewTransition);
  });
  quietTransition((doc as Document & { activeViewTransition?: ViewTransitionLike | null }).activeViewTransition);
}
