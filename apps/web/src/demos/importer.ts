/** Import: a file drops in, is read, previewed with duplicates marked, and imported; three sources in turn (prototype 4743). */
import { backOut, power2InOut, power2Out } from "../anim/ease.ts";
import type { Timeline } from "../anim/timeline.ts";
import { pick, pickAll, story, wait } from "./text.ts";

/** Fixed example sources: file name, items, duplicates. The chips in the markup are in the same order. */
const SOURCES = [
  ["Chrome Passwords.csv", 184, 1],
  ["1Password.1pux", 312, 3],
  ["lastpass_export.csv", 96, 2],
] as const;

interface Parts {
  srcs: HTMLElement[];
  file: HTMLElement;
  fileName: HTMLElement;
  dropText: HTMLElement;
  bar: HTMLElement;
  rows: HTMLElement[];
  count: HTMLElement;
  dups: HTMLElement;
  btn: HTMLElement;
  done: HTMLElement;
  drop: string;
  reading: string;
  preview: string;
}

function scene(tl: Timeline, p: Parts, source: (typeof SOURCES)[number], i: number): void {
  const [name, n, d] = source;
  const c = { n: 0 };
  tl.call(() => {
    p.srcs.forEach((s, j) => s.classList.toggle("is-on", j === i));
    p.fileName.textContent = name;
    p.dropText.textContent = p.drop;
    p.count.textContent = "0";
    p.dups.textContent = "0";
  })
    .set(p.rows, { opacity: 0, x: -6 })
    .set(p.done, { opacity: 0 })
    .set(p.bar, { scaleX: 0 })
    .set(p.file, { opacity: 0, xPercent: -50, yPercent: -50, x: -140, y: -90 })
    .to(p.file, { opacity: 1, x: 0, y: 0 }, { duration: 0.8, ease: backOut(1.3) }, "+=0.3")
    .call(() => {
      p.dropText.textContent = `${p.reading} ${name}…`;
    })
    .to(p.file, { opacity: 0, scale: 0.9 }, { duration: 0.3 }, "+=0.3")
    .to(p.bar, { scaleX: 1 }, { duration: 0.7, ease: power2InOut }, "<")
    .call(() => {
      p.dropText.textContent = `${p.preview} · ${name}`;
    })
    .to(p.rows, { opacity: 1, x: 0 }, { duration: 0.35, stagger: 0.09, ease: power2Out })
    .to(
      c,
      { n },
      {
        duration: 0.6,
        ease: power2Out,
        onUpdate: () => {
          p.count.textContent = String(Math.round(c.n));
        },
      },
      "-=0.3",
    )
    .call(() => {
      p.dups.textContent = String(d);
    })
    .to(p.btn, { scale: 0.95 }, { duration: 0.12, yoyo: true, repeat: 1 }, "+=0.5")
    .to(p.done, { opacity: 1 }, { duration: 0.3 });
  wait(tl, i === SOURCES.length - 1 ? 1.5 : 1.4);
}

export function build(root: HTMLElement): Timeline | null {
  const drop = pick(root, ".js-drop");
  const p: Parts = {
    srcs: pickAll(root, ".js-src"),
    file: pick(root, ".js-file"),
    fileName: pick(root, ".js-file-name"),
    dropText: pick(root, ".js-drop-text"),
    bar: pick(root, ".js-bar"),
    rows: pickAll(root, ".js-row"),
    count: pick(root, ".js-count"),
    dups: pick(root, ".js-dups"),
    btn: pick(root, ".js-import-btn"),
    done: pick(root, ".js-done"),
    drop: drop.dataset["textDrop"] ?? "",
    reading: drop.dataset["textReading"] ?? "",
    preview: drop.dataset["textPreview"] ?? "",
  };
  const tl = story(1.2);
  SOURCES.forEach((source, i) => {
    scene(tl, p, source, i);
  });
  return tl;
}
