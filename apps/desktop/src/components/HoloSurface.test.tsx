import { render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HoloSurface } from "./HoloSurface.tsx";
import { HoloProvider } from "../holo/HoloProvider.tsx";
import { SurfaceSet, SURFACE_LIVE } from "../holo/surfaces.ts";

describe("HoloSurface without a provider", () => {
  it("renders a decorative canvas that stays flat", () => {
    const { container, unmount } = render(<HoloSurface className="x" />);
    const canvas = container.querySelector("canvas");
    expect(canvas?.getAttribute("aria-hidden")).toBe("true");
    expect(canvas?.className).toBe("holo-surface x");
    expect(canvas?.classList.contains(SURFACE_LIVE)).toBe(false);
    expect(container.textContent).toBe("");
    expect(() => {
      unmount();
    }).not.toThrow();
  });

  it("stays flat inside a provider without WebGL2 (jsdom)", () => {
    const { container } = render(
      <HoloProvider dark={false}>
        <HoloSurface />
      </HoloProvider>,
    );
    expect(container.querySelector("canvas")?.classList.contains(SURFACE_LIVE)).toBe(false);
  });
});

describe("SurfaceSet", () => {
  function setup(live: boolean) {
    const source = document.createElement("canvas");
    source.width = 40;
    source.height = 40;
    const context = { clearRect: vi.fn(), drawImage: vi.fn() };
    vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
      context as unknown as CanvasRenderingContext2D,
    );
    const draw = vi.fn(() => source);
    let dark = false;
    const set = new SurfaceSet({ live: () => live, dark: () => dark, dpr: () => 2, draw });
    const canvas = document.createElement("canvas");
    Object.defineProperty(canvas, "clientWidth", { value: 10 });
    Object.defineProperty(canvas, "clientHeight", { value: 20 });
    return { set, canvas, draw, context, setDark: (value: boolean) => (dark = value) };
  }

  it("paints once, caches per size and theme, and repaints on a theme change", () => {
    const { set, canvas, draw, context, setDark } = setup(true);
    set.register(canvas);
    expect(canvas.classList.contains(SURFACE_LIVE)).toBe(true);
    expect([canvas.width, canvas.height]).toEqual([20, 40]);
    set.repaintAll();
    expect(draw).toHaveBeenCalledOnce();
    setDark(true);
    set.repaintAll();
    expect(draw).toHaveBeenCalledTimes(2);
    expect(context.drawImage).toHaveBeenCalled();
  });

  it("stays flat when not live and drops the live class on context loss", () => {
    const flat = setup(false);
    flat.set.register(flat.canvas);
    expect(flat.canvas.classList.contains(SURFACE_LIVE)).toBe(false);
    expect(flat.draw).not.toHaveBeenCalled();
    const live = setup(true);
    live.set.register(live.canvas);
    live.set.dropLive();
    expect(live.canvas.classList.contains(SURFACE_LIVE)).toBe(false);
  });
});
