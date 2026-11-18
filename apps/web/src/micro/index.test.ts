// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from "vitest";
import { MOTION_EVENT } from "../runtime/motion.ts";
import { angleTo, bootMicro } from "./index.ts";

function pointer(type: string, init: { x: number; y: number; kind?: string }): Event {
  const event = new Event(type);
  Object.assign(event, { clientX: init.x, clientY: init.y, pointerType: init.kind ?? "mouse" });
  return event;
}

const rect = (el: Element): void => {
  el.getBoundingClientRect = () => ({ left: 100, top: 100, width: 200, height: 40, right: 300, bottom: 140, x: 100, y: 100, toJSON: () => ({}) });
};

describe("angleTo", () => {
  const box = { left: 0, top: 0, width: 100, height: 100 };
  it("measures clockwise from the top", () => {
    expect(angleTo(box, 50, 0)).toBeCloseTo(0);
    expect(angleTo(box, 100, 50)).toBeCloseTo(90);
    expect(angleTo(box, 50, 100)).toBeCloseTo(180);
    expect(angleTo(box, 0, 50)).toBeCloseTo(270);
  });
});

describe("bootMicro", () => {
  beforeEach(() => {
    document.body.innerHTML = '<a class="button--line" href="/">x</a><a class="card" href="/">y</a><ul class="site-nav__links"><li><a href="/">z</a></li></ul>';
  });

  it("follows the pointer with a glow position and starts the rim from the entry side", () => {
    const ring = document.querySelector<HTMLElement>(".button--line");
    if (ring === null) {
      throw new Error("missing");
    }
    rect(ring);
    bootMicro(document);
    ring.dispatchEvent(pointer("pointerenter", { x: 300, y: 120 }));
    expect(ring.style.getPropertyValue("--bl-x")).toBe("200.0px");
    expect(ring.style.getPropertyValue("--bl-y")).toBe("20.0px");
    expect(Number(ring.style.getPropertyValue("--bl-start"))).toBeCloseTo(90);
  });

  it("ignores touch pointers", () => {
    const card = document.querySelector<HTMLElement>(".card");
    if (card === null) {
      throw new Error("missing");
    }
    rect(card);
    bootMicro(document);
    card.dispatchEvent(pointer("pointerenter", { x: 150, y: 120, kind: "touch" }));
    expect(card.style.getPropertyValue("--bl-x")).toBe("");
  });

  it("grows the nav underline from the side the pointer came from", () => {
    const link = document.querySelector<HTMLElement>(".site-nav__links a");
    if (link === null) {
      throw new Error("missing");
    }
    rect(link);
    bootMicro(document);
    link.dispatchEvent(pointer("pointerenter", { x: 290, y: 110 }));
    expect(link.style.getPropertyValue("--ul-pos")).toBe("100%");
    link.dispatchEvent(pointer("pointerenter", { x: 110, y: 110 }));
    expect(link.style.getPropertyValue("--ul-pos")).toBe("0%");
  });

  it("stops listening when motion is switched off", () => {
    const card = document.querySelector<HTMLElement>(".card");
    if (card === null) {
      throw new Error("missing");
    }
    rect(card);
    bootMicro(document);
    document.dispatchEvent(new CustomEvent(MOTION_EVENT, { detail: { off: true } }));
    card.dispatchEvent(pointer("pointerenter", { x: 150, y: 120 }));
    expect(card.style.getPropertyValue("--bl-x")).toBe("");
  });
});
