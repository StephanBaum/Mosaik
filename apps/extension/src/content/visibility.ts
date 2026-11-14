/**
 * Would the user see this field? A page can hide a field from the user and still fill it
 * (opacity 0, off screen, 1 px, clipped, covered by something else), so we only fill what a
 * person could see and click. `env` is the only part that touches layout; tests replace it.
 *
 * The same checks run on the `<iframe>` elements above a frame, from inside the frame, so
 * nothing here may depend on the realm of an element (`instanceof` fails across frames).
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
  /** The document is a frame: its viewport is all the user sees of it, so fields must be on screen. */
  framed?: boolean;
}

/** How an element clips what it contains. */
export interface Clip {
  /** `overflow` hidden or clip on an axis: nothing outside its box is shown or can be scrolled to. */
  overflow: boolean;
  /** A `clip-path`, or a `clip` rectangle on a positioned element. */
  shape: boolean;
}

export interface VisibilityEnv {
  /** CSS visibility: display, visibility, content-visibility, own opacity. */
  checkVisibility(el: Element): boolean;
  box(el: Element): Box;
  /** This element alone: `opacity` times every `filter: opacity()`, and 0 under a `mask-image`. */
  opacity(el: Element): number;
  /** The parent, crossing out of a shadow root into its host. */
  parent(el: Element): Element | null;
  /** The viewport and size of the document `el` lives in. */
  viewport(el: Element): ViewportInfo;
  /** The topmost element at a viewport point, in the tree of `el` (document or shadow root). */
  elementAt(el: Element, x: number, y: number): Element | null;
  clip(el: Element): Clip;
}

/** Smallest width and height of a field the user can see. */
export const MIN_SIZE = 4;
/** Least combined opacity of a field and its ancestors. */
export const MIN_OPACITY = 0.2;
const MAX_ANCESTORS = 512;

const isShadowRoot = (node: Node): node is ShadowRoot =>
  node.nodeType === Node.DOCUMENT_FRAGMENT_NODE && "host" in node;

const styleOf = (el: Element): CSSStyleDeclaration => (el.ownerDocument.defaultView ?? window).getComputedStyle(el);

const FILTER_OPACITY = /opacity\(\s*([\d.]+)(%?)\s*\)/g;
const isNone = (value: string): boolean => value === "" || value === "none";

/**
 * `opacity` times every `opacity()` of `filter` (`checkVisibility` sees neither). A mask is not
 * measured: any `mask-image` counts as hiding the element (the conservative choice).
 */
export function ownOpacity(style: Pick<CSSStyleDeclaration, "opacity" | "filter" | "getPropertyValue">): number {
  if (!isNone(style.getPropertyValue("mask-image")) || !isNone(style.getPropertyValue("-webkit-mask-image"))) {
    return 0;
  }
  const own = Number.parseFloat(style.opacity);
  let value = Number.isNaN(own) ? 1 : own;
  for (const [, amount = "1", percent] of style.filter.matchAll(FILTER_OPACITY)) {
    const factor = Number.parseFloat(amount) / (percent === "%" ? 100 : 1);
    value *= Number.isNaN(factor) ? 1 : Math.min(factor, 1);
  }
  return value;
}

const clipsOverflow = (value: string): boolean => value === "hidden" || value === "clip";

