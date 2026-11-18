/** The sync tamper check: the server offers an older checkpoint and {name} stops to warn (prototype 4690). */
import { backOut, power3Out } from "../anim/ease.ts";
import type { Timeline } from "../anim/timeline.ts";
import { decodeTo, flag, pick, story, wait } from "./text.ts";

export function build(root: HTMLElement): Timeline | null {
  const led = pick(root, ".js-led");
  const server = pick(root, ".js-server");
  const cp = pick(root, ".js-server-cp");
  const chip = pick(root, ".js-server-chip");
  const btn = pick(root, ".js-sync-btn");
  const warn = pick(root, ".js-warn");
  const checked = pick(root, ".js-checked");
  const label = cp.textContent;
  const tl = story(2.2);
  tl.set(server, { opacity: 0.35 })
    .set(chip, { opacity: 0, scale: 0.8 })
    .set(warn, { opacity: 0, y: 6 })
    .set(checked, { opacity: 0 });
  flag(tl, led, ["is-bad", false]);
  tl.call(() => {
    cp.textContent = label.replace(/\d+$/, "————");
  })
    .to(btn, { scale: 0.95 }, { duration: 0.12, yoyo: true, repeat: 1 }, 0.5)
    .to(server, { opacity: 1 }, { duration: 0.3 });
  decodeTo(tl, cp, label, { duration: 0.6 });
  tl.to(chip, { opacity: 1, scale: 1 }, { duration: 0.35, ease: backOut(2) });
  flag(tl, led, ["is-bad", true]);
  tl.to(warn, { opacity: 1, y: 0 }, { duration: 0.45, ease: power3Out }, "+=0.2").to(checked, { opacity: 1 }, { duration: 0.3 }, "+=0.3");
  wait(tl, 2);
  return tl;
}
