/**
 * The transition veil: a full-screen canvas running the field program in overlay mode. `wipeIn`
 * sweeps a re-scrambling mosaic over the screen top-left to bottom-right before a navigation.
 */
import { power2In } from "../anim/ease.ts";
import { tween } from "../anim/tween.ts";
import type { VeilApi } from "../runtime/fxRegistry.ts";
import { createFieldGl } from "./gl.ts";

/** Share of the wipe after which the hint starts to fade in. */
const HINT_FROM = 0.18;

export function createVeil(
  doc: Document,
  atlas: HTMLCanvasElement,
  createGl: typeof createFieldGl = createFieldGl,
): VeilApi | null {
  const canvas = doc.createElement("canvas");
  canvas.className = "veil";
  canvas.setAttribute("aria-hidden", "true");
  doc.body.append(canvas);
  const gl = createGl(canvas, { overlay: true, dprCap: 1 });
  if (gl === null) {
    canvas.remove();
    return null;
  }
  gl.setAtlas(atlas);
  const hint = doc.querySelector<HTMLElement>(".transit-hint");
  let running: { cancel(): void } | null = null;

  const reset = (): void => {
    running?.cancel();
    running = null;
    canvas.classList.remove("is-on");
    gl.u.wipe = 0;
    if (hint !== null) {
      hint.hidden = true;
      hint.style.setProperty("opacity", "0");
    }
  };

  const wipeIn = (seconds: number): Promise<void> => {
    if (gl.lost() || !gl.ready()) {
      return Promise.resolve();
    }
    reset();
    canvas.classList.add("is-on");
    gl.resize();
    if (hint !== null) {
      hint.hidden = false;
    }
    const t0 = performance.now();
    const run = tween({
      from: 0,
      to: 1,
      duration: seconds,
      ease: power2In,
      onUpdate: (v) => {
        gl.u.wipe = v;
        gl.u.time = 6.5 + (performance.now() - t0) / 1000;
        gl.render();
        if (hint !== null) {
          hint.style.setProperty("opacity", String(Math.max(0, (v - HINT_FROM) / (1 - HINT_FROM))));
        }
      },
    });
    running = run;
    return run.done.then(() => undefined);
  };

  return { wipeIn, reset };
}