export const domEnv: VisibilityEnv = {
  checkVisibility: (el) =>
    el.checkVisibility({ checkOpacity: true, checkVisibilityCSS: true, contentVisibilityAuto: true }),
  box: (el) => el.getBoundingClientRect(),
  opacity: (el) => ownOpacity(styleOf(el)),
  parent: (el) => {
    if (el.parentElement !== null) {
      return el.parentElement;
    }
    const root = el.getRootNode();
    return isShadowRoot(root) ? root.host : null;
  },
  viewport: (el) => {
    const doc = el.ownerDocument;
    const win = doc.defaultView ?? window;
    return {
      width: win.innerWidth,
      height: win.innerHeight,
      scrollX: win.scrollX,
      scrollY: win.scrollY,
      docWidth: doc.documentElement.scrollWidth,
      docHeight: doc.documentElement.scrollHeight,
      framed: win !== win.top,
    };
  },
  elementAt: (el, x, y) => {
    const root = el.getRootNode();
    return root.nodeType === Node.DOCUMENT_NODE || isShadowRoot(root)
      ? (root as Document | ShadowRoot).elementFromPoint(x, y)
      : null;
  },
  clip: (el) => {
    const style = styleOf(el);
    const positioned = style.position === "absolute" || style.position === "fixed";
    return {
      overflow: clipsOverflow(style.overflowX) || clipsOverflow(style.overflowY),
      shape: !isNone(style.clipPath) || (positioned && !["", "auto"].includes(style.getPropertyValue("clip"))),
    };
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

/** A random number in [0, 1) from the platform's CSPRNG: the page can neither predict nor bias it. */
export function secureRandom(): number {
  const word = new Uint32Array(1);
  crypto.getRandomValues(word);
  return (word[0] ?? 0) / 2 ** 32;
}

/** Points drawn per check, besides the centre. */
const RANDOM_POINTS = 4;

/**
 * The centre, then four points drawn with `random` inside the box with a small margin, so a
 * page cannot know where the test looks. The box is never empty (`intersectViewport`), so every
 * point lies inside it.
 */
function samplePoints(box: Box, random: () => number): [number, number][] {
  const mx = Math.min(2, box.width / 4);
  const my = Math.min(2, box.height / 4);
  const points: [number, number][] = [[box.left + box.width / 2, box.top + box.height / 2]];
  for (let i = 0; i < RANDOM_POINTS; i++) {
    points.push([box.left + mx + random() * (box.width - 2 * mx), box.top + my + random() * (box.height - 2 * my)]);
  }
  return points;
}

/** Only the field itself or something inside it counts as on top: a label or any other element does not. */
function isTarget(hit: Element | null, el: Element): boolean {
  return hit !== null && (hit === el || el.contains(hit));
}

function intersect(a: Box, b: Box): Box | null {
  const left = Math.max(a.left, b.left);
  const top = Math.max(a.top, b.top);
  const right = Math.min(a.right, b.right);
  const bottom = Math.min(a.bottom, b.bottom);
  if (right - left < MIN_SIZE || bottom - top < MIN_SIZE) {
    return null;
  }
  return { left, top, right, bottom, width: right - left, height: bottom - top };
}

/** The root element and the body: their overflow is the viewport's, not a clip of their own. */
function isViewportScroller(el: Element): boolean {
  const doc = el.ownerDocument;
  return el.parentNode === doc || (el.localName === "body" && el.parentElement?.parentNode === doc);
}

/**
 * Whether `el` (with box `box`) is clipped away: it or an ancestor has a clip shape, or an
 * ancestor's `overflow` box leaves less than `MIN_SIZE` of it. For where no hit test can tell
 * (off screen), so a `clip-path` there counts as hiding, whatever its shape.
 */
export function isClippedAway(el: Element, box: Box, env: VisibilityEnv): boolean {
  let visible: Box | null = box;
  let node: Element | null = el;
  for (let steps = 0; node !== null && steps < MAX_ANCESTORS; steps++) {
    if (!isViewportScroller(node)) {
      const clip = env.clip(node);
      if (clip.shape) {
        return true;
      }
      if (clip.overflow && node !== el) {
        visible = intersect(visible, env.box(node));
        if (visible === null) {
          return true;
        }
      }
    }
    node = env.parent(node);
  }
  return false;
}

/**
 * Whether the user could see and click `el`: displayed, at least 4 x 4 px, combined opacity at
 * least 0.2, inside the document, and on top at the centre and at a majority of five sample
 * points (the centre and four drawn per check with `random`). Off screen there is
 * no hit test, so there it must be in the top document (in a frame the viewport is all the user
 * sees) and not clipped away by an ancestor.
 */
export function isFillable(el: Element, env: VisibilityEnv = domEnv, random: () => number = secureRandom): boolean {
  if (!isElementVisible(el, env)) {
    return false;
  }
  const box = env.box(el);
  const view = env.viewport(el);
  if (box.right + view.scrollX <= 0 || box.bottom + view.scrollY <= 0) {
    return false;
  }
  if (box.left + view.scrollX >= view.docWidth || box.top + view.scrollY >= view.docHeight) {
    return false;
  }
  const onScreen = intersectViewport(box, view);
  if (onScreen === null) {
    return view.framed !== true && !isClippedAway(el, box, env);
  }
  let points: [number, number][];
  try {
    points = samplePoints(onScreen, random);
  } catch {
    // No randomness, no trust: the fixed centre alone would be what the page can aim at.
    return false;
  }
  const hits = points.map(([x, y]) => isTarget(env.elementAt(el, x, y), el));
  return hits[0] === true && hits.filter(Boolean).length > points.length / 2;
}
