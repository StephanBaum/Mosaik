import { afterEach, describe, expect, it, vi } from "vitest";
import { isElementVisible, isFillable, opacityProduct, type Box, type VisibilityEnv } from "./visibility.ts";

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

  it("accepts a field that is on top at just one of the five points", () => {
    const el = field();
    const cover = document.createElement("div");
    let calls = 0;
    const env = makeEnv({ elementAt: () => (++calls === 3 ? el : cover) });
    expect(isFillable(el, env)).toBe(true);
    expect(calls).toBe(3);
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

  it("treats a label whose control is the field, and a descendant, as the field", () => {
    document.body.innerHTML = '<label id="l" for="f">Name</label><input id="f">';
    const el = document.querySelector<HTMLInputElement>("#f") as HTMLInputElement;
    const label = document.querySelector("#l") as HTMLLabelElement;
    expect(isFillable(el, makeEnv({ elementAt: () => label }))).toBe(true);
    const other = document.createElement("label");
    expect(isFillable(el, makeEnv({ elementAt: () => other }))).toBe(false);
  });
});

describe("isElementVisible", () => {
  it("is the same test without the position and hit checks", () => {
    const env = makeEnv({ box: () => box(-9999, 100, 200, 30), elementAt: () => null });
    expect(isElementVisible(field(), env)).toBe(true);
    expect(isElementVisible(field(), makeEnv({ opacity: () => 0.1 }))).toBe(false);
  });
});
