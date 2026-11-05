/**
 * Would the user see this field? A page can hide a field from the user and still fill it
 * (opacity 0, off screen, 1 px, clipped, covered by something else), so we only fill what a
 * person could see and click. `env` is the only part that touches layout; tests replace it.
 */

export interface Box {
  left: number;
  top: number;
  right: number;
  bottom: number;
  width: number;
  height: number;
}

export interface ViewportInfo {
  width: number;
  height: number;
  scrollX: number;
  scrollY: number;
  /** Size of the whole document, in CSS pixels. */
  docWidth: number;
  docHeight: number;
}

export interface VisibilityEnv {
  /** CSS visibility: display, visibility, content-visibility, own opacity. */
  checkVisibility(el: Element): boolean;
  box(el: Element): Box;
  /** The computed opacity of this element alone. */
  opacity(el: Element): number;
  /** The parent, crossing out of a shadow root into its host. */
  parent(el: Element): Element | null;
  viewport(): ViewportInfo;
  /** The topmost element at a viewport point, in the tree of `el` (document or shadow root). */
  elementAt(el: Element, x: number, y: number): Element | null;
}

/** Smallest width and height of a field the user can see. */
export const MIN_SIZE = 4;
/** Least combined opacity of a field and its ancestors. */
export const MIN_OPACITY = 0.2;
const MAX_ANCESTORS = 512;

export const domEnv: VisibilityEnv = {
  checkVisibility: (el) =>
    el.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true, contentVisibilityAuto: true }),
  box: (el) => el.getBoundingClientRect(),
  opacity: (el) => {
    const value = Number.parseFloat(getComputedStyle(el).opacity);
    return Number.isNaN(value) ? 1 : value;
  },
  parent: (el) => {
    if (el.parentElement !== null) {
      return el.parentElement;
    }
    const root = el.getRootNode();
    return root instanceof ShadowRoot ? root.host : null;
  },
  viewport: () => ({
    width: window.innerWidth,
    height: window.innerHeight,
    scrollX: window.scrollX,
    scrollY: window.scrollY,
    docWidth: document.documentElement.scrollWidth,
    docHeight: document.documentElement.scrollHeight,
  }),
  elementAt: (el, x, y) => {
    const root = el.getRootNode();
    return root instanceof Document || root instanceof ShadowRoot ? root.elementFromPoint(x, y) : null;
  },
};

/** The product of the opacities of `el` and all its ancestors (it stops early once below the limit). */
export function opacityProduct(el: Element, env: VisibilityEnv): number {
  let product = 1;
  let node: Element | null = el;
  for (let steps = 0; node !== null && steps < MAX_ANCESTORS; steps++) {
    product *= env.opacity(node);
    if (product < MIN_OPACITY) {
      return product;
    }
    node = env.parent(node);
  }
  return product;
}

/** Connected, displayed, big enough and not (nearly) transparent. Used for fields and frame elements. */
export function isElementVisible(el: Element, env: VisibilityEnv = domEnv): boolean {
  if (!el.isConnected || !env.checkVisibility(el)) {
    return false;
  }
  const box = env.box(el);
  return box.width >= MIN_SIZE && box.height >= MIN_SIZE && opacityProduct(el, env) >= MIN_OPACITY;
}

function intersectViewport(box: Box, view: ViewportInfo): Box | null {
  const left = Math.max(box.left, 0);
  const top = Math.max(box.top, 0);
  const right = Math.min(box.right, view.width);
  const bottom = Math.min(box.bottom, view.height);
  if (right <= left || bottom <= top) {
    return null;
  }
  return { left, top, right, bottom, width: right - left, height: bottom - top };
}

/** The centre and four points inset by a quarter of the width and height. */
function samplePoints(box: Box): [number, number][] {
  const dx = box.width / 4;
  const dy = box.height / 4;
  const cx = box.left + box.width / 2;
  const cy = box.top + box.height / 2;
  return [
    [cx, cy],
    [box.left + dx, box.top + dy],
    [box.right - dx, box.top + dy],
    [box.left + dx, box.bottom - dy],
    [box.right - dx, box.bottom - dy],
  ];
}

function isTarget(hit: Element | null, el: Element): boolean {
  if (hit === null) {
    return false;
  }
  return hit === el || el.contains(hit) || (hit instanceof HTMLLabelElement && hit.control === el);
}

/**
 * Whether the user could see and click `el`: displayed, at least 4 x 4 px, combined opacity at
 * least 0.2, inside the document, and (when on screen) on top at one of five sample points.
 */
export function isFillable(el: Element, env: VisibilityEnv = domEnv): boolean {
  if (!isElementVisible(el, env)) {
    return false;
  }
  const box = env.box(el);
  const view = env.viewport();
  if (box.right + view.scrollX <= 0 || box.bottom + view.scrollY <= 0) {
    return false;
  }
  if (box.left + view.scrollX >= view.docWidth || box.top + view.scrollY >= view.docHeight) {
    return false;
  }
  const shown = intersectViewport(box, view);
  if (shown === null) {
    return true;
  }
  return samplePoints(shown).some(([x, y]) => isTarget(env.elementAt(el, x, y), el));
}
