/**
 * Where a reveal grows from or closes into. The last pointer press on the Unlock (or
 * quick-unlock) control, or on the Lock button, is the origin; a keyboard submit, a shortcut
 * or the palette falls back to the centre of the control, then of the stage.
 */
import type { Point } from "./scene.ts";

export type RevealLayerName = "unlock" | "vault";

export interface PointerMark {
  /** Stage px. */
  x: number;
  y: number;
  /** When it was pressed (the reveal clock). */
  at: number;
  layer: RevealLayerName;
  /** The press was on the Lock button. */
  lockButton: boolean;
}

/** A quick-unlock prompt can keep the user waiting, so an unlock press stays valid this long. */
export const UNLOCK_ORIGIN_MAX_AGE_MS = 30_000;
export const LOCK_ORIGIN_MAX_AGE_MS = 2_000;

const LOCK_ORIGIN = '[data-reveal-origin="lock"]';
const UNLOCK_ORIGIN = '[data-reveal-origin="unlock"]';

/** A mark for a press that can start a reveal, or null for any other press. */
export function markFromPointer(
  e: { clientX: number; clientY: number; target: EventTarget | null },
  stage: Element,
  now: number,
): PointerMark | null {
  if (!(e.target instanceof Element)) {
    return null;
  }
  const layer = e.target.closest("[data-reveal-layer]")?.getAttribute("data-reveal-layer");
  const rect = stage.getBoundingClientRect();
  const at = { x: e.clientX - rect.left, y: e.clientY - rect.top, at: now };
  if (layer === "unlock" && e.target.closest("button") !== null) {
    return { ...at, layer, lockButton: false };
  }
  if (layer === "vault" && e.target.closest(LOCK_ORIGIN) !== null) {
    return { ...at, layer, lockButton: true };
  }
  return null;
}

function centreOf(layer: Element, selector: string, stageRect: DOMRect): Point | null {
  const rect = layer.querySelector(selector)?.getBoundingClientRect();
  if (rect === undefined || rect.width <= 0 || rect.height <= 0) {
    return null;
  }
  return [rect.left + rect.width / 2 - stageRect.left, rect.top + rect.height / 2 - stageRect.top];
}

export function resolveOrigin(args: {
  dir: "unlock" | "lock";
  mark: PointerMark | null;
  layer: Element;
  stageRect: DOMRect;
  now: number;
}): Point {
  const { dir, mark, layer, stageRect, now } = args;
  const fresh =
    mark !== null &&
    (dir === "unlock"
      ? mark.layer === "unlock" && now - mark.at <= UNLOCK_ORIGIN_MAX_AGE_MS
      : mark.lockButton && now - mark.at <= LOCK_ORIGIN_MAX_AGE_MS);
  if (fresh) {
    return [mark.x, mark.y];
  }
  return (
    centreOf(layer, dir === "unlock" ? UNLOCK_ORIGIN : LOCK_ORIGIN, stageRect) ?? [
      stageRect.width / 2,
      stageRect.height / 2,
    ]
  );
}
