/**
 * The sealed snapshot (ADR-0032): the live vault painted into a picture, secrets excluded
 * by `collect.ts`, so the vault can be unmounted while the closing animation plays over the
 * picture. Prepared synchronously inside the click that locks.
 */
import type { Band } from "./band.ts";
import { lockSchedule, REST_POSE, type Schedule } from "./choreography.ts";
import type { RunMode } from "./driver.ts";
import type { RevealEnv } from "./env.ts";
import { buildDisplay, compose, releaseDisplay, type Display } from "./paint.ts";
import { sceneFor, type Point, type Scene } from "./scene.ts";
import { BAND_PIXEL_BUDGET, SEALED_PIXEL_BUDGET } from "./timing.ts";

export interface SealedDisplay {
  display: Display;
  /** The picture that is shown (`display.cv`, kept here after the display is released). */
  cv: HTMLCanvasElement;
  /** The picture at the band's scale, refilled before each upload; glass mode only. */
  bandSrc: HTMLCanvasElement | null;
  scene: Scene;
  mode: RunMode;
  schedule: Schedule;
}

function modeFor(reduced: boolean, band: Band | null, source: { width: number; height: number }): RunMode {
  if (reduced) {
    return "fade";
  }
  const fits = band !== null && source.width <= band.maxTexture && source.height <= band.maxTexture;
  return band !== null && !band.lost && fits ? "glass" : "lens";
}

export function prepareSeal(args: {
  vault: HTMLElement;
  stage: HTMLElement;
  origin: Point;
  env: RevealEnv;
  band: Band | null;
}): SealedDisplay | null {
  const { vault, stage, origin, env, band } = args;
  const reduced = env.deps.reduced();
  const size = stage.getBoundingClientRect();
  const dpr = env.deps.dpr();
  const scene = sceneFor(size, origin, dpr, SEALED_PIXEL_BUDGET);
  const display = buildDisplay(vault, stage, scene.S, { groups: !reduced, paint: true });
  if (display === null) {
    return null;
  }
  const cv = display.cv;
  if (cv === null) {
    releaseDisplay(display);
    return null;
  }
  compose(
    display,
    display.groups.map(() => REST_POSE),
  );
  const bandScale = sceneFor(size, origin, dpr, BAND_PIXEL_BUDGET).S;
  const source = { width: Math.max(1, Math.round(size.width * bandScale)), height: Math.max(1, Math.round(size.height * bandScale)) };
  const mode = modeFor(reduced, band, source);
  const bandSrc =
    mode === "glass" ? Object.assign(document.createElement("canvas"), { width: source.width, height: source.height }) : null;
  return { display, cv, bandSrc, scene, mode, schedule: lockSchedule(display.groups, scene, reduced) };
}

/** Frees the snapshot's canvases. */
export function releaseSealed(sealed: SealedDisplay): void {
  releaseDisplay(sealed.display);
  sealed.cv.width = 0;
  sealed.cv.height = 0;
  if (sealed.bandSrc !== null) {
    sealed.bandSrc.width = 0;
    sealed.bandSrc.height = 0;
  }
}
