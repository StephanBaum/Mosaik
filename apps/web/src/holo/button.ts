/**
 * The holo primary action as a stage target. The foil maths, easing and draw call are the
 * app's (copied holo/motion.ts and holo/draw.ts); the wiring is ported from
 * apps/desktop/src/holo/events.ts for links, which have no Space activation.
 * At most one holo button is visible at a time (ADR-0014). Cosmetic only.
 */
import { drawFoil } from "../fx/draw.ts";
import { FOIL_MARGIN } from "../fx/effects.ts";
import { newState, startFocusWaves, startRipple, update, type FoilState } from "../fx/motion.ts";
import { DEFAULT_SEED } from "../fx/shader.ts";
import type { FrameInfo, Stage, Target } from "../fx/stage.ts";

const LIVE = "holo--live";
const PRESSED = "holo--pressed";
const MAX_DPR = 2;

export interface HoloEnv {
  reduced: boolean;
  dark: () => boolean;
  visible: () => boolean;
  poke: () => void;
}

function dpr(): number {
  return Math.min(window.devicePixelRatio || 1, MAX_DPR);
}

function wire(el: HTMLElement, s: FoilState, env: HoloEnv): void {
  const local = (event: PointerEvent): void => {
    const rect = el.getBoundingClientRect();
    if (rect.height > 0 && !env.reduced) {
      s.mxT = (event.clientX - (rect.left + rect.width / 2)) / rect.height;
      s.myT = -(event.clientY - (rect.top + rect.height / 2)) / rect.height;
    }
  };
  const press = (on: boolean): void => {
    s.pressT = on ? 1 : 0;
    el.classList.toggle(PRESSED, on);
    env.poke();
  };
  el.addEventListener("pointerenter", (event) => {
    s.inside = true;
    s.hoverT = 1;
    local(event);
    env.poke();
  });
  el.addEventListener("pointermove", (event) => {
    local(event);
    env.poke();
  });
  el.addEventListener("pointerleave", () => {
    s.inside = false;
    s.hoverT = 0;
    s.leaveAt = performance.now();
    press(false);
  });
  el.addEventListener("pointerdown", (event) => {
    if (event.button === 0) {
      local(event);
      startRipple(s, s.mxT, s.myT, env.reduced);
      press(true);
    }
  });
  el.addEventListener("pointerup", () => {
    press(false);
  });
  el.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.repeat) {
      startRipple(s, 0, 0, env.reduced);
      env.poke();
    }
  });
  el.addEventListener("focus", () => {
    const visible = el.matches(":focus-visible");
    s.focusT = visible ? 1 : 0;
    if (visible) {
      startFocusWaves(s, env.reduced);
    }
    env.poke();
  });
  el.addEventListener("blur", () => {
    s.focusT = 0;
    s.leaveAt = performance.now();
    press(false);
  });
}

/** The pill-shaped holo CTA: `.holo` element with a `.holo__foil` canvas inside. */
export class HoloButtonTarget implements Target {
  private readonly s: FoilState;
  private ctx: CanvasRenderingContext2D | null = null;

  constructor(
    private readonly el: HTMLElement,
    private readonly foil: HTMLCanvasElement,
    private readonly env: HoloEnv,
  ) {
    this.s = newState({ dark: env.dark(), seed: DEFAULT_SEED });
    wire(el, this.s, env);
    new ResizeObserver(() => {
      this.s.dirty = true;
      env.poke();
    }).observe(el);
  }

  visible(): boolean {
    return this.env.visible();
  }

  interval(): number {
    return 16;
  }

  step(frame: FrameInfo): boolean {
    this.s.dark = this.env.dark() ? 1 : 0;
    update(this.s, frame.dt, frame.now, { reduced: this.env.reduced, slow: false });
    return this.s.active;
  }

  invalidate(): void {
    this.s.dirty = true;
  }

  lost(): void {
    this.el.classList.remove(LIVE, PRESSED);
  }

  paint(stage: Stage): void {
    const s = this.s;
    if (!s.active && !s.dirty) {
      return;
    }
    const ratio = dpr();
    const w = Math.round((this.el.offsetWidth + 2 * FOIL_MARGIN) * ratio);
    const h = Math.round((this.el.offsetHeight + 2 * FOIL_MARGIN) * ratio);
    if (w === 0 || h === 0) {
      return;
    }
    if (this.foil.width !== w || this.foil.height !== h) {
      this.foil.width = w;
      this.foil.height = h;
    }
    s.bx = (0.5 * this.el.offsetWidth) / Math.max(this.el.offsetHeight, 1);
    stage.ensure(w, h);
    stage.gl.useProgram(stage.holo.program);
    drawFoil(stage.gl, stage.holo, {
      width: w,
      height: h,
      dpr: ratio,
      mouse: { x: s.mx, y: s.my },
      time: s.time,
      hover: s.hover,
      press: s.press,
      focus: s.focus,
      dark: s.dark,
      seed: s.seed,
      mask: 1,
      stripeK: 1,
      unit: [0, 0],
      margin: Math.round(FOIL_MARGIN * ratio),
      ripple: { x: s.ripX, y: s.ripY, age: s.ripAge },
      focusAge: s.focusAge,
    });
    this.ctx ??= this.foil.getContext("2d");
    if (this.ctx !== null) {
      stage.copy(this.ctx, w, h);
    }
    s.dirty = false;
    this.el.classList.add(LIVE);
  }
}
