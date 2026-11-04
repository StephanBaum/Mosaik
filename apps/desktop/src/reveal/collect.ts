/**
 * DOM to display list for the snapshot painter (study L415-470, with the exclusion rules of
 * ADR-0032). The snapshot is a picture of the vault, so it must never contain a secret:
 *
 * - `[data-secret]` elements paint their box and the static `data-mask` text only. Their
 *   content is never read and their subtree never visited.
 * - Inputs and textareas paint their box and, when empty, the placeholder. A value is never
 *   read into the picture.
 * - Dialogs, media and visually hidden text are skipped.
 */
import { GROUP_SELECTOR, ORIGIN_SELECTOR, POP_SELECTOR } from "./choreography.ts";
import { MASK_ATTR, SECRET_ATTR, type Collected, type Measure, type Op, type Rect, type SvgItem } from "./ops.ts";
import { collectText, fontOf } from "./textOps.ts";

export { MASK_ATTR, SECRET_ATTR } from "./ops.ts";

const SKIP_TAGS: ReadonlySet<string> = new Set([
  "DIALOG",
  "SCRIPT",
  "STYLE",
  "TEMPLATE",
  "IFRAME",
  "VIDEO",
  "AUDIO",
  "OBJECT",
  "EMBED",
]);

interface Walk {
  measure: Measure;
  groups: boolean;
  out: Collected;
  /** Padding boxes of the overflow clips around the current element, outermost first. */
  clips: Rect[];
  /** The part of the stage still visible inside those clips: the stage first, one entry per clip after. */
  visible: Rect[];
}

/** One element on its way through the walk. */
interface Item {
  el: Element;
  cs: CSSStyleDeclaration;
  r: Rect;
  /** Opacity of the element and everything above it. */
  a: number;
  /** Index of the group the element paints into, or -1 for the base. */
  g: number;
  root: boolean;
}

const px = (value: string): number => Number.parseFloat(value) || 0;

const isClear = (c: string): boolean => c === "" || c === "transparent" || /rgba\([^)]*,\s*0\)$/.test(c);

/** A border radius in px; a percentage is a share of the shorter side. */
function radiusOf(cs: CSSStyleDeclaration, r: Rect): number {
  const raw = cs.borderTopLeftRadius;
  const value = px(raw);
  const size = raw.endsWith("%") ? (value / 100) * Math.min(r.w, r.h) : value;
  return Math.min(size, r.w / 2, r.h / 2);
}

function intersect(a: Rect, b: Rect): Rect {
  const x = Math.max(a.x, b.x);
  const y = Math.max(a.y, b.y);
  return {
    x,
    y,
    w: Math.max(0, Math.min(a.x + a.w, b.x + b.w) - x),
    h: Math.max(0, Math.min(a.y + a.h, b.y + b.h) - y),
  };
}

function outside(r: Rect, v: Rect): boolean {
  return r.x >= v.x + v.w || r.x + r.w <= v.x || r.y >= v.y + v.h || r.y + r.h <= v.y;
}

function hiddenByStyle(cs: CSSStyleDeclaration): boolean {
  return cs.display === "none" || cs.visibility === "hidden" || cs.opacity === "0";
}

/** The `visually-hidden` pattern: a clip-path or clip, or a 1 px box that clips its overflow. */
function clippedAway(cs: CSSStyleDeclaration): boolean {
  const clipPath = cs.clipPath;
  const clip = cs.getPropertyValue("clip");
  const tiny = cs.width !== "" && px(cs.width) <= 1 && px(cs.height) <= 1 && cs.overflow === "hidden";
  return (clipPath !== "" && clipPath !== "none") || (clip !== "" && clip !== "auto") || tiny;
}

function skipped(el: Element, cs: CSSStyleDeclaration, r: Rect, w: Walk): boolean {
  if (SKIP_TAGS.has(el.tagName.toUpperCase()) || hiddenByStyle(cs) || clippedAway(cs)) {
    return true;
  }
  const visible = w.visible[w.visible.length - 1];
  return cs.display !== "contents" && visible !== undefined && outside(r, visible);
}

/** Background, then border (study L433-444). */
function boxOps(cs: CSSStyleDeclaration, r: Rect, a: number, ops: Op[]): void {
  const rad = radiusOf(cs, r);
  if (!isClear(cs.backgroundColor)) {
    ops.push({ k: "fill", r, rad, c: cs.backgroundColor, a });
  }
  const top = { w: px(cs.borderTopWidth), c: cs.borderTopColor };
  const right = { w: px(cs.borderRightWidth), c: cs.borderRightColor };
  const bottom = { w: px(cs.borderBottomWidth), c: cs.borderBottomColor };
  const left = { w: px(cs.borderLeftWidth), c: cs.borderLeftColor };
  if ([right, bottom, left].every((side) => side.w === top.w && side.c === top.c)) {
    if (top.w > 0 && !isClear(top.c)) {
      ops.push({ k: "stroke", r, rad, w: top.w, c: top.c, a });
    }
    return;
  }
  const edges: [typeof top, Rect][] = [
    [top, { x: r.x, y: r.y, w: r.w, h: top.w }],
    [right, { x: r.x + r.w - right.w, y: r.y, w: right.w, h: r.h }],
    [bottom, { x: r.x, y: r.y + r.h - bottom.w, w: r.w, h: bottom.w }],
    [left, { x: r.x, y: r.y, w: left.w, h: r.h }],
  ];
  for (const [side, rect] of edges) {
    if (side.w > 0 && !isClear(side.c)) {
      ops.push({ k: "rect", r: rect, c: side.c, a });
    }
  }
}

