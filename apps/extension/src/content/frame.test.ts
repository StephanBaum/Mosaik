import { describe, expect, it } from "vitest";
import { frameAncestry, MAX_FRAME_DEPTH, type FrameWindow } from "./frame.ts";
import type { VisibilityEnv } from "./visibility.ts";

const visibleEnv: VisibilityEnv = {
  checkVisibility: (el) => (el as HTMLElement).dataset["hidden"] === undefined,
  box: () => ({ left: 0, top: 0, right: 300, bottom: 150, width: 300, height: 150 }),
  opacity: () => 1,
  parent: (el) => el.parentElement,
  viewport: () => ({ width: 1000, height: 800, scrollX: 0, scrollY: 0, docWidth: 1000, docHeight: 800 }),
  elementAt: (el) => el,
};

interface Spec {
  origin: string;
  /** Reading this window's location throws, as it does for a cross-origin window. */
  blocked?: boolean;
  /** The `<iframe>` that holds this window is hidden. */
  hidden?: boolean;
}

/** A chain of windows, top first; the last one is the frame under test. */
function chain(specs: Spec[]): FrameWindow {
  let parent: FrameWindow | null = null;
  let last: FrameWindow | null = null;
  for (const spec of specs) {
    const owner: FrameWindow | null = parent;
    const iframe = owner === null ? null : document.createElement("iframe");
    if (iframe !== null) {
      document.body.append(iframe);
      if (spec.hidden === true) {
        iframe.dataset["hidden"] = "1";
      }
    }
    const win: FrameWindow = {
      get parent(): FrameWindow {
        return owner ?? win;
      },
      frameElement: iframe,
      get location(): { origin: string } {
        if (spec.blocked === true) {
          throw new DOMException("Blocked a frame from accessing a cross-origin frame.", "SecurityError");
        }
        return { origin: spec.origin };
      },
    };
    parent = win;
    last = win;
  }
  if (last === null) {
    throw new Error("empty chain");
  }
  return last;
}

const A = "https://a.test";
const EVIL = "https://evil.test";

describe("frameAncestry", () => {
  it("the top frame has no ancestors", () => {
    expect(frameAncestry(chain([{ origin: A }]), visibleEnv)).toEqual({
      ancestorsSameOrigin: true,
      frameVisible: true,
      depth: 0,
    });
  });

  it("a same-origin child and grandchild are fine", () => {
    const win = chain([{ origin: A }, { origin: A }, { origin: A }]);
    expect(frameAncestry(win, visibleEnv)).toEqual({ ancestorsSameOrigin: true, frameVisible: true, depth: 2 });
  });

  it("a cross-origin parent is refused", () => {
    const win = chain([{ origin: EVIL }, { origin: A }]);
    expect(frameAncestry(win, visibleEnv).ancestorsSameOrigin).toBe(false);
  });

  it("A inside B inside A is refused: an ancestor in the middle has another origin", () => {
    const win = chain([{ origin: A }, { origin: EVIL }, { origin: A }]);
    expect(frameAncestry(win, visibleEnv)).toMatchObject({ ancestorsSameOrigin: false, depth: 2 });
  });

  it("a parent whose location cannot be read counts as cross-origin", () => {
    const win = chain([{ origin: A, blocked: true }, { origin: A }]);
    expect(frameAncestry(win, visibleEnv).ancestorsSameOrigin).toBe(false);
  });

  it("an embedder that is not reachable (frameElement is null) counts as cross-origin", () => {
    const win = chain([{ origin: A }, { origin: A }]);
    const orphan: FrameWindow = { parent: win.parent, frameElement: null, location: { origin: A } };
    expect(frameAncestry(orphan, visibleEnv).ancestorsSameOrigin).toBe(false);
  });

  it("an opaque origin is never same-origin, even under an opaque parent", () => {
    const win = chain([{ origin: "null" }, { origin: "null" }]);
    expect(frameAncestry(win, visibleEnv).ancestorsSameOrigin).toBe(false);
  });

  it("a frame element the user cannot see makes the frame invisible", () => {
    const hiddenFrame = chain([{ origin: A }, { origin: A, hidden: true }]);
    expect(frameAncestry(hiddenFrame, visibleEnv)).toMatchObject({ ancestorsSameOrigin: true, frameVisible: false });
    const tinyEnv: VisibilityEnv = {
      ...visibleEnv,
      box: () => ({ left: 0, top: 0, right: 1, bottom: 1, width: 1, height: 1 }),
    };
    expect(frameAncestry(chain([{ origin: A }, { origin: A }]), tinyEnv).frameVisible).toBe(false);
  });

  it("gives up beyond the depth limit", () => {
    const specs = Array.from({ length: MAX_FRAME_DEPTH + 2 }, () => ({ origin: A }));
    expect(frameAncestry(chain(specs), visibleEnv).ancestorsSameOrigin).toBe(false);
  });
});
