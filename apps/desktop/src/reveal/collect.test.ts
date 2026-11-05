import { afterEach, describe, expect, it } from "vitest";
import { GROUP_SELECTOR, POP_SELECTOR } from "./choreography.ts";
import { collect, MASK_ATTR, SECRET_ATTR } from "./collect.ts";
import type { Measure } from "./ops.ts";

const measure: Measure = {
  rect: () => ({ x: 0, y: 0, w: 1000, h: 1000 }),
  textRects: () => [{ x: 10, y: 20, w: 50, h: 14 }],
};

let mounted: HTMLElement[] = [];

function mount(html: string): HTMLElement {
  const root = document.createElement("div");
  root.innerHTML = html;
  document.body.append(root);
  mounted.push(root);
  return root;
}

afterEach(() => {
  for (const el of mounted) {
    el.remove();
  }
  mounted = [];
});

describe("collect: secrets stay out of the picture", () => {
  it("never paints a secret, an input value, a dialog or hidden text", () => {
    const style = document.createElement("style");
    style.textContent =
      ".visually-hidden { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }";
    document.head.append(style);
    const root = mount(`
      <div>
        <input type="password" value="hunter2">
        <input type="text" value="hunter2" placeholder="Search">
        <textarea>hunter2</textarea>
        <span data-secret data-mask="•••">hunter2</span>
        <output data-secret>hunter2</output>
        <dialog open>hunter2</dialog>
        <span class="visually-hidden">hunter2</span>
        <span>GitHub</span>
      </div>`);
    const json = JSON.stringify(collect(root, measure, { groups: false }).base);
    style.remove();
    expect(json).not.toContain("hunter2");
    expect(json).toContain("GitHub");
    expect(json).toContain("•••");
    // The typed value is not there, so neither is a placeholder in its place.
    expect(json).not.toContain("Search");
  });

  it("paints a placeholder only while the field is empty", () => {
    const root = mount('<input type="text" value="" placeholder="Search">');
    expect(JSON.stringify(collect(root, measure, { groups: false }).base)).toContain("Search");
  });

  it("never descends into a secret", () => {
    const root = mount('<div data-secret data-mask="***"><span>nested-secret</span></div>');
    const json = JSON.stringify(collect(root, measure, { groups: false }).base);
    expect(json).not.toContain("nested-secret");
    expect(json).toContain("***");
  });

  it("paints no live canvas for a preview frame inside a secret", () => {
    const root = mount(
      '<div><div data-secret><canvas width="4" height="4"></canvas></div><canvas width="4" height="4"></canvas></div>',
    );
    const canvases = root.querySelectorAll("canvas");
    const live = collect(root, measure, { groups: false }).base.filter((op) => op.k === "live");
    // The plain canvas is a live op; the one in the secret frame is never visited.
    expect(live).toHaveLength(1);
    expect(live.some((op) => op.el === canvases[0])).toBe(false);
  });

  it("uses the mask attribute names the components write", () => {
    expect(SECRET_ATTR).toBe("data-secret");
    expect(MASK_ATTR).toBe("data-mask");
  });
});

describe("collect: groups", () => {
  it("opens a group for each selector match, with pops under their parent", () => {
    const root = mount(`
      <nav class="sidebar"><button class="nav-item">Home</button><div class="sidebar__footer"><button>Lock</button></div></nav>
      <div class="item-row"><span class="avatar">A</span><span>GitHub</span></div>`);
    const { groups } = collect(root, measure, { groups: true });
    expect(groups.map((g) => g.el.className || g.el.tagName)).toEqual(["nav-item", "BUTTON", "item-row", "avatar"]);
    const row = groups.findIndex((g) => g.el.classList.contains("item-row"));
    const avatar = groups.find((g) => g.el.classList.contains("avatar"));
    expect(avatar?.kind).toBe("pop");
    expect(avatar?.parent).toBe(row);
    expect(groups[row]?.kind).toBe("item");
    // The text of a group lands in the group, not in the base.
    expect(JSON.stringify(groups[row]?.ops)).toContain("GitHub");
  });

  it("finds no groups unless asked", () => {
    const root = mount('<div class="item-row">GitHub</div>');
    expect(collect(root, measure, { groups: false }).groups).toEqual([]);
  });

  it("marks the group that holds the lock control as the origin", () => {
    const root = mount('<div class="sidebar__footer"><button data-reveal-origin="lock">Lock</button></div>');
    expect(collect(root, measure, { groups: true }).groups[0]?.origin).toBe(true);
  });

  it("keeps the selectors the choreography uses", () => {
    expect(GROUP_SELECTOR).toContain(".item-row");
    expect(POP_SELECTOR).toBe(".item-row .avatar");
  });
});