/** The content box of an element: inside its border and padding. */
function contentBox(cs: CSSStyleDeclaration, r: Rect): Rect {
  const left = px(cs.paddingLeft) + px(cs.borderLeftWidth);
  const top = px(cs.paddingTop) + px(cs.borderTopWidth);
  const right = px(cs.paddingRight) + px(cs.borderRightWidth);
  const bottom = px(cs.paddingBottom) + px(cs.borderBottomWidth);
  return { x: r.x + left, y: r.y + top, w: Math.max(0, r.w - left - right), h: Math.max(0, r.h - top - bottom) };
}

/** The padding box: inside the border. This is what an overflow clip cuts to. */
function paddingBox(cs: CSSStyleDeclaration, r: Rect): Rect {
  const left = px(cs.borderLeftWidth);
  const top = px(cs.borderTopWidth);
  return {
    x: r.x + left,
    y: r.y + top,
    w: Math.max(0, r.w - left - px(cs.borderRightWidth)),
    h: Math.max(0, r.h - top - px(cs.borderBottomWidth)),
  };
}

function textAt(text: string, box: Rect, cs: CSSStyleDeclaration, a: number): Op {
  return { k: "text", text, x: box.x, y: box.y, h: box.h, font: fontOf(cs), c: cs.color, ls: cs.letterSpacing, sh: null, a };
}

/** A secret: its box and the static mask. The content is never read. */
function secretOps({ el, cs, r, a }: Item, ops: Op[]): void {
  boxOps(cs, r, a, ops);
  const mask = el.getAttribute(MASK_ATTR);
  if (mask !== null && mask !== "") {
    ops.push(textAt(mask, contentBox(cs, r), cs, a));
  }
}

function isEditable(el: Element): boolean {
  const attr = el.getAttribute("contenteditable");
  return attr !== null && attr !== "false";
}

function isControl(el: Element): boolean {
  return (
    el instanceof HTMLInputElement ||
    el instanceof HTMLTextAreaElement ||
    el instanceof HTMLSelectElement ||
    isEditable(el)
  );
}

/** A text control: its box and, when it is empty, its placeholder. The value is never painted. */
function controlOps({ el, cs, r, a }: Item, ops: Op[]): void {
  boxOps(cs, r, a, ops);
  if (el instanceof HTMLSelectElement) {
    const label = el.selectedOptions[0]?.label;
    if (label !== undefined && label !== "") {
      ops.push(textAt(label, contentBox(cs, r), cs, a));
    }
    return;
  }
  const empty = (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) && el.value === "";
  const placeholder = el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement ? el.placeholder : "";
  if (empty && placeholder !== "") {
    ops.push(textAt(placeholder, contentBox(cs, r), cs, a * 0.6));
  }
}

function svgItems(svg: Element): SvgItem[] {
  const items: SvgItem[] = [];
  for (const child of svg.querySelectorAll("path, rect, circle")) {
    const kind = child.tagName.toLowerCase();
    if (kind !== "path" && kind !== "rect" && kind !== "circle") {
      continue;
    }
    const style = getComputedStyle(child);
    const attrs: Record<string, string> = {};
    for (const attr of child.attributes) {
      attrs[attr.name] = attr.value;
    }
    items.push({
      kind,
      attrs,
      fill: style.fill === "" || style.fill === "none" ? null : style.fill,
      stroke: style.stroke === "" || style.stroke === "none" ? null : style.stroke,
      lw: Number.parseFloat(style.strokeWidth) || 1,
    });
  }
  return items;
}

function viewBoxOf(svg: Element, r: Rect): readonly [number, number, number, number] {
  const parts = (svg.getAttribute("viewBox") ?? "").split(/[\s,]+/).map(Number);
  const [x, y, w, h] = parts;
  if (parts.length === 4 && x !== undefined && y !== undefined && w !== undefined && h !== undefined) {
    if ([x, y, w, h].every(Number.isFinite) && w > 0 && h > 0) {
      return [x, y, w, h];
    }
  }
  return [0, 0, Math.max(r.w, 1), Math.max(r.h, 1)];
}

/** A canvas or image is drawn as it is (the foil tiles, the holo buttons). */
function liveOps({ el, cs, r, a }: Item, ops: Op[]): void {
  if (!(el instanceof HTMLCanvasElement || el instanceof HTMLImageElement)) {
    return;
  }
  const rad = radiusOf(cs, r);
  if (!isClear(cs.backgroundColor)) {
    ops.push({ k: "fill", r, rad, c: cs.backgroundColor, a });
  }
  ops.push({ k: "live", el, r, rad, a });
}

