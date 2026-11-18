/**
 * The motion switch (WCAG 2.2.2): `prefers-reduced-motion` or the footer toggle turns every
 * auto-playing effect off. The choice lives in localStorage when the browser allows it; a blocked
 * or throwing storage simply means motion stays on.
 */
export const MOTION_KEY = "mosaic:motion";
export const MOTION_EVENT = "mosaic:motion";

type ReadStore = Pick<Storage, "getItem">;
type WriteStore = Pick<Storage, "setItem" | "removeItem">;

/** `localStorage`, or null when the accessor throws (private window, blocked site data). */
export function safeStorage(win: Window): Storage | null {
  try {
    return win.localStorage;
  } catch {
    return null;
  }
}

export function isMotionOff(storage: ReadStore | null): boolean {
  try {
    return storage?.getItem(MOTION_KEY) === "off";
  } catch {
    return false;
  }
}

/** Persists the choice, marks `html[data-motion="off"]` and tells the running effects. */
export function setMotionOff(off: boolean, storage: WriteStore | null, doc: Document): void {
  try {
    if (off) {
      storage?.setItem(MOTION_KEY, "off");
    } else {
      storage?.removeItem(MOTION_KEY);
    }
  } catch {
    // Not persisted; the choice still holds for this page view.
  }
  if (off) {
    doc.documentElement.dataset["motion"] = "off";
  } else {
    delete doc.documentElement.dataset["motion"];
  }
  doc.dispatchEvent(new CustomEvent<{ off: boolean }>(MOTION_EVENT, { detail: { off } }));
}

/** Motion may play: no reduced-motion preference and the toggle is not off. */
export function motionAllowed(win: Window, storage: ReadStore | null): boolean {
  return !win.matchMedia("(prefers-reduced-motion: reduce)").matches && !isMotionOff(storage);
}

/** Unhides the footer toggle (it needs JS) and keeps its state, label and `aria-pressed` in sync. */
export function wireMotionToggle(doc: Document, storage: (ReadStore & WriteStore) | null): void {
  const button = doc.querySelector<HTMLButtonElement>("button[data-motion-toggle]");
  if (button === null) {
    return;
  }
  const sync = (): void => {
    const off = isMotionOff(storage);
    if (off) {
      doc.documentElement.dataset["motion"] = "off";
    }
    button.setAttribute("aria-pressed", String(off));
    button.textContent = (off ? button.dataset["labelPlay"] : button.dataset["labelPause"]) ?? button.textContent;
  };
  button.hidden = false;
  sync();
  button.addEventListener("click", () => {
    setMotionOff(!isMotionOff(storage), storage, doc);
    sync();
  });
}
