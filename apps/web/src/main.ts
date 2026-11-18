/**
 * Client entry. HTML and CSS paint every page on their own. This entry wires the header menu,
 * the footer motion toggle and the scroll reveals, loads only the islands a page marks with
 * `data-island`, and decides the capability tier (`html[data-fx]`) once the page has loaded and
 * is idle. The effects themselves (field, micro-interactions) join the entry's idle hook in
 * the tasks that add them. No React and no ICU code ship to the browser.
 */
import "virtual:mosaic-tokens.css";
import "./styles/index.css";
import { wireMenu } from "./menu.ts";
import { wireJumps } from "./nav/jump.ts";
import { wireNav } from "./nav/wireNav.ts";
import { fxTier, readEnv } from "./runtime/capability.ts";
import { afterLoadIdle, afterLoadSettled } from "./runtime/idle.ts";
import { motionAllowed, safeStorage, wireMotionToggle } from "./runtime/motion.ts";
import { wireReveals } from "./runtime/reveal.ts";

const storage = safeStorage(window);

wireMenu(document);
wireMotionToggle(document, storage);
wireReveals(document, motionAllowed(window, storage));
wireNav(document, window);
wireJumps(document, window);

/** An island boots synchronously, or returns a promise that settles when its visible work is done. */
type Boot = (root: Document) => void | Promise<void>;

const ISLANDS: Readonly<Record<string, () => Promise<Boot>>> = {
  holo: () => import("./holo/index.ts").then((m) => m.bootHolo),
  downloads: () =>
    import("./downloads.ts").then((m) => (root: Document) => {
      m.wireDownloads(root, navigator);
    }),
  demos: () => import("./demos/bootDemos.ts").then((m) => m.bootDemos),
  decode: () => import("./demos/decode.ts").then((m) => m.bootDecode),
  print: () => import("./islands/print.ts").then((m) => m.bootPrint),
  selfcheck: () => import("./islands/selfcheck.ts").then((m) => m.bootSelfCheck),
};

interface ConnectionHint {
  connection?: { saveData?: boolean };
}

/** Loads one island and resolves when its boot is done. */
async function runIsland(name: string): Promise<void> {
  const boot = await ISLANDS[name]?.();
  await boot?.(document);
}

const nextFrames = (): Promise<void> =>
  new Promise((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        resolve();
      });
    });
  });

const wantedIslands = (): Set<string> => new Set([...document.querySelectorAll<HTMLElement>("[data-island]")].map((el) => el.dataset["island"] ?? ""));

/** Islands that rewrite visible hero text: their new text is an LCP candidate, so they run first. */
const EARLY = ["decode", "downloads"];

async function startEarly(): Promise<void> {
  const wanted = wantedIslands();
  await Promise.all(EARLY.filter((name) => wanted.has(name)).map((name) => runIsland(name).catch(() => undefined)));
}

function startIslands(): void {
  const saveData = (navigator as Navigator & ConnectionHint).connection?.saveData === true;
  for (const name of wantedIslands()) {
    // Holo buttons load on every tier except Save-Data; they keep their own reduced-motion handling.
    if (!EARLY.includes(name) && name in ISLANDS && !(saveData && name === "holo")) {
      void runIsland(name);
    }
  }
}

function startEffects(): void {
  const tier = fxTier(readEnv(window, storage));
  document.documentElement.dataset["fx"] = tier;
  if (tier !== "none") {
    void import("./field/bootField.ts").then((m) => m.bootField(document, tier));
    void import("./micro/index.ts").then((m) => {
      m.bootMicro(document);
    });
  }
}

// After `load` and two frames: the islands that rewrite hero text. Then, once the intro animations are over and the
// largest contentful paint has settled, the other islands and the field, so that no chunk request
// competes with the paint that decides LCP.
function afterLoad(fn: () => void): void {
  if (document.readyState === "complete") {
    fn();
  } else {
    window.addEventListener("load", fn, { once: true });
  }
}

afterLoad(() => {
  void nextFrames()
    .then(startEarly)
    .then(() => {
      afterLoadSettled(() => {
        startIslands();
        afterLoadIdle(startEffects);
      });
    });
});
