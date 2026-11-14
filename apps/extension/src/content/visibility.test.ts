import { afterEach, describe, expect, it, vi } from "vitest";
import {
  isElementVisible,
  isFillable,
  opacityProduct,
  ownOpacity,
  type Box,
  type VisibilityEnv,
} from "./visibility.ts";

const box = (left: number, top: number, width: number, height: number): Box => ({
  left,
  top,
  width,
  height,
  right: left + width,
  bottom: top + height,
});

/** A page where everything is visible, replaced piece by piece in each test. */
function makeEnv(over: Partial<VisibilityEnv> = {}): VisibilityEnv {
  return {
    checkVisibility: () => true,
    box: () => box(100, 100, 200, 30),
    opacity: () => 1,
    parent: (el) => el.parentElement,
    viewport: () => ({ width: 1000, height: 800, scrollX: 0, scrollY: 0, docWidth: 1000, docHeight: 2000 }),
    elementAt: (el) => el,
    clip: () => ({ overflow: false, shape: false }),
    ...over,
  };
}

function field(): HTMLInputElement {
  document.body.innerHTML = '<div id="outer"><div id="inner"><input id="f"></div></div>';
  return document.querySelector<HTMLInputElement>("#f") as HTMLInputElement;
}

afterEach(() => {
  document.body.innerHTML = "";
});

