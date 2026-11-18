/**
 * Boots the holo foil buttons (ADR-0014): the hero's and the download section's primary
 * action. One shared WebGL2 stage (src/fx/stage.ts) paints them; a button paints only while it
 * is on screen, and the two never are at the same time. Without WebGL2 they keep the static
 * token gradient from buttons.css. Cosmetic only.
 */
import { createStage } from "../fx/stage.ts";
import { HoloButtonTarget } from "./button.ts";

export function bootHolo(root: Document): void {
  const buttons = [...root.querySelectorAll<HTMLElement>(".holo")];
  if (buttons.length === 0) {
    return;
  }
  const stage = createStage();
  if (stage === null) {
    return;
  }
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  for (const el of buttons) {
    const foil = el.querySelector<HTMLCanvasElement>(".holo__foil");
    if (foil === null) {
      continue;
    }
    let visible = false;
    const target = new HoloButtonTarget(el, foil, {
      reduced,
      // The landing page is dark only.
      dark: () => true,
      visible: () => visible,
      poke: () => {
        stage.kick();
      },
    });
    new IntersectionObserver((entries) => {
      for (const entry of entries) {
        visible = entry.isIntersecting;
        if (visible) {
          target.invalidate();
          stage.kick();
        }
      }
    }).observe(el);
    stage.add(target);
  }
}
