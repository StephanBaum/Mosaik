import { describe, expect, it } from "vitest";
import { shouldWipe } from "./intercept.ts";

const click = { button: 0, metaKey: false, ctrlKey: false, shiftKey: false, altKey: false, detail: 1, defaultPrevented: false };
const link = { href: "https://mosaic.example/features/", target: "", hasDownload: false, isNav: true };
const here = { origin: "https://mosaic.example", pathname: "/" };

describe("shouldWipe", () => {
  it("wipes a plain pointer click to another page", () => {
    expect(shouldWipe(click, link, here)).toBe(true);
  });
  it("wipes an explicit _self target", () => {
    expect(shouldWipe(click, { ...link, target: "_self" }, here)).toBe(true);
  });
  it.each<[string, Partial<typeof click>, Partial<typeof link>]>([
    ["ctrl", { ctrlKey: true }, {}],
    ["meta", { metaKey: true }, {}],
    ["shift", { shiftKey: true }, {}],
    ["alt", { altKey: true }, {}],
    ["middle", { button: 1 }, {}],
    ["keyboard", { detail: 0 }, {}],
    ["prevented", { defaultPrevented: true }, {}],
    ["blank", {}, { target: "_blank" }],
    ["download", {}, { hasDownload: true }],
    ["not nav", {}, { isNav: false }],
    ["other origin", {}, { href: "https://github.com/x" }],
    ["same page", {}, { href: "https://mosaic.example/#main" }],
    ["unparsable", {}, { href: "http://[bad" }],
  ])("never wipes on %s", (_n, e, a) => {
    expect(shouldWipe({ ...click, ...e }, { ...link, ...a }, here)).toBe(false);
  });
});
