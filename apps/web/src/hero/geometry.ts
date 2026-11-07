/**
 * Pure geometry of the painted hero: how the canvas maps onto the CSS `object-fit: cover`
 * painting, the pixel budget of the canvas and the inertia of the cursor. No DOM, no GL, so
 * the unit tests pin it.
 */

export interface Size {
  width: number;
  height: number;
}

/** `[scaleU, scaleV, offsetU, offsetV]`: canvas uv (0..1, y down) to image uv. */
export type Cover = readonly [number, number, number, number];

/**
 * The image uv transform of `object-fit: cover` with `object-position` `(px, py)` (shares,
 * 0..1), so the shader samples exactly the pixels the `<img>` shows.
 */
export function coverMap(view: Size, image: Size, px: number, py: number): Cover {
  if (view.width <= 0 || view.height <= 0 || image.width <= 0 || image.height <= 0) {
    return [1, 1, 0, 0];
  }
  const scale = Math.max(view.width / image.width, view.height / image.height);
  const shownW = image.width * scale;
  const shownH = image.height * scale;
  const offsetX = (view.width - shownW) * px;
  const offsetY = (view.height - shownH) * py;
  return [view.width / shownW, view.height / shownH, -offsetX / shownW, -offsetY / shownH];
}

/** One `object-position` component (`"62%"`, `"left"`, `"center"`) as a share; `0.5` otherwise. */
function positionShare(token: string | undefined): number {
  if (token === undefined) {
    return 0.5;
  }
  const keywords: Record<string, number> = { left: 0, top: 0, center: 0.5, right: 1, bottom: 1 };
  const keyword = keywords[token];
  if (keyword !== undefined) {
    return keyword;
  }
  const match = /^(-?\d+(?:\.\d+)?)%$/.exec(token);
  return match?.[1] === undefined ? 0.5 : Number(match[1]) / 100;
}

/** A computed `object-position` (`"62% 50%"`) as shares. Pixel lengths count as centred. */
export function parseObjectPosition(value: string): [number, number] {
  const [x, y] = value.trim().split(/\s+/);
  return [positionShare(x), positionShare(y)];
}

/**
 * Canvas resolution scale: the device pixel ratio times the adaptive `quality`, held under a
 * pixel budget. The sorted runs are soft along their length, so a lower resolution barely shows;
 * the painting itself is always the crisp `<img>` underneath.
 */
export function canvasScale(view: Size, dpr: number, quality: number, budget: number): number {
  const area = Math.max(view.width * view.height, 1);
  const wanted = Math.min(Math.max(dpr, 1), 2) * quality;
  return Math.max(0.25, Math.min(wanted, Math.sqrt(budget / area)));
}

/** Frame-rate independent approach of `current` to `target` at `rate` per second. */
export function follow(current: number, target: number, rate: number, dt: number): number {
  return target + (current - target) * Math.exp(-Math.max(rate, 0) * Math.max(dt, 0));
}

/** How far the hero has scrolled out: 0 while its top is at the viewport top, 1 once it is gone. */
export function scrollProgress(heroTop: number, heroHeight: number): number {
  if (heroHeight <= 0) {
    return 0;
  }
  return Math.min(Math.max(-heroTop / heroHeight, 0), 1);
}

/** A child rectangle as uv (0..1) of its parent: `[x0, y0, x1, y1]`. */
export function rectUv(
  child: { left: number; top: number; right: number; bottom: number },
  parent: { left: number; top: number; width: number; height: number },
): [number, number, number, number] {
  const w = Math.max(parent.width, 1);
  const h = Math.max(parent.height, 1);
  return [
    (child.left - parent.left) / w,
    (child.top - parent.top) / h,
    (child.right - parent.left) / w,
    (child.bottom - parent.top) / h,
  ];
}
