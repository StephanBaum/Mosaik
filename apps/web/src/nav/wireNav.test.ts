// @vitest-environment jsdom
import { describe, expect, it, vi } from "vitest";
import type { VeilApi } from "../runtime/fxRegistry.ts";
import { quietTransition, wireNav } from "./wireNav.ts";

interface Fake {
  win: Window;
  assign: ReturnType<typeof vi.fn>;
  wipeIn: ReturnType<typeof vi.fn>;
  veil: VeilApi;
}

/** A window stand-in: real event wiring, a recordable `location.assign`, motion allowed. */
function fake(matchesReduced = false): Fake {
  const assign = vi.fn();
  const win = {
    location: { origin: "http://localhost:3000", pathname: "/", assign },
    matchMedia: () => ({ matches: matchesReduced }),
    localStorage: window.localStorage,
    addEventListener: window.addEventListener.bind(window),
  } as unknown as Window;
  const wipeIn = vi.fn(() => Promise.resolve());
  return { win, assign, wipeIn, veil: { wipeIn, reset: vi.fn() } };
}

const press = (a: Element, init: MouseEventInit = {}): MouseEvent => {
  const e = new MouseEvent("click", { bubbles: true, cancelable: true, detail: 1, ...init });
  a.dispatchEvent(e);
  return e;
};

/** A fresh document per test, so listeners from earlier tests never see its clicks. */
function freshDoc(): { doc: Document; link: Element } {
  const doc = document.implementation.createHTMLDocument("t");
  doc.body.innerHTML = '<a id="l" data-nav="" href="http://localhost:3000/features/">F</a>';
  return { doc, link: doc.getElementById("l") as Element };
}

describe("wireNav", () => {
  it("wipes, then navigates", async () => {
    const f = fake();
    const { doc, link } = freshDoc();
    wireNav(doc, f.win, () => f.veil);
    const e = press(link);
    expect(e.defaultPrevented).toBe(true);
    expect(f.wipeIn).toHaveBeenCalledWith(0.32);
    await Promise.resolve();
    await Promise.resolve();
    expect(f.assign).toHaveBeenCalledWith("http://localhost:3000/features/");
  });

  it("lets a ctrl-click through", () => {
    const f = fake();
    const { doc, link } = freshDoc();
    wireNav(doc, f.win, () => f.veil);
    const e = press(link, { ctrlKey: true });
    expect(e.defaultPrevented).toBe(false);
    expect(f.wipeIn).not.toHaveBeenCalled();
  });

  it("navigates plainly without a veil", () => {
    const f = fake();
    const { doc, link } = freshDoc();
    wireNav(doc, f.win, () => null);
    expect(press(link).defaultPrevented).toBe(false);
  });

  it("navigates plainly while motion is off", () => {
    const f = fake(true);
    const { doc, link } = freshDoc();
    wireNav(doc, f.win, () => f.veil);
    expect(press(link).defaultPrevented).toBe(false);
    expect(f.wipeIn).not.toHaveBeenCalled();
  });

  it("uncovers a page restored from the back-forward cache", () => {
    const reset = vi.fn();
    wireNav(freshDoc().doc, fake().win, () => ({ wipeIn: vi.fn(() => Promise.resolve()), reset }));
    window.dispatchEvent(new PageTransitionEvent("pageshow", { persisted: true }));
    expect(reset).toHaveBeenCalled();
  });

  it("handles an aborted view transition on both pages, and skips it while motion is off", async () => {
    const unhandled = vi.fn();
    process.on("unhandledRejection", unhandled);
    const aborted = (): Promise<never> => Promise.reject(new DOMException("Transition was aborted because of invalid state", "InvalidStateError"));
    const transition = () => ({ ready: aborted(), finished: aborted(), updateCallbackDone: aborted(), skipTransition: vi.fn() });
    wireNav(freshDoc().doc, fake(true).win, () => null);
    const swap = transition();
    window.dispatchEvent(Object.assign(new Event("pageswap"), { viewTransition: swap }));
    window.dispatchEvent(Object.assign(new Event("pagereveal"), { viewTransition: transition() }));
    quietTransition(transition());
    await new Promise((resolve) => setTimeout(resolve, 0));
    process.off("unhandledRejection", unhandled);
    expect(unhandled).not.toHaveBeenCalled();
    expect(swap.skipTransition).toHaveBeenCalled();
  });
});
