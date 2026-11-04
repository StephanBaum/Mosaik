/** Text nodes to text ops (study L463-470): one op per line, so wrapped text lands where it wraps. */
import type { Measure, Op, Rect, Shadow } from "./ops.ts";

/** Lines whose tops differ by at most this are the same line. */
const SAME_LINE_PX = 1;

export function fontOf(cs: CSSStyleDeclaration): string {
  return `${cs.fontStyle} ${cs.fontWeight} ${cs.fontSize} ${cs.fontFamily}`;
}

function shadowOf(cs: CSSStyleDeclaration): Shadow | null {
  const shadow = cs.textShadow;
  if (shadow === "" || shadow === "none") {
    return null;
  }
  const m = /(rgba?\([^)]*\))\s+(-?[\d.]+)px\s+(-?[\d.]+)px\s+([\d.]+)px/.exec(shadow);
  return m === null ? null : { c: m[1] ?? "", x: Number(m[2]), y: Number(m[3]), b: Number(m[4]) };
}

function transformed(text: string, textTransform: string): string {
  if (textTransform === "uppercase") {
    return text.toUpperCase();
  }
  return textTransform === "lowercase" ? text.toLowerCase() : text;
}

interface Line {
  words: string[];
  x: number;
  top: number;
  bottom: number;
}

/** Words of a wrapped text grouped by the line they sit on, each measured on its own. */
function linesOf(node: Text, from: number, to: number, measure: Measure): Line[] {
  const lines: Line[] = [];
  const words = node.data.slice(from, to).matchAll(/\S+/g);
  for (const word of words) {
    const start = from + word.index;
    const rect = measure.textRects(node, start, start + word[0].length)[0];
    if (rect === undefined) {
      continue;
    }
    const line = lines.find((l) => Math.abs(l.top - rect.y) <= SAME_LINE_PX);
    if (line === undefined) {
      lines.push({ words: [word[0]], x: rect.x, top: rect.y, bottom: rect.y + rect.h });
    } else {
      line.words.push(word[0]);
      line.x = Math.min(line.x, rect.x);
      line.bottom = Math.max(line.bottom, rect.y + rect.h);
    }
  }
  return lines;
}

function rectsOf(node: Text, from: number, to: number, measure: Measure): Rect[] {
  return measure.textRects(node, from, to).filter((r) => r.h > 0);
}

/** The text ops of one text node. Whitespace-only nodes have none. */
export function collectText(node: Text, cs: CSSStyleDeclaration, a: number, measure: Measure): Op[] {
  const from = node.data.search(/\S/);
  if (from < 0) {
    return [];
  }
  const to = node.data.trimEnd().length;
  const rects = rectsOf(node, from, to, measure);
  const common = { font: fontOf(cs), c: cs.color, ls: cs.letterSpacing, sh: shadowOf(cs), a };
  const say = (text: string) => transformed(text.replace(/\s+/g, " "), cs.textTransform);
  const [first] = rects;
  if (first === undefined) {
    return [];
  }
  if (rects.length === 1) {
    return [{ k: "text", text: say(node.data.slice(from, to)), x: first.x, y: first.y, h: first.h, ...common }];
  }
  return linesOf(node, from, to, measure).map(
    (line): Op => ({
      k: "text",
      text: say(line.words.join(" ")),
      x: line.x,
      y: line.top,
      h: line.bottom - line.top,
      ...common,
    }),
  );
}
