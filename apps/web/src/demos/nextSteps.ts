/** Next steps: the line draws, the cards tick (prototype 4875). Plays once, when the list scrolls into view. */
import { backOut, power2InOut } from "../anim/ease.ts";
import type { Timeline } from "../anim/timeline.ts";
import { pickAll, pickSvg, story } from "./text.ts";

/** The path dash length (user units); the stylesheet sets the matching `stroke-dasharray`. */
const LENGTH = 1000;

export function build(root: HTMLElement): Timeline | null {
  const path = pickSvg(root, ".js-steps-path");
  const ticks = pickAll(root, ".js-tick");
  return story(0, 1)
    .set(path, { strokeDashoffset: LENGTH })
    .set(ticks, { opacity: 0, scale: 0.6 })
    .to(path, { strokeDashoffset: 0 }, { duration: 1.4, ease: power2InOut })
    .to(ticks, { opacity: 1, scale: 1 }, { duration: 0.35, stagger: 0.2, ease: backOut(2) }, 0.3);
}
