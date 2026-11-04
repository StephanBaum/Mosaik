import { afterEach, describe, expect, it } from "vitest";
import { markFromPointer, resolveOrigin, type PointerMark } from "./origin.ts";

function rect(left: number, top: number, width: number, height: number): DOMRect {
  return new DOMRect(left, top, width, height);
}

let mounted: HTMLElement[] = [];

function mount(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  document.body.append(root);
  mounted.push(root);
  return root;
}

function place(root: HTMLElement, selector: string, r: DOMRect): void {
  const el = root.querySelector(selector);
  if (el === null) {
    throw new Error(`no ${selector}`);
  }
  el.getBoundingClientRect = () => r;
}

afterEach(() => {
  for (const el of mounted) {
    el.remove();
  }
  mounted = [];
});

const stageRect = rect(0, 0, 1000, 600);

describe("markFromPointer", () => {
  const html = `
    <div data-reveal-layer="unlock"><button id="unlock"><span id="label">Unlock</span></button><p id="text">text</p></div>
    <div data-reveal-layer="vault"><button id="lock" data-reveal-origin="lock">Lock</button><button id="other">Other</button></div>`;

  it("marks a press on a button of the unlock screen, at the pointer", () => {
    const root = mount(html);
    const mark = markFromPointer({ clientX: 120, clientY: 340, target: root.querySelector("#label") }, root, 5);
    expect(mark).toEqual({ x: 120, y: 340, at: 5, layer: "unlock", lockButton: false });
  });

  it("marks a press on the lock button only", () => {
    const root = mount(html);
    const press = (selector: string) =>
      markFromPointer({ clientX: 1, clientY: 2, target: root.querySelector(selector) }, root, 0);
    expect(press("#lock")?.lockButton).toBe(true);
    expect(press("#other")).toBeNull();
    expect(press("#text")).toBeNull();
  });
});

describe("resolveOrigin", () => {
  const mark = (patch: Partial<PointerMark>): PointerMark => ({
    x: 10,
    y: 20,
    at: 1000,
    layer: "unlock",
    lockButton: false,
    ...patch,
  });

  it("unlock: uses a fresh press, even a slow quick-unlock one", () => {
    const layer = mount("<div></div>");
    const args = { dir: "unlock", layer, stageRect } as const;
    expect(resolveOrigin({ ...args, mark: mark({}), now: 1000 + 29_000 })).toEqual([10, 20]);
    expect(resolveOrigin({ ...args, mark: mark({}), now: 1000 + 31_000 })).toEqual([500, 300]);
  });

  it("unlock: falls back to the centre of the unlock control, then of the stage", () => {
    const layer = mount(`<button data-reveal-origin="unlock">Unlock</button>`);
    place(layer, "button", rect(400, 500, 200, 50));
    const args = { dir: "unlock", layer, stageRect, now: 0, mark: null } as const;
    expect(resolveOrigin(args)).toEqual([500, 525]);
    expect(resolveOrigin({ ...args, layer: mount("<div></div>") })).toEqual([500, 300]);
  });

  it("lock: uses a press on the lock button for two seconds, then the button centre", () => {
    const layer = mount(`<button data-reveal-origin="lock">Lock</button>`);
    place(layer, "button", rect(20, 540, 40, 40));
    const args = { dir: "lock", layer, stageRect } as const;
    const press = mark({ layer: "vault", lockButton: true });
    expect(resolveOrigin({ ...args, mark: press, now: 1500 })).toEqual([10, 20]);
    expect(resolveOrigin({ ...args, mark: press, now: 4000 })).toEqual([40, 560]);
    expect(resolveOrigin({ ...args, mark: mark({ layer: "unlock" }), now: 1100 })).toEqual([40, 560]);
  });
});
