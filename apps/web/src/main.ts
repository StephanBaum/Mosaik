/**
 * Client entry (design concept §4 page load). HTML and CSS paint the header, the headline and
 * the trust strip on their own; after first paint the hero island starts WebGL and the canvas
 * fades in. No React and no ICU code ship to the browser.
 */
import "virtual:mosaic-tokens.css";
import "./styles/index.css";
import { wireMenu } from "./menu.ts";

wireMenu(document);

function startHero(): void {
  void import("./hero/index.ts").then(({ bootHero }) => {
    const hero = bootHero(document);
    // Dev only: the tuning panel for checkpoint A. Statically false in builds, so the panel
    // and its import are dropped from the production bundle.
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
