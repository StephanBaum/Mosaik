/** The snapshot display list: plain data, so it can be built and tested without a canvas. */

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Shadow {
  c: string;
  x: number;
  y: number;
  b: number;
}

export interface SvgItem {
  kind: "path" | "rect" | "circle";
  /** The geometry attributes of the element (`d`, `x`, `cx`, ...), as written. */
  attrs: Readonly<Record<string, string>>;
  fill: string | null;
  stroke: string | null;
  /** Stroke width in the svg's own units. */
  lw: number;
}

export type Op =
  | { k: "fill"; r: Rect; rad: number; c: string; a: number }
  | { k: "rect"; r: Rect; c: string; a: number }
  | { k: "stroke"; r: Rect; rad: number; w: number; c: string; a: number }
  | {
      k: "text";
      text: string;
      x: number;
      y: number;
      /** Height of the text's line box; the baseline is centred in it. */
      h: number;
      font: string;
      c: string;
      ls: string;
      sh: Shadow | null;
      a: number;
    }
  | {
      k: "svg";
      r: Rect;
      vb: readonly [number, number, number, number];
      items: readonly SvgItem[];
      a: number;
    }
  | { k: "clip"; r: Rect }
  | { k: "unclip" }
  | { k: "live"; el: HTMLCanvasElement | HTMLImageElement; r: Rect; rad: number; a: number };

export type GroupKind = "item" | "pop";

export interface Group {
  el: HTMLElement;
  /** Index of the enclosing group, or -1. */
  parent: number;
  kind: GroupKind;
  /** The group holds the clicked lock control. */
  origin: boolean;
  rect: Rect;
  ops: Op[];
}

export interface Collected {
  base: Op[];
  groups: Group[];
}

/** Where things are, relative to the stage. Injectable, so tests need no layout engine. */
export interface Measure {
  rect(el: Element): Rect;
  /** The line boxes of a text range (one rect per line). */
  textRects(node: Text, start: number, end: number): readonly Rect[];
}

export const SECRET_ATTR = "data-secret";
export const MASK_ATTR = "data-mask";
