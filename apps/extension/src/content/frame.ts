/**
 * Where this frame sits. A frame may only be filled when every ancestor is same-origin with it
 * (no A-B-A nesting, no sandboxed or opaque frame) and every `<iframe>` element on the way is
 * one the user can see: the field rules of `isFillable` (size, opacity, on top, not clipped),
 * applied in the parent's document.
 *
 * Origins are the documents' own (`window.origin`), not their URLs' (`location.origin`): a
 * document under a CSP `sandbox` or in a sandboxed frame has an opaque origin ("null") although
 * its URL names a site.
 */
import { domEnv, isFillable, type VisibilityEnv } from "./visibility.ts";

/** The part of `Window` this module reads (so tests can pass plain objects). */
export interface FrameWindow {
  readonly parent: FrameWindow;
  readonly frameElement: Element | null;
  /** The document's origin; reading it on a cross-origin window throws. */
  readonly origin: string;
}

export interface Ancestry {
  ancestorsSameOrigin: boolean;
  frameVisible: boolean;
  /** Number of ancestor frames: 0 for the top frame. */
  depth: number;
}

export const MAX_FRAME_DEPTH = 16;

function depthOf(win: FrameWindow): number {
  let depth = 0;
  for (let w = win; w.parent !== w && depth < MAX_FRAME_DEPTH; w = w.parent) {
    depth++;
  }
  return depth;
}

/** Reads the parent's origin; a cross-origin parent throws, which counts as "not same-origin". */
function parentOrigin(win: FrameWindow): string | null {
  try {
    return win.parent.origin;
  } catch {
    return null;
  }
}

export function frameAncestry(win: FrameWindow, env: VisibilityEnv = domEnv): Ancestry {
  const depth = depthOf(win);
  const own = win.origin;
  let visible = true;
  let w = win;
  for (let level = 0; level < depth; level++) {
    const element = w.frameElement;
    if (own === "null" || element === null || parentOrigin(w) !== own) {
      return { ancestorsSameOrigin: false, frameVisible: false, depth };
    }
    visible &&= isFillable(element, env);
    w = w.parent;
  }
  return { ancestorsSameOrigin: depth < MAX_FRAME_DEPTH, frameVisible: visible, depth };
}
