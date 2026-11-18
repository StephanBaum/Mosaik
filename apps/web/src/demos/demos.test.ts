// @vitest-environment jsdom
import { readdirSync, readFileSync } from "node:fs";
import { fileURLToPath, URL as NodeUrl } from "node:url";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { manualTicker } from "../anim/testTicker.ts";
import type { Ticker } from "../anim/ticker.ts";
import { context } from "../pages/i18n.ts";
import { AgentDemo } from "../pages/demos/AgentDemo.tsx";
import { CliDemo } from "../pages/demos/CliDemo.tsx";
import { FillDemo } from "../pages/demos/FillDemo.tsx";
import { ImportDemo } from "../pages/demos/ImportDemo.tsx";
import { NextSteps } from "../pages/demos/NextSteps.tsx";
import { PhoneDemo } from "../pages/demos/PhoneDemo.tsx";
import { SyncDemo } from "../pages/demos/SyncDemo.tsx";
import { CipherMock } from "../pages/views/mocks.tsx";
import { setMotionOff } from "../runtime/motion.ts";
import { bootDemos, DEMOS } from "./bootDemos.ts";
import { useClock } from "./text.ts";

/** Every script of the demos chunk, with its text. */
function demoSources(): (readonly [string, string])[] {
  const dir = fileURLToPath(new NodeUrl(".", import.meta.url));
  return readdirSync(dir)
    .filter((f) => f.endsWith(".ts") && !f.endsWith(".test.ts"))
    .map((f) => [f, readFileSync(`${dir}${f}`, "utf8")] as const);
}

const ctx = context("en");
const MARKUP = {
  fill: () => createElement(FillDemo, { ctx }),
  sync: () => createElement(SyncDemo, { ctx }),
  agent: () => createElement(AgentDemo, { ctx }),
  import: () => createElement(ImportDemo, { ctx }),
  phone: () => createElement(PhoneDemo, { ctx }),
  cli: () => createElement(CliDemo, { ctx, withTitle: true }),
  cipher: () => createElement(CipherMock, { ctx }),
  nextSteps: () => createElement(NextSteps, { ctx }),
} as const;
const ALL = Object.keys(MARKUP) as (keyof typeof MARKUP)[];

function mount(name: keyof typeof MARKUP): HTMLElement {
  document.body.innerHTML = renderToStaticMarkup(MARKUP[name]());
  const root = document.body.querySelector<HTMLElement>("[data-demo]");
  if (root === null) {
    throw new Error(`no demo root for ${name}`);
  }
  return root;
}

/** The DOM as a static page would show it: inline styles are the timeline's, not the markup's. */
const normalise = (html: string): string => html.replace(/ style="[^"]*"/g, "");

/** Empty when equal, else a short window around the first difference. */
function diff(a: string, b: string): string {
  let i = 0;
  while (i < a.length && a[i] === b[i]) {
    i += 1;
  }
  return i === a.length && i === b.length ? "" : `at ${String(i)}: ${a.slice(Math.max(0, i - 60), i + 80)} <> ${b.slice(Math.max(0, i - 60), i + 80)}`;
}

function countingTicker(): Ticker & { step(dt: number): void; readonly count: number } {
  const inner = manualTicker();
  let count = 0;
  return {
    add(fn) {
      count += 1;
      const off = inner.add(fn);
      return () => {
        count -= 1;
        off();
      };
    },
    step: (dt) => {
      inner.step(dt);
    },
    get count() {
      return count;
    },
  };
}

class FakeObserver {
  static all: FakeObserver[] = [];
  readonly targets: Element[] = [];
  constructor(private readonly cb: IntersectionObserverCallback) {
    FakeObserver.all.push(this);
  }
  observe(el: Element): void {
    this.targets.push(el);
  }
  unobserve(el: Element): void {
    this.targets.splice(this.targets.indexOf(el), 1);
  }
  disconnect(): void {
    this.targets.length = 0;
  }
  show(): void {
    this.cb(
      this.targets.map((target) => ({ target, isIntersecting: true }) as IntersectionObserverEntry),
      this as unknown as IntersectionObserver,
    );
  }
}

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute("data-motion");
  FakeObserver.all = [];
  window.matchMedia = (() => ({ matches: false })) as unknown as typeof window.matchMedia;
});

afterEach(() => {
  useClock(undefined);
});

describe("interface demos", () => {
  it.each(ALL)("%s builds a bounded story that ends on its static markup", (name) => {
    const root = mount(name);
    const before = root.innerHTML;
    const tl = DEMOS[name]?.(root);
    expect(tl).not.toBeNull();
    expect(tl?.duration).toBeGreaterThan(1);
    expect(tl?.duration).toBeLessThan(20);
    tl?.progress(1);
    expect(diff(normalise(root.innerHTML), normalise(before))).toBe("");
  });

  it.each(ALL)("%s stops after three loops on the final frame and frees the ticker", (name) => {
    const clock = countingTicker();
    useClock(clock);
    const root = mount(name);
    const before = root.innerHTML;
    const tl = DEMOS[name]?.(root);
    if (tl === null || tl === undefined) {
      throw new Error("no timeline");
    }
    tl.progress(0);
    tl.play();
    expect(clock.count).toBe(1);
    const seconds = tl.duration * 4 + 20;
    for (let t = 0; t < seconds; t += 1 / 30) {
      clock.step(1 / 30);
    }
    expect(tl.loops).toBeLessThanOrEqual(3);
    expect(clock.count).toBe(0);
    expect(diff(normalise(root.innerHTML), normalise(before))).toBe("");
  });

  it("shows final frames without motion and runs no clock", () => {
    const clock = countingTicker();
    useClock(clock);
    setMotionOff(true, localStorage, document);
    document.body.innerHTML = ALL.map((name) => renderToStaticMarkup(MARKUP[name]())).join("");
    const before = normalise(document.body.innerHTML);
    bootDemos(document, FakeObserver as unknown as typeof IntersectionObserver);
    expect(diff(normalise(document.body.innerHTML), before)).toBe("");
    expect(clock.count).toBe(0);
    expect(FakeObserver.all).toHaveLength(0);
  });

  it("builds near the viewport, starts from the first frame, plays while visible and rests when motion is switched off", () => {
    const clock = countingTicker();
    useClock(clock);
    document.body.innerHTML = renderToStaticMarkup(MARKUP.sync());
    const before = normalise(document.body.innerHTML);
    bootDemos(document, FakeObserver as unknown as typeof IntersectionObserver);
    const [player, builder] = FakeObserver.all;
    // Nothing is built (or measured) until the demo nears the viewport.
    expect(normalise(document.body.innerHTML)).toBe(before);
    builder?.show();
    expect(normalise(document.body.innerHTML)).not.toBe(before);
    expect(builder?.targets).toHaveLength(0);
    expect(clock.count).toBe(0);
    player?.show();
    expect(clock.count).toBe(1);
    clock.step(1.2);
    setMotionOff(true, localStorage, document);
    expect(diff(normalise(document.body.innerHTML), before)).toBe("");
    expect(clock.count).toBe(0);
  });

  it("never writes HTML strings", () => {
    const sources = demoSources();
    expect(sources.length).toBeGreaterThanOrEqual(11);
    for (const [file, text] of sources) {
      expect(text, file).not.toMatch(/innerHTML|outerHTML|insertAdjacentHTML/);
    }
  });

  it("keeps the demo values fixed: no randomness anywhere", () => {
    for (const [file, text] of demoSources()) {
      expect(text, file).not.toMatch(/Math.random|crypto./);
    }
  });
});
