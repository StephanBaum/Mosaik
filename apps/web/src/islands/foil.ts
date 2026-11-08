/**
 * The hero's holo-foil plane: the app's own foil shader (src/fx, verbatim copies of the
 * desktop holo code, pinned by fx/parity.test.ts) painted as a bare surface behind the app
 * window, like a light source under glass. It flows slowly, shifts its light a little towards
 * the cursor, and paints only while on screen (30 fps, rendered at reduced resolution and
 * scaled up). With reduced motion it paints one designed frame and stops. Without WebGL2 the
 * stylesheet's static gradient stays. Cosmetic only.
 */
import { drawFoil } from "../fx/draw.ts";
import { DEFAULT_SEED } from "../fx/shader.ts";
import { createStage, type FrameInfo, type Stage, type Target } from "../fx/stage.ts";

/** Render scale: the plane is soft light, so 0.6 of the CSS size is plenty. */
const SCALE = 0.6;
/** Reference unit (render px) of the stripe pattern; larger means broader bands. */
const UNIT = 900;
/** Stripe frequency against the button's (1): broad, calm bands for a large plane. */
const STRIPE_K = 0.14;
/** Seconds of shader time per second: slow, calm flow. */
const SPEED = 0.16;
/** The designed still for reduced motion. */
const STILL_TIME = 6.5;
const STILL_MOUSE = { x: -0.35, y: 0.25 };

class FoilPlane implements Target {
  private readonly ctx: CanvasRenderingContext2D | null;
  private time = STILL_TIME;
  private mouse = { ...STILL_MOUSE };
  private target = { ...STILL_MOUSE };
  private onScreen = false;
  private painted = false;

  constructor(
    private readonly host: HTMLElement,
    private readonly canvas: HTMLCanvasElement,
    private readonly reduced: boolean,
    private readonly kick: () => void,
  ) {
    this.ctx = canvas.getContext("2d");
    new ResizeObserver(() => {
      this.resize();
    }).observe(host);
    new IntersectionObserver((entries) => {
      this.onScreen = entries.some((e) => e.isIntersecting);
      if (this.onScreen) {
        kick();
      }
    }).observe(host);
    if (!reduced) {
      window.addEventListener(
        "pointermove",
        (event) => {
          const rect = host.getBoundingClientRect();
          if (rect.height > 0) {
            this.target = {
              x: ((event.clientX - (rect.left + rect.width / 2)) / rect.height) * 0.6,
              y: (-(event.clientY - (rect.top + rect.height / 2)) / rect.height) * 0.6,
            };
          }
        },
        { passive: true },
      );
    }
    this.resize();
  }

  private resize(): void {
    const w = Math.max(1, Math.round(this.host.clientWidth * SCALE));
    const h = Math.max(1, Math.round(this.host.clientHeight * SCALE));
    if (this.canvas.width !== w || this.canvas.height !== h) {
      this.canvas.width = w;
      this.canvas.height = h;
      this.painted = false;
      this.kick();
    }
  }

  visible(): boolean {
    return this.onScreen && (!this.reduced || !this.painted);
  }

  interval(): number {
    return 33;
  }

  step(frame: FrameInfo): boolean {
    if (this.reduced) {
      return false;
    }
    this.time += frame.dt * SPEED;
    const k = Math.min(1, frame.dt * 2.5);
    this.mouse.x += (this.target.x - this.mouse.x) * k;
    this.mouse.y += (this.target.y - this.mouse.y) * k;
    return true;
  }

  paint(stage: Stage): void {
    const { width, height } = this.canvas;
    if (this.ctx === null || width < 2 || height < 2) {
      return;
    }
    stage.ensure(width, height);
    stage.gl.useProgram(stage.holo.program);
    drawFoil(stage.gl, stage.holo, {
      width,
      height,
      dpr: 1,
      mouse: this.mouse,
      time: this.time,
      hover: 0,
      press: 0,
      focus: 0,
      dark: 0,
      seed: DEFAULT_SEED,
      mask: 0,
      stripeK: STRIPE_K,
      unit: [UNIT, width / height / 2],
    });
    stage.copy(this.ctx, width, height);
    if (!this.painted) {
      this.painted = true;
      this.host.classList.add("is-live");
    }
  }

  lost(): void {
    this.host.classList.remove("is-live");
  }
}

export function bootFoil(root: Document): void {
  const hosts = [...root.querySelectorAll<HTMLElement>("[data-island='foil']")];
  if (hosts.length === 0) {
    return;
  }
  const stage = createStage();
  if (stage === null) {
    return;
  }
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  for (const host of hosts) {
    const canvas = host.querySelector<HTMLCanvasElement>("canvas");
    if (canvas !== null) {
      stage.add(new FoilPlane(host, canvas, reduced, () => {
        stage.kick();
      }));
    }
  }
}