function isLive(el: Element): boolean {
  if (el instanceof HTMLCanvasElement) {
    return el.width > 0;
  }
  return el instanceof HTMLImageElement && el.complete && el.naturalWidth > 0;
}

const isGroupElement = (el: Element): boolean => el.matches(GROUP_SELECTOR) || el.matches(POP_SELECTOR);

function openGroup(el: HTMLElement, parent: number, rect: Rect, w: Walk): number {
  w.out.groups.push({
    el,
    parent,
    kind: el.matches(POP_SELECTOR) ? "pop" : "item",
    origin: el.matches(ORIGIN_SELECTOR) || el.querySelector(ORIGIN_SELECTOR) !== null,
    rect,
    // A group is composed apart from the page, so it repeats the clips it sits in.
    ops: w.clips.map((clip): Op => ({ k: "clip", r: clip })),
  });
  return w.out.groups.length - 1;
}

function walkChildren(item: Item, ops: Op[], w: Walk): void {
  for (const node of item.el.childNodes) {
    if (node instanceof Text) {
      ops.push(...collectText(node, item.cs, item.a, w.measure));
    } else if (node instanceof Element) {
      walk(node, { g: item.g, a: item.a }, w);
    }
  }
}

/** Children of a box, inside its overflow clip (a scroller, a truncated label). */
function walkClipped(item: Item, ops: Op[], w: Walk): void {
  const { cs } = item;
  const clipping = !item.root && (cs.overflowX !== "visible" || cs.overflowY !== "visible");
  if (!clipping) {
    walkChildren(item, ops, w);
    return;
  }
  const box = paddingBox(cs, item.r);
  ops.push({ k: "clip", r: box });
  w.clips.push(box);
  w.visible.push(intersect(w.visible[w.visible.length - 1] ?? box, box));
  walkChildren(item, ops, w);
  w.visible.pop();
  w.clips.pop();
  ops.push({ k: "unclip" });
}

/** Paints one element's own content, and walks into its children where that is safe. */
function paintItem(item: Item, w: Walk): void {
  const { el, cs, r, a } = item;
  const ops = item.g < 0 ? w.out.base : (w.out.groups[item.g]?.ops ?? w.out.base);
  if (el.hasAttribute(SECRET_ATTR)) {
    secretOps(item, ops);
  } else if (isControl(el)) {
    controlOps(item, ops);
  } else if (el instanceof HTMLCanvasElement || el instanceof HTMLImageElement) {
    if (isLive(el)) {
      liveOps(item, ops);
    }
  } else if (el.tagName.toLowerCase() === "svg") {
    ops.push({ k: "svg", r, vb: viewBoxOf(el, r), items: svgItems(el), a });
  } else {
    boxOps(cs, r, a, ops);
    walkClipped(item, ops, w);
  }
}

/** Where a walk step comes from: the group it paints into and the opacity passed down. */
interface Parent {
  g: number;
  a: number;
}

/** Opacity of an element and everything above it (the root ignores its own). */
function opacityOf(cs: CSSStyleDeclaration, parent: Parent, root: boolean): number {
  const opacity = Number.parseFloat(cs.opacity);
  return root || Number.isNaN(opacity) ? parent.a : parent.a * opacity;
}

function walk(el: Element, parent: Parent, w: Walk, root = false): void {
  const cs = getComputedStyle(el);
  const r = w.measure.rect(el);
  if (!root && skipped(el, cs, r, w)) {
    return;
  }
  const opens = w.groups && !root && el instanceof HTMLElement && isGroupElement(el);
  const g = opens ? openGroup(el, parent.g, r, w) : parent.g;
  paintItem({ el, cs, r, a: opacityOf(cs, parent, root), g, root }, w);
  if (opens) {
    // Close the clips the group repeated at its start.
    const ops = w.out.groups[g]?.ops;
    for (let i = 0; i < w.clips.length; i++) {
      ops?.push({ k: "unclip" });
    }
  }
}

/**
 * The display list of `root` and everything in it. With `groups`, the elements that move on
 * their own in the choreography get an op list of their own.
 */
export function collect(root: Element, measure: Measure, opts: { groups: boolean }): Collected {
  const out: Collected = { base: [], groups: [] };
  const stage = measure.rect(root);
  walk(root, { g: -1, a: 1 }, { measure, groups: opts.groups, out, clips: [], visible: [stage] }, true);
  return out;
}

export function domMeasure(base: { readonly left: number; readonly top: number }): Measure {
  let range: Range | null = null;
  const relative = (r: DOMRectReadOnly): Rect => ({
    x: r.left - base.left,
    y: r.top - base.top,
    w: r.width,
    h: r.height,
  });
  return {
    rect: (el) => relative(el.getBoundingClientRect()),
    textRects: (node, start, end) => {
      range ??= document.createRange();
      range.setStart(node, start);
      range.setEnd(node, end);
      return Array.from(range.getClientRects(), relative);
    },
  };
}
