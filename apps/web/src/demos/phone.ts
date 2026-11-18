/** Phone autofill: tap the field, pick the suggestion, confirm with the face check, the login fills (prototype 4799). */
import { backOut, power2Out, power3In, power3Out } from "../anim/ease.ts";
import type { Position, Timeline } from "../anim/timeline.ts";
import { after, centerIn, flag, pick, pickAll, story, typeTo, wait } from "./text.ts";

interface Parts {
  phone: HTMLElement;
  user: HTMLElement;
  userText: HTMLElement;
  pdots: HTMLElement[];
  kb: HTMLElement;
  chip: HTMLElement;
  face: HTMLElement;
  tap: HTMLElement;
}

/** A tap ripple on `el`. */
function tapAt(tl: Timeline, p: Parts, el: Element, at: Position): void {
  const c = centerIn(el, p.phone);
  tl.set(p.tap, { x: c.x - 14, y: c.y - 14, opacity: 0.7, scale: 0.5 }, at).to(p.tap, { scale: 1.3, opacity: 0 }, { duration: 0.45, ease: power2Out }, "<");
}

export function build(root: HTMLElement): Timeline | null {
  const p: Parts = {
    phone: pick(root, ".dp__phone"),
    user: pick(root, ".js-puser"),
    userText: pick(root, ".js-puser-text"),
    pdots: pickAll(root, ".js-pdots i"),
    kb: pick(root, ".js-kb"),
    chip: pick(root, ".js-chip"),
    face: pick(root, ".js-face"),
    tap: pick(root, ".js-tap"),
  };
  const text = p.userText.textContent;
  const tl = story(2);
  tl.set(p.kb, { yPercent: 100 })
    .set(p.chip, { opacity: 0, y: 8 })
    .set(p.face, { opacity: 0, xPercent: -50, yPercent: -50, scale: 0.9 })
    .set(p.pdots, { scale: 0 });
  flag(tl, p.user, ["is-focus", false]);
  tl.call(() => {
    p.userText.textContent = "";
  });
  tapAt(tl, p, p.user, 0.5);
  flag(tl, p.user, ["is-focus", true], after(0.1));
  tl.to(p.kb, { yPercent: 0 }, { duration: 0.5, ease: power3Out }).to(p.chip, { opacity: 1, y: 0 }, { duration: 0.35, ease: backOut(1.6) }, "-=0.2");
  tapAt(tl, p, p.chip, after(0.6));
  tl.to(p.face, { opacity: 1, scale: 1 }, { duration: 0.3, ease: backOut(2) }, "+=0.1").to(p.face, { opacity: 0 }, { duration: 0.25 }, "+=0.9");
  typeTo(tl, p.userText, text, { duration: 0.4 });
  tl.to(p.pdots, { scale: 1 }, { duration: 0.2, stagger: 0.03, ease: backOut(2) }).to(p.kb, { yPercent: 100 }, { duration: 0.4, ease: power3In }, "+=0.4");
  wait(tl, 1.2);
  return tl;
}
