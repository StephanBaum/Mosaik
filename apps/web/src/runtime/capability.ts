/**
 * Capability tiers: what the page may spend on effects. `none` shows the CSS still only,
 * `lite` runs the field at DPR 1 and 30 fps without the data layer and veil wipe, `full` runs
 * everything. Decided from facts the browser reports, strictest tier first.
 */
import { isMotionOff } from "./motion.ts";

export type FxTier = "full" | "lite" | "none";

export interface CapabilityEnv {
  reducedMotion: boolean;
  motionOff: boolean;
  saveData: boolean;
  effectiveType: string | undefined;
  cores: number | undefined;
  memoryGb: number | undefined;
  coarsePointer: boolean;
  viewportWidth: number;
  webgl2: boolean;
}

const SLOW_LINKS = ["slow-2g", "2g"];

/** The reasons to show the CSS still only. */
function stillOnly(env: CapabilityEnv, cores: number, memory: number): boolean {
  const off = env.reducedMotion || env.motionOff || env.saveData;
  const weak = !env.webgl2 || cores <= 2 || memory < 2;
  return off || weak || SLOW_LINKS.includes(env.effectiveType ?? "");
}

export function fxTier(env: CapabilityEnv): FxTier {
  const cores = env.cores ?? 8;
  const memory = env.memoryGb ?? 8;
  if (stillOnly(env, cores, memory)) {
    return "none";
  }
  const modest = env.coarsePointer || env.viewportWidth < 700 || env.effectiveType === "3g";
  return modest || cores <= 4 || memory <= 4 ? "lite" : "full";
}

interface NavigatorHints {
  connection?: { saveData?: boolean; effectiveType?: string };
  deviceMemory?: number;
}

/** A WebGL2 context that is not a software fallback; released at once. */
function hasWebgl2(win: Window): boolean {
  try {
    const canvas = win.document.createElement("canvas");
    const gl = canvas.getContext("webgl2", { failIfMajorPerformanceCaveat: true });
    if (gl === null) {
      return false;
    }
    gl.getExtension("WEBGL_lose_context")?.loseContext();
    return true;
  } catch {
    return false;
  }
}

export function readEnv(win: Window, storage: Pick<Storage, "getItem"> | null): CapabilityEnv {
  const nav = win.navigator as Navigator & NavigatorHints;
  return {
    reducedMotion: win.matchMedia("(prefers-reduced-motion: reduce)").matches,
    motionOff: isMotionOff(storage),
    saveData: nav.connection?.saveData === true,
    effectiveType: nav.connection?.effectiveType,
    cores: nav.hardwareConcurrency,
    memoryGb: nav.deviceMemory,
    coarsePointer: win.matchMedia("(pointer: coarse)").matches,
    viewportWidth: win.innerWidth,
    webgl2: hasWebgl2(win),
  };
}
