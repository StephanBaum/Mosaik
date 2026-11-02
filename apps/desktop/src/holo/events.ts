/** Pointer, keyboard and focus tracking of one foil button (port of the prototype's event wiring). */
import { startFocusWaves, startRipple, type FoilState } from "./motion.ts";

export interface EventHooks {
  /** Requests a repaint (marks the state dirty and queues a frame). */
  poke: () => void;
  now: () => number;
  reduced: () => boolean;
}

const PRESSED = "holo--pressed";

interface Wiring {
  button: HTMLButtonElement;
  s: FoilState;
  hooks: EventHooks;
  on: <K extends keyof HTMLElementEventMap>(
    type: K,
    handler: (event: HTMLElementEventMap[K]) => void,
  ) => void;
  pressStart: (x: number, y: number) => void;
  pressEnd: () => void;
}

function focusVisible(button: HTMLButtonElement): boolean {
  try {
    return button.matches(":focus-visible");
  } catch {
    return true;
  }
}

function wirePointer({ button, s, hooks, on, pressStart, pressEnd }: Wiring): void {
  const local = (event: PointerEvent): void => {
    if (hooks.reduced()) {
      return;
    }
    const rect = button.getBoundingClientRect();
    if (rect.height === 0) {
      return;
    }
    s.mxT = (event.clientX - (rect.left + rect.width / 2)) / rect.height;
    s.myT = -(event.clientY - (rect.top + rect.height / 2)) / rect.height;
  };
  on("pointerenter", (event) => {
    s.inside = true;
    s.hoverT = 1;
    local(event);
    hooks.poke();
  });
  on("pointermove", (event) => {
    if (!hooks.reduced()) {
      local(event);
      hooks.poke();
    }
  });
  on("pointerleave", () => {
    s.inside = false;
    s.hoverT = 0;
    s.leaveAt = hooks.now();
    pressEnd();
  });
  on("pointerdown", (event) => {
    if (event.button === 0) {
      local(event);
      pressStart(s.mxT, s.myT);
    }
  });
  on("pointerup", pressEnd);
  on("pointercancel", () => {
    s.hoverT = 0;
    pressEnd();
  });
}

function wireKeyboardAndFocus({ button, s, hooks, on, pressStart, pressEnd }: Wiring): void {
  const isAction = (event: KeyboardEvent): boolean => event.key === " " || event.key === "Enter";
  on("keydown", (event) => {
    if (isAction(event) && !event.repeat) {
      s.mxT = 0;
      s.myT = 0.15;
      pressStart(0, 0);
    }
  });
  on("keyup", (event) => {
    if (isAction(event)) {
      pressEnd();
    }
  });
  on("focus", () => {
    const visible = focusVisible(button);
    s.focusT = visible ? 1 : 0;
    if (visible) {
      startFocusWaves(s, hooks.reduced());
    }
    hooks.poke();
  });
  on("blur", () => {
    s.focusT = 0;
    s.leaveAt = hooks.now();
    pressEnd();
  });
}

/** Attaches the listeners; returns the function that removes them and the pressed class. */
export function attachEvents(button: HTMLButtonElement, s: FoilState, hooks: EventHooks): () => void {
  const off: (() => void)[] = [];
  const on: Wiring["on"] = (type, handler) => {
    button.addEventListener(type, handler);
    off.push(() => {
      button.removeEventListener(type, handler);
    });
  };
  const pressStart = (x: number, y: number): void => {
    s.pressT = 1;
    startRipple(s, x, y, hooks.reduced());
    button.classList.add(PRESSED);
    hooks.poke();
  };
  const pressEnd = (): void => {
    s.pressT = 0;
    button.classList.remove(PRESSED);
    hooks.poke();
  };
  const wiring: Wiring = { button, s, hooks, on, pressStart, pressEnd };
  wirePointer(wiring);
  wireKeyboardAndFocus(wiring);
  return () => {
    for (const remove of off) {
      remove();
    }
    button.classList.remove(PRESSED);
  };
}
