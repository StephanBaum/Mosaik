/** Sealed rows: a few groups re-scramble from a fixed sequence, then settle on the markup values (prototype 4857). */
import { linear } from "../anim/ease.ts";
import type { Timeline } from "../anim/timeline.ts";
import { hexChar, nextInt, pickAll, story } from "./text.ts";

const SECONDS = 5;
/** One scramble step every ~0.11 s. */
const STEPS = 45;

export function build(root: HTMLElement): Timeline | null {
  const spans = pickAll(root, ".cipher__hex span");
  if (spans.length === 0) {
    return null;
  }
  const original = spans.map((s) => s.textContent);
  const clock = { t: 0 };
  let step = 0;
  return story(1).to(
    clock,
    { t: 1 },
    {
      duration: SECONDS,
      ease: linear,
      onUpdate: () => {
        if (clock.t >= 1) {
          spans.forEach((s, i) => {
            s.textContent = original[i] ?? "";
          });
          step = 0;
          return;
        }
        const now = Math.floor(clock.t * STEPS);
        for (; step < now; step++) {
          for (let k = 0; k < 3; k++) {
            const s = spans[nextInt(spans.length)];
            if (s !== undefined) {
              s.textContent = hexChar() + hexChar() + hexChar() + hexChar();
            }
          }
        }
      },
    },
  );
}
