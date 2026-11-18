/** Canvas pixel ceiling of the field (website plan, runtime budgets). */
export const MAX_PIXELS = 2_400_000;

/**
 * Backing-store size for a CSS size: the device pixel ratio capped at `cap`, lowered further
 * until the canvas holds at most `maxPixels` pixels (a 4K screen would otherwise paint 8 M).
 */
export function canvasSize(cssW: number, cssH: number, dpr: number, cap: number): { w: number; h: number; scale: number } {
  const maxPixels = MAX_PIXELS;
  let scale = Math.min(dpr, cap);
  const area = Math.max(1, cssW * cssH);
  if (area * scale * scale > maxPixels) {
    scale = Math.sqrt(maxPixels / area);
  }
  let w = Math.max(2, Math.round(cssW * scale));
  let h = Math.max(2, Math.round(cssH * scale));
  if (w * h > maxPixels && w > 2 && h > 2) {
    w = Math.max(2, Math.floor(cssW * scale));
    h = Math.max(2, Math.floor(cssH * scale));
  }
  return { w, h, scale };
}
