/**
 * Client entry. HTML and CSS paint every page on their own. This entry wires the header menu
 * and then loads only the islands the page marks with `data-island` (website v2 §1.4): the holo
 * buttons, the hero's foil plane, the download page's system detection, the sticky step stage,
 * the print button and the self-check. Pages without islands load nothing else. No React and no ICU code
 * ship to the browser.
 */
import "virtual:mosaic-tokens.css";
import "./styles/index.css";
import { wireMenu } from "./menu.ts";

wireMenu(document);

type Boot = (root: Document) => void;

const ISLANDS: Readonly<Record<string, () => Promise<Boot>>> = {
  holo: () => import("./holo/index.ts").then((m) => m.bootHolo),
  downloads: () => import("./downloads.ts").then((m) => (root: Document) => {
    m.wireDownloads(root, navigator);
  }),
  foil: () => import("./islands/foil.ts").then((m) => m.bootFoil),
  steps: () => import("./islands/steps.ts").then((m) => m.bootSteps),
  print: () => import("./islands/print.ts").then((m) => m.bootPrint),
  selfcheck: () => import("./islands/selfcheck.ts").then((m) => m.bootSelfCheck),
};

function startIslands(): void {
  const wanted = new Set([...document.querySelectorAll<HTMLElement>("[data-island]")].map((el) => el.dataset["island"] ?? ""));
  for (const name of wanted) {
    const load = ISLANDS[name];
    if (load !== undefined) {
      void load().then((boot) => {
        boot(document);
      });
    }
  }
}

// After first paint: two frames, so the images and the headline are on screen before any work.
requestAnimationFrame(() => {
  requestAnimationFrame(startIslands);
});