describe("isFillable", () => {
  it("accepts a visible field that is on top", () => {
    expect(isFillable(field(), makeEnv())).toBe(true);
  });

  it("refuses a field that is not connected", () => {
    const el = document.createElement("input");
    expect(isFillable(el, makeEnv())).toBe(false);
  });

  it("refuses what CSS hides (display none, visibility hidden, content-visibility)", () => {
    expect(isFillable(field(), makeEnv({ checkVisibility: () => false }))).toBe(false);
  });

  it.each([
    ["width 3", box(100, 100, 3, 30)],
    ["height 3", box(100, 100, 200, 3)],
    ["zero size", box(100, 100, 0, 0)],
  ])("refuses a field that is too small (%s)", (_name, size) => {
    expect(isFillable(field(), makeEnv({ box: () => size }))).toBe(false);
  });

  it("accepts the smallest size", () => {
    expect(isFillable(field(), makeEnv({ box: () => box(100, 100, 4, 4) }))).toBe(true);
  });

  it("multiplies opacity over the field and its ancestors", () => {
    const el = field();
    const opacities: Record<string, number> = { f: 0.5, inner: 0.5, outer: 0.9 };
    const env = makeEnv({ opacity: (node) => opacities[node.id] ?? 1 });
    expect(opacityProduct(el, env)).toBeCloseTo(0.5 * 0.5 * 0.9 * 1 * 1);
    expect(isFillable(el, env)).toBe(true);
    opacities["outer"] = 0.7;
    expect(isFillable(el, env)).toBe(false);
  });

  it("refuses opacity 0 on the field itself and on an ancestor", () => {
    const el = field();
    expect(isFillable(el, makeEnv({ opacity: (node) => (node.id === "f" ? 0 : 1) }))).toBe(false);
    expect(isFillable(el, makeEnv({ opacity: (node) => (node.id === "outer" ? 0 : 1) }))).toBe(false);
  });

  it("follows the parent function across a shadow host", () => {
    const el = field();
    const host = document.createElement("div");
    host.id = "host";
    const env = makeEnv({
      parent: (node) => (node.id === "outer" ? host : node.parentElement),
      opacity: (node) => (node === host ? 0.1 : 1),
    });
    expect(isFillable(el, env)).toBe(false);
  });

  it("refuses a field placed off the left or the top of the document", () => {
    expect(isFillable(field(), makeEnv({ box: () => box(-9999, 100, 200, 30) }))).toBe(false);
    expect(isFillable(field(), makeEnv({ box: () => box(100, -9999, 200, 30) }))).toBe(false);
  });

  it("counts the scroll offset: a field above the viewport of a scrolled page is still in the document", () => {
    const env = makeEnv({
      box: () => box(100, -300, 200, 30),
      viewport: () => ({ width: 1000, height: 800, scrollX: 0, scrollY: 600, docWidth: 1000, docHeight: 2000 }),
    });
    expect(isFillable(field(), env)).toBe(true);
  });

  it("refuses a field beyond the right or bottom edge of the document", () => {
    expect(isFillable(field(), makeEnv({ box: () => box(5000, 100, 200, 30) }))).toBe(false);
    expect(isFillable(field(), makeEnv({ box: () => box(100, 5000, 200, 30) }))).toBe(false);
  });

  it("does not hit-test a field that is outside the viewport", () => {
    const elementAt = vi.fn(() => null);
    const env = makeEnv({ box: () => box(100, 1500, 200, 30), elementAt });
    expect(isFillable(field(), env)).toBe(true);
    expect(elementAt).not.toHaveBeenCalled();
  });

  it("refuses a field covered by something else at every sample point", () => {
    const cover = document.createElement("div");
    expect(isFillable(field(), makeEnv({ elementAt: () => cover }))).toBe(false);
    expect(isFillable(field(), makeEnv({ elementAt: () => null }))).toBe(false);
  });

  it("needs the centre and a majority of the five points", () => {
    const el = field();
    const cover = document.createElement("div");
    const hitsAt = (pattern: boolean[]): VisibilityEnv => {
      let calls = 0;
      return makeEnv({ elementAt: () => (pattern[calls++] === true ? el : cover) });
    };
    expect(isFillable(el, hitsAt([true, true, true, false, false]), () => 0.5)).toBe(true);
    expect(isFillable(el, hitsAt([true, true, false, false, false]), () => 0.5)).toBe(false);
    expect(isFillable(el, hitsAt([false, true, true, true, true]), () => 0.5)).toBe(false);
  });

  it("refuses a label over the field, accepts a child of the field", () => {
    document.body.innerHTML = '<label for="f" id="l">x</label><div id="w"><input id="f"><span id="c"></span></div>';
    const el = document.querySelector("#w") as HTMLElement;
    const label = document.querySelector("#l") as HTMLElement;
    expect(isFillable(el, makeEnv({ elementAt: () => label }), () => 0.5)).toBe(false);
    const input = document.querySelector("#f") as HTMLInputElement;
    expect(isFillable(input, makeEnv({ elementAt: () => label }), () => 0.5)).toBe(false);
    expect(isFillable(el, makeEnv({ elementAt: () => document.querySelector("#c") }), () => 0.5)).toBe(true);
  });

  it("draws four points inside the box, with a margin, from the injected source", () => {
    const points: [number, number][] = [];
    const env = makeEnv({
      box: () => box(100, 100, 200, 30),
      elementAt: (el, x, y) => {
        points.push([x, y]);
        return el;
      },
    });
    for (const r of [0, 0.999999]) {
      points.length = 0;
      expect(isFillable(field(), env, () => r)).toBe(true);
      expect(points).toHaveLength(5);
      expect(points[0]).toEqual([200, 115]);
      for (const [x, y] of points.slice(1)) {
        expect(x).toBeGreaterThanOrEqual(102);
        expect(x).toBeLessThanOrEqual(298);
        expect(y).toBeGreaterThanOrEqual(102);
        expect(y).toBeLessThanOrEqual(128);
      }
    }
  });

  it("changes the points per check with the real source", () => {
    const seen = new Set<string>();
    const env = makeEnv({
      elementAt: (el, x, y) => {
        seen.add([x, y].join(","));
        return el;
      },
    });
    const el = field();
    isFillable(el, env);
    isFillable(el, env);
    expect(seen.size).toBeGreaterThan(5);
  });

  it("fails closed when no random source works", () => {
    const boom = (): number => {
      throw new Error("no entropy");
    };
    expect(isFillable(field(), makeEnv(), boom)).toBe(false);
  });

  it("keeps the points inside a tiny box", () => {
    const points: [number, number][] = [];
    const env = makeEnv({
      box: () => box(100, 100, 4, 4),
      elementAt: (el, x, y) => {
        points.push([x, y]);
        return el;
      },
    });
    expect(isFillable(field(), env, () => 1)).toBe(true);
    expect(points.every(([x, y]) => x >= 100 && x <= 104 && y >= 100 && y <= 104)).toBe(true);
  });

  it("samples the visible part when the field is half off screen", () => {
    const el = field();
    const points: [number, number][] = [];
    const env = makeEnv({
      box: () => box(-100, 100, 200, 30),
      elementAt: (_el, x, y) => {
        points.push([x, y]);
        return null;
      },
    });
    expect(isFillable(el, env)).toBe(false);
    expect(points).toHaveLength(5);
    expect(points.every(([x]) => x >= 0 && x <= 100)).toBe(true);
  });

  it("refuses a label whose control is the field (RT-EXT-1)", () => {
    document.body.innerHTML = '<label id="l" for="f">Name</label><input id="f">';
    const el = document.querySelector<HTMLInputElement>("#f") as HTMLInputElement;
    const label = document.querySelector("#l") as HTMLLabelElement;
    expect(isFillable(el, makeEnv({ elementAt: () => label }))).toBe(false);
    const other = document.createElement("label");
    expect(isFillable(el, makeEnv({ elementAt: () => other }))).toBe(false);
  });
});

