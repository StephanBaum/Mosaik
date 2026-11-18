// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { wireReveals } from "./reveal.ts";

/** An IntersectionObserver the test drives by hand. */
class FakeIO {
  static last: FakeIO | undefined;
  readonly observed: Element[] = [];
  readonly unobserved: Element[] = [];

  constructor(private readonly cb: (entries: IntersectionObserverEntry[]) => void) {
    FakeIO.last = this;
  }

  observe(el: Element): void {
    this.observed.push(el);
  }

  unobserve(el: Element): void {
    this.unobserved.push(el);
  }

  report(...entries: [Element, boolean][]): void {
    this.cb(entries.map(([target, isIntersecting]) => ({ target, isIntersecting }) as IntersectionObserverEntry));
  }
}

const FakeCtor = FakeIO as unknown as typeof IntersectionObserver;

function two(): [HTMLElement, HTMLElement] {
  document.body.innerHTML = "";
  const a = document.createElement("section");
  const b = document.createElement("section");
  a.dataset["reveal"] = "";
  b.dataset["reveal"] = "";
  document.body.append(a, b);
  return [a, b];
}

afterEach(() => {
  document.body.replaceChildren();
  FakeIO.last = undefined;
});

describe("wireReveals", () => {
  it("hides only what starts below the fold", () => {
    const [a, b] = two();
    wireReveals(document, true, FakeCtor);
    FakeIO.last?.report([a, true], [b, false]);
    expect(a.classList.contains("reveal-pending")).toBe(false);
    expect(b.classList.contains("reveal-pending")).toBe(true);
    FakeIO.last?.report([b, true]);
    expect(b.classList.contains("reveal-pending")).toBe(false);
    expect(b.classList.contains("reveal-in")).toBe(true);
    expect(FakeIO.last?.unobserved).toContain(b);
  });

  it("does nothing when motion is not allowed", () => {
    two();
    wireReveals(document, false, FakeCtor);
    expect(FakeIO.last).toBeUndefined();
    expect(document.querySelector(".reveal-pending")).toBeNull();
  });
});
