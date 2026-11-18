/** The agent session: lines appear one by one, the last request to another host is refused (prototype 4721). */
import { power2Out } from "../anim/ease.ts";
import type { Position, Timeline } from "../anim/timeline.ts";
import { after, pick, pickAll, story, typeTo, wait } from "./text.ts";

const GAPS = [0, 0.5, 0.6, 0.7, 0.5, 1.1, 0.9, 0.6, 1.0] as const;

export function build(root: HTMLElement): Timeline | null {
  const lines = pickAll(root, ".da__line").filter((l) => !l.classList.contains("js-caret-line"));
  const caret = pick(root, ".js-caret-line");
  const typed = root.querySelector<HTMLElement>(".js-type");
  const userText = typed?.textContent ?? "";
  const tl = story(3.5);
  tl.set(lines, { opacity: 0, y: 4 }).set(caret, { opacity: 1 });
  lines.forEach((line, i) => {
    const gap = GAPS[i] ?? 0.5;
    const at: Position = i === 0 ? 0.4 : after(gap);
    if (i === 0 && typed !== null) {
      tl.set(line, { opacity: 1, y: 0 }, at).call(() => {
        typed.textContent = "";
      }, "<");
      typeTo(tl, typed, userText, { duration: 1.4 });
    } else {
      tl.to(line, { opacity: 1, y: 0 }, { duration: 0.35, ease: power2Out }, at);
    }
  });
  wait(tl, 1);
  return tl;
}