describe("off screen, where no hit test runs", () => {
  const below = box(100, 1500, 200, 30);

  it("refuses a field an ancestor's overflow box clips away", () => {
    const el = field();
    const env = makeEnv({
      box: (node) => (node.id === "outer" ? box(100, 1500, 200, 0) : below),
      clip: (node) => ({ overflow: node.id === "outer", shape: false }),
    });
    expect(isFillable(el, env)).toBe(false);
  });

  it("accepts a field inside an overflow box that shows it", () => {
    const env = makeEnv({
      box: (node) => (node.id === "outer" ? box(0, 1400, 800, 400) : below),
      clip: (node) => ({ overflow: node.id === "outer", shape: false }),
    });
    expect(isFillable(field(), env)).toBe(true);
  });

  it("refuses a field with a clip shape on it or an ancestor", () => {
    for (const id of ["f", "inner"]) {
      const env = makeEnv({ box: () => below, clip: (node) => ({ overflow: false, shape: node.id === id }) });
      expect(isFillable(field(), env)).toBe(false);
    }
  });

  it("ignores the overflow of the root element and the body (that is the viewport's)", () => {
    const env = makeEnv({
      box: (node) => (node.localName === "body" || node.localName === "html" ? box(0, 0, 0, 0) : below),
      clip: (node) => ({ overflow: node.localName === "body" || node.localName === "html", shape: false }),
    });
    expect(isFillable(field(), env)).toBe(true);
  });

  it("refuses any field off screen in a frame", () => {
    const env = makeEnv({
      box: () => below,
      viewport: () => ({ width: 1000, height: 800, scrollX: 0, scrollY: 0, docWidth: 1000, docHeight: 2000, framed: true }),
    });
    expect(isFillable(field(), env)).toBe(false);
  });
});

describe("ownOpacity", () => {
  const style = (opacity: string, filter: string, mask = "none") => ({
    opacity,
    filter,
    getPropertyValue: (name: string) => (name.endsWith("mask-image") ? mask : ""),
  });

  it("multiplies opacity with every filter opacity()", () => {
    expect(ownOpacity(style("1", "none"))).toBe(1);
    expect(ownOpacity(style("1", "opacity(0)"))).toBe(0);
    expect(ownOpacity(style("0.8", "blur(2px) opacity(0.5)"))).toBeCloseTo(0.4);
    expect(ownOpacity(style("1", "opacity(50%) opacity(0.5)"))).toBeCloseTo(0.25);
    expect(ownOpacity(style("1", "opacity(2)"))).toBe(1);
  });

  it("counts any mask as hiding", () => {
    expect(ownOpacity(style("1", "none", "linear-gradient(transparent, transparent)"))).toBe(0);
  });
});

describe("isElementVisible", () => {
  it("is the same test without the position and hit checks", () => {
    const env = makeEnv({ box: () => box(-9999, 100, 200, 30), elementAt: () => null });
    expect(isElementVisible(field(), env)).toBe(true);
    expect(isElementVisible(field(), makeEnv({ opacity: () => 0.1 }))).toBe(false);
  });
});
