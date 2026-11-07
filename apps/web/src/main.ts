/**
 * Client entry. HTML and CSS paint the whole page on their own, the painting included. After
 * first paint the islands start: the hero's pixel sort, the holo foil buttons, the download
 * section's system detection and the menu. No React and no ICU code ship to the browser.
 */
import "virtual:mosaic-tokens.css";
import "./styles/index.css";
import { wireDownloads } from "./downloads.ts";
import { wireMenu } from "./menu.ts";

wireMenu(document);
wireDownloads(document, navigator);

function startIslands(): void {
  void import("./holo/index.ts").then(({ bootHolo }) => {
    bootHolo(document);
  });
  void import("./hero/index.ts").then(async ({ bootHero }) => {
    const hero = await bootHero(document);
    // Dev only: the tuning panel and the still baker. Statically false in builds, so neither
    // they nor their imports reach the production bundle.
    if (import.meta.env.DEV) {
      const query = new URLSearchParams(location.search);
      if (hero !== null && query.get("tune") === "1") {
        void import("./tune/panel.ts").then(({ mountTune }) => {
          mountTune(hero);
        });
      }
      if (query.get("bake") === "1") {
        void import("./hero/bake.ts").then(({ bake }) => {
          (window as unknown as { mosaicBake?: typeof bake }).mosaicBake = bake;
        });
      }
    }
  });
}

// After first paint: two frames, so the painting and the headline are on screen before GL work.
requestAnimationFrame(() => {
  requestAnimationFrame(startIslands);
});
