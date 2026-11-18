// Provenance: verbatim copy of apps/desktop/src/holo/motion.ts (design concept §5, owner decision 7).
// Never edit this copy; change the desktop source and copy it again. src/fx/parity.test.ts pins it.

/**
 * Foil animation state: a pure port of the prototype's per-button easing
 * (docs/design/holo/holo-button.js `update`). No DOM, no GL.
 */
import { EFFECT_IDLE, focusRunning, rippleRunning } from "./effects.ts";
import { DEFAULT_SEED } from "./shader.ts";

export interface FoilState {
  /** Half the button's aspect ratio (width / height / 2). */
  bx: number;
  time: number;
  hover: number;
  hoverT: number;
  press: number;
  pressT: number;
  focus: number;
  focusT: number;
  /** Pointer position in button heights from the centre, eased and its target. */
  mx: number;
  my: number;
  mxT: number;
  myT: number;
  inside: boolean;
  active: boolean;
  /** Clock time of the last pointer leave or blur. */
  leaveAt: number;
  /** 1 when the button sits on a dark surface. */
  dark: number;
  ambient: boolean;
  seed: number;
  /** Needs a repaint even when nothing animates. */
  dirty: boolean;
  /** Press ripple: origin in button heights from the centre (y up) and age in seconds. */
  ripX: number;
  ripY: number;
  ripAge: number;
  /** Seconds since keyboard focus arrived (focus waves). */
  focusAge: number;
  /** A ripple or focus wave was running on the last update. */
  fxRunning: boolean;
}

export interface MotionEnv {
  reduced: boolean;
  slow: boolean;
}

/** How long the foil keeps easing after the pointer leaves (ms). */
const LEAVE_WINDOW_MS = 900;
const SETTLE = 0.002;

export function newState(
  options: { dark?: boolean; ambient?: boolean; seed?: number | undefined } = {},
): FoilState {
  return {
    bx: 1.5,
    time: 2,
    hover: 0,
    hoverT: 0,
    press: 0,
    pressT: 0,
    focus: 0,
    focusT: 0,
    mx: 0,
    my: 0.3,
    mxT: 0,
    myT: 0.3,
    inside: false,
    active: false,
    leaveAt: -1e9,
    dark: options.dark === true ? 1 : 0,
    ambient: options.ambient === true,
    seed: options.seed ?? DEFAULT_SEED,
    dirty: true,
    ripX: 0,
    ripY: 0,
    ripAge: EFFECT_IDLE,
    focusAge: EFFECT_IDLE,
    fxRunning: false,
  };
}

/** Starts the press ripple at `(x, y)` (button heights from the centre). Nothing under reduced motion. */
export function startRipple(s: FoilState, x: number, y: number, reduced: boolean): void {
  if (!reduced) {
    s.ripX = x;
    s.ripY = y;
    s.ripAge = 0;
  }
}

/** Starts the focus waves and swell. Only call for keyboard focus; nothing under reduced motion. */
export function startFocusWaves(s: FoilState, reduced: boolean): void {
  if (!reduced) {
    s.focusAge = 0;
  }
}

/** Exponential approach to `target`; jumps straight there under reduced motion. */
export function approach(
  current: number,
  target: number,
  step: { rate: number; dt: number; reduced: boolean },
): number {
  return step.reduced
    ? target
    : current + (target - current) * (1 - Math.exp(-step.rate * step.dt));
}

/** Where the virtual pointer wanders while nothing touches the button. */
export function idleTarget(s: Pick<FoilState, "time" | "seed" | "bx">): { x: number; y: number } {
  return {
    x: Math.sin(s.time * 0.7 + s.seed) * s.bx * 0.55,
    y: 0.3 + 0.12 * Math.sin(s.time * 0.98 + s.seed * 2),
  };
}

function setPointerTarget(s: FoilState): void {
  if (s.inside) {
    return;
  }
  if (s.focusT > 0.5) {
    s.mxT = Math.sin(s.time * 0.6) * s.bx * 0.4;
    s.myT = 0.25;
    return;
  }
  const idle = idleTarget(s);
  s.mxT = idle.x;
  s.myT = idle.y;
}

/** Ages the ripple and focus waves; returns whether one is still running. */
function stepEffects(s: FoilState, dt: number, reduced: boolean): boolean {
  let running = false;
  if (reduced) {
    s.ripAge = EFFECT_IDLE;
    s.focusAge = EFFECT_IDLE;
  } else {
    s.ripAge += dt;
    s.focusAge += dt;
    running = rippleRunning(s.ripAge) || focusRunning(s.focusAge);
  }
  if (s.fxRunning && !running) {
    s.dirty = true;
  }
  s.fxRunning = running;
  return running;
}

/** Pointer, press or an easing value still moving (a focused button only animates its waves). */
function isBusy(s: FoilState, now: number, hoverTarget: number): boolean {
  return (
    s.inside ||
    s.pressT > 0 ||
    now - s.leaveAt < LEAVE_WINDOW_MS ||
    Math.abs(s.hover - hoverTarget) > SETTLE ||
    Math.abs(s.press - s.pressT) > SETTLE ||
    Math.abs(s.focus - s.focusT) > SETTLE
  );
}

/** Advances `s` by `dt` seconds at clock time `now` (ms). */
export function update(s: FoilState, dt: number, now: number, env: MotionEnv): void {
  const { reduced } = env;
  const ambient = s.ambient && !env.slow && !reduced;
  setPointerTarget(s);
  const hoverTarget = Math.max(s.hoverT, s.focusT * 0.6);
  const step = (rate: number) => ({ rate, dt, reduced });
  s.hover = approach(s.hover, hoverTarget, step(9));
  s.press = approach(s.press, s.pressT, step(22));
  s.focus = approach(s.focus, s.focusT, step(12));
  const pointerStep = step(s.inside ? 24 : 7);
  s.mx = approach(s.mx, s.mxT, pointerStep);
  s.my = approach(s.my, s.myT, pointerStep);
  const fx = stepEffects(s, dt, reduced);
  s.active = !reduced && (ambient || fx || isBusy(s, now, hoverTarget));
  if (s.active) {
    s.time += dt;
  } else {
    s.mx = s.mxT;
    s.my = s.myT;
  }
}
