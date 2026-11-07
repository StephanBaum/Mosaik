/**
 * The hero island (design concept §3 M1 and §4 page load). Starts after first paint: one
 * shared WebGL2 stage paints the living mosaic, the holo CTA and the logo's foil tile.
 * Without WebGL2 (or after a lost context) the flat tiles sweep in instead; under reduced
 * motion or Save-Data the mosaic is one designed still. Rendering runs only while the hero is
 * on screen and the tab is visible.
 */
import { createStage, type Stage } from "../fx/stage.ts";
import { HoloButtonTarget, HoloSurfaceTarget } from "./holo.ts";
import { MosaicTarget } from "./mosaic.ts";
import { attachPointer, introEnd } from "./motion.ts";
import { defaultParams, type MosaicParams } from "./params.ts";

export interface HeroController {
  readonly params: MosaicParams;
  readonly mosaic: MosaicTarget | null;
  /** Repaints after a parameter change. */
  refresh(): void;
  /** Replays the load sweep. */
  replay(): void;
}

const LANDED = "is-landed";

function prefersStill(): boolean {
  const saveData = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection?.saveData === true;
  return saveData || matchMedia("(prefers-reduced-motion: reduce)").matches;
}

const darkQuery = (): MediaQueryList => matchMedia("(prefers-color-scheme: dark)");

/** Without WebGL: the flat tiles sweep in diagonally from the logo corner, set through the CSSOM. */
function flatFallback(hero: HTMLElement, box: HTMLElement): void {
  const tiles = box.querySelectorAll<HTMLElement>(".tile");
  const first = tiles[0];
  const pitch = first === undefined ? 1 : first.offsetWidth + 8;
  for (const tile of tiles) {
    const diagonal = Math.round(tile.offsetLeft / pitch + tile.offsetTop / pitch);
    tile.style.animationDelay = `${String(Math.min(diagonal * 18, 500))}ms`;
  }
  box.classList.add("mosaic--flat");
  hero.classList.add(LANDED);
}

function observeVisibility(el: HTMLElement, onChange: (visible: boolean) => void): void {
  new IntersectionObserver((entries) => {
    for (const entry of entries) {
      onChange(entry.isIntersecting);
    }
  }).observe(el);
}

interface Parts {
  hero: HTMLElement;
  box: HTMLElement;
  canvas: HTMLCanvasElement;
}

function startMosaic(stage: Stage, parts: Parts, params: MosaicParams): MosaicTarget {
  const { hero, box, canvas } = parts;
  const still = prefersStill();
  const dark = darkQuery();
  const mosaic = new MosaicTarget({
    box,
    canvas,
    params,
    still,
    dark: () => dark.matches,
    onLive: () => {
      box.classList.add("mosaic--live");
      window.setTimeout(() => {
        hero.classList.add(LANDED);
      }, still ? 0 : introEnd(params) * 1000);
    },
    onLost: () => {
      box.classList.remove("mosaic--live");
      flatFallback(hero, box);
    },
  });
  const poke = (): void => {
    mosaic.invalidate();
    stage.kick();
  };
  new ResizeObserver(() => {
    mosaic.relayout();
    stage.kick();
  }).observe(box);
  dark.addEventListener("change", poke);
  observeVisibility(hero, (visible) => {
    mosaic.setVisible(visible);
    if (visible) {
      poke();
    }
  });
  if (!still) {
    attachPointer(hero, mosaic.motion, {
      geometry: () => ({
        rect: box.getBoundingClientRect(),
        pitch: mosaic.layout.pitch,
        inset: (mosaic.layout.pitch - mosaic.layout.tile) / 2,
      }),
      poke,
      now: () => performance.now(),
    });
  }
  stage.add(mosaic);
  return mosaic;
}

function startHolo(stage: Stage, root: Document, heroVisible: () => boolean): void {
  const dark = darkQuery();
  const reduced = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const cta = root.querySelector<HTMLElement>(".hero .holo");
  const foil = cta?.querySelector<HTMLCanvasElement>(".holo__foil");
  if (cta && foil) {
    const target = new HoloButtonTarget(cta, foil, {
      reduced,
      dark: () => dark.matches,
      visible: heroVisible,
      poke: () => {
        stage.kick();
      },
    });
    dark.addEventListener("change", () => {
      target.invalidate();
      stage.kick();
    });
    stage.add(target);
  }
  const logo = root.querySelector<HTMLCanvasElement>(".logo-mark__foil");
  if (logo) {
    const surface = new HoloSurfaceTarget(logo, () => dark.matches);
    dark.addEventListener("change", () => {
      surface.invalidate();
      stage.kick();
    });
    stage.add(surface);
  }
}

export function bootHero(root: Document): HeroController | null {
  const hero = root.querySelector<HTMLElement>(".hero");
  const box = hero?.querySelector<HTMLElement>(".mosaic");
  const canvas = box?.querySelector<HTMLCanvasElement>(".mosaic__canvas");
  if (!hero || !box || !canvas) {
    return null;
  }
  const params = defaultParams();
  const stage = createStage();
  if (stage === null) {
    flatFallback(hero, box);
    return { params, mosaic: null, refresh: () => undefined, replay: () => undefined };
  }
  const mosaic = startMosaic(stage, { hero, box, canvas }, params);
  startHolo(stage, root, () => mosaic.visible());
  return {
    params,
    mosaic,
    refresh: () => {
      mosaic.invalidate();
      stage.kick();
    },
    replay: () => {
      mosaic.motion.intro = 0;
      mosaic.motion.lastInput = performance.now();
      mosaic.invalidate();
      stage.kick();
    },
  };
}
