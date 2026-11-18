/**
 * Exact-origin fill: the cursor opens the popup on the real address, the login fills; then the
 * same page on a look-alike address, where nothing fills. Prototype 4613, on the in-house
 * timeline. Every word comes from the markup's data attributes.
 */
import { backOut, power1InOut, power3InOut, power3Out } from "../anim/ease.ts";
import type { Timeline } from "../anim/timeline.ts";
import { centerIn, decodeTo, flag, pick, pickAll, story, typeTo, wait } from "./text.ts";

/** The address with its digit one set apart, built from nodes (no HTML strings). */
function markBad(el: Element, bad: string): void {
  const at = bad.indexOf("1");
  const mark = el.ownerDocument.createElement("span");
  mark.className = "bad";
  mark.textContent = "1";
  el.replaceChildren(...[bad.slice(0, at), mark, bad.slice(at + 1)].filter((n) => n !== ""));
}

interface Parts {
  page: HTMLElement;
  url: HTMLElement;
  urlText: HTMLElement;
  tag: HTMLElement;
  email: HTMLElement;
  emailText: HTMLElement;
  popup: HTMLElement;
  popupSite: HTMLElement;
  go: HTMLElement;
  dots: HTMLElement[];
  cursor: HTMLElement;
  status: HTMLElement;
  statusText: HTMLElement;
}

function parts(root: HTMLElement): Parts {
  return {
    page: pick(root, ".js-page"),
    url: pick(root, ".js-url"),
    urlText: pick(root, ".js-url-text"),
    tag: pick(root, ".js-tag"),
    email: pick(root, ".js-email"),
    emailText: pick(root, ".js-email-text"),
    popup: pick(root, ".js-popup"),
    popupSite: pick(root, ".js-popup-site"),
    go: pick(root, ".js-go"),
    dots: pickAll(root, ".js-dots i"),
    cursor: pick(root, ".js-cursor"),
    status: pick(root, ".js-status"),
    statusText: pick(root, ".js-status-text"),
  };
}

/** The Fill button centre. The markup rests on the look-alike scene, where that row is hidden. */
function measureGo(p: Parts): { x: number; y: number } {
  const hidden = p.popup.classList.contains("is-no");
  p.popup.classList.remove("is-no");
  const at = centerIn(p.go, p.page);
  p.popup.classList.toggle("is-no", hidden);
  return at;
}

/** Scene one: the real address, the popup, the fill. */
function realAddress(tl: Timeline, p: Parts, d: DOMStringMap): void {
  const pw = p.page.clientWidth;
  const ph = p.page.clientHeight;
  const toEmail = centerIn(p.email, p.page, 0.55, 0.6);
  tl.set(p.cursor, { x: pw * 0.82, y: ph * 0.88, opacity: 1 })
    .set(p.tag, { opacity: 0 })
    .set(p.popup, { opacity: 0, scale: 0.96, y: 6 })
    .set(p.status, { xPercent: -50, opacity: 0, y: 8 })
    .set(p.dots, { scale: 0 })
    .call(() => {
      p.urlText.textContent = d["site"] ?? "";
      p.popupSite.textContent = d["site"] ?? "";
      p.emailText.textContent = "";
      p.statusText.textContent = d["textOk"] ?? "";
    })
    .call(() => {
      for (const [el, name] of [[p.url, "is-bad"], [p.popup, "is-no"], [p.status, "is-no"], [p.email, "is-focus"]] as const) {
        el.classList.remove(name);
      }
    })
    .to(p.cursor, { x: toEmail.x, y: toEmail.y }, { duration: 0.8, ease: power3InOut }, 0.3);
  flag(tl, p.email, ["is-focus", true]);
  tl.to(p.popup, { opacity: 1, scale: 1, y: 0 }, { duration: 0.4, ease: backOut(1.6) });
  const g = measureGo(p);
  tl.to(p.cursor, { x: g.x, y: g.y }, { duration: 0.55, ease: power3InOut }, "+=0.25");
  wait(tl, 0.6, "<");
  tl.to(p.go, { scale: 0.92 }, { duration: 0.1, yoyo: true, repeat: 1, ease: power1InOut })
    .to(p.popup, { opacity: 0, scale: 0.97, y: 4 }, { duration: 0.25, ease: power3InOut }, "+=0.05");
  flag(tl, p.email, ["is-focus", false]);
  typeTo(tl, p.emailText, d["user"] ?? "", { duration: 0.55 });
  tl.to(p.dots, { scale: 1 }, { duration: 0.25, stagger: 0.03, ease: backOut(2) }, "-=0.1")
    .to(p.status, { opacity: 1, y: 0 }, { duration: 0.4, ease: power3Out })
    .to(p.cursor, { x: pw * 0.78, y: ph * 0.8 }, { duration: 0.8, ease: power3InOut }, "+=0.2");
  wait(tl, 1.3);
}

/** Scene two: the look-alike address, where nothing fills. */
function lookalikeAddress(tl: Timeline, p: Parts, d: DOMStringMap): void {
  const toEmail = centerIn(p.email, p.page, 0.55, 0.6);
  const bad = d["bad"] ?? "";
  tl.to(p.status, { opacity: 0, y: 8 }, { duration: 0.3, ease: power3InOut });
  decodeTo(tl, p.urlText, bad, { duration: 0.7 });
  flag(tl, p.url, ["is-bad", true]);
  tl.set(p.tag, { opacity: 1 }).call(() => {
    markBad(p.urlText, bad);
    p.emailText.textContent = "";
    p.popupSite.textContent = bad;
    p.statusText.textContent = d["textNo"] ?? "";
  });
  tl.set(p.dots, { scale: 0 });
  flag(tl, p.popup, ["is-no", true]);
  flag(tl, p.status, ["is-no", true]);
  tl.to(p.cursor, { x: toEmail.x, y: toEmail.y }, { duration: 0.8, ease: power3InOut }, "+=0.3");
  flag(tl, p.email, ["is-focus", true]);
  tl.to(p.popup, { opacity: 1, scale: 1, y: 0 }, { duration: 0.4, ease: backOut(1.6) })
    .to(p.status, { opacity: 1, y: 0 }, { duration: 0.4, ease: power3Out }, "+=0.5");
  wait(tl, 1.6);
}

export function build(root: HTMLElement): Timeline | null {
  const p = parts(root);
  const tl = story(1.4);
  realAddress(tl, p, root.dataset);
  lookalikeAddress(tl, p, root.dataset);
  return tl;
}
