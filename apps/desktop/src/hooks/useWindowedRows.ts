/**
 * Windowed rendering for long lists of equal-height rows: only the rows inside the scroll
 * viewport (plus an overscan) are in the DOM. The row height is measured from a rendered
 * row, so text scaling and theme changes keep the geometry right.
 */
import { useLayoutEffect, useState, type RefObject } from "react";

/** Rows rendered on each side of the viewport, so Tab and arrow keys always find a neighbour. */
export const OVERSCAN = 8;
/** Rows rendered before the first measurement (and in environments without layout). */
export const INITIAL_ROWS = 40;

export interface RowRange {
  /** First rendered row. */
  from: number;
  /** One past the last rendered row. */
  to: number;
}

/** A vertical span in list coordinates (0 = top of the first row). */
export interface View {
  top: number;
  bottom: number;
}

/** The rows that intersect `view`, widened by `OVERSCAN` on each side and clamped to `[0, count)`. */
export function visibleRange(view: View, rowHeight: number, count: number): RowRange {
  if (rowHeight <= 0) {
    return { from: 0, to: Math.min(count, INITIAL_ROWS) };
  }
  const from = Math.max(0, Math.floor(view.top / rowHeight) - OVERSCAN);
  const to = Math.min(count, Math.max(from, Math.ceil(view.bottom / rowHeight) + OVERSCAN));
  return { from: Math.min(from, to), to };
}

/** The sorted, distinct indices to render: the range plus pinned rows (selection, focus). */
export function renderedIndices(range: RowRange, pinned: readonly number[], count: number): number[] {
  const rows = new Set<number>();
  for (let index = range.from; index < range.to; index++) {
    rows.add(index);
  }
  for (const index of pinned) {
    if (index >= 0 && index < count) {
      rows.add(index);
    }
  }
  return [...rows].sort((a, b) => a - b);
}

function scrollParent(element: HTMLElement): HTMLElement | null {
  for (let node = element.parentElement; node !== null; node = node.parentElement) {
    const { overflowY } = getComputedStyle(node);
    if (overflowY === "auto" || overflowY === "scroll") {
      return node;
    }
  }
  return null;
}

/**
 * Tracks which rows of `list` are visible. Sets `--row-height` and `--rows` on the list for
 * the stylesheet, which positions each row at `var(--row) * var(--row-height)`.
 */
export function useWindowedRows(
  list: RefObject<HTMLElement | null>,
  rowSelector: string,
  count: number,
  enabled: boolean,
): RowRange {
  const [range, setRange] = useState<RowRange>(() => visibleRange({ top: 0, bottom: 0 }, 0, count));
  useLayoutEffect(() => {
    const element = list.current;
    if (!enabled || element === null) {
      return;
    }
    const scroller = scrollParent(element);
    const update = () => {
      const row = element.querySelector<HTMLElement>(rowSelector);
      const height = row?.offsetHeight ?? 0;
      element.style.setProperty("--rows", String(count));
      if (height > 0) {
        element.style.setProperty("--row-height", `${String(height)}px`);
      }
      const top = element.getBoundingClientRect().top;
      const viewTop = scroller === null ? 0 : scroller.getBoundingClientRect().top;
      const viewBottom = scroller === null ? window.innerHeight : scroller.getBoundingClientRect().bottom;
      const next = visibleRange({ top: viewTop - top, bottom: viewBottom - top }, height, count);
      setRange((current) => (current.from === next.from && current.to === next.to ? current : next));
    };
    update();
    const target: HTMLElement | Window = scroller ?? window;
    target.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(update);
    observer?.observe(element);
    if (scroller !== null) {
      observer?.observe(scroller);
    }
    return () => {
      target.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
      observer?.disconnect();
    };
  }, [list, rowSelector, count, enabled]);
  return enabled ? range : { from: 0, to: count };
}
