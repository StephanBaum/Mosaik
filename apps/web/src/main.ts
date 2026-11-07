/**
 * Client entry. HTML and CSS paint the page on their own; the headline shows as pixel blocks
 * at once. After first paint the hero island starts WebGL, the headline decodes and the art
 * fades in over its CSS still.
 * No React and no ICU code ship to the browser.
 */
import "virtual:mosaic-tokens.css";
import "./styles/index.css";
import { wireMenu } from "./menu.ts";
import { wireType } from "./type.ts";

wireMenu(document);
const startHeadline = wireType(document);

function startHero(): void {
  void import("./hero/index.ts").then(async ({ bootHero }) => {
    // bootHero creates the GL context synchronously (it can stall the main thread for a
    // moment); the headline decode starts right after, so it never freezes mid-scramble.
    const booting = bootHero(document);
    startHeadline();
    const hero = await booting;
    // Dev only: the tuning panel. Statically false in builds, so the panel and its import
    // are dropped from the production bundle.
    if (import.meta.env.DEV && hero !== null && new URLSearchParams(location.search).get("tune") === "1") {
      void import("./tune/panel.ts").then(({ mountTune }) => {
        mountTune(hero);
      });
    }
  });
}

// After first paint: two frames, so the headline's first frame is on screen before GL work.
requestAnimationFrame(() => {
  requestAnimationFrame(startHero);
});
