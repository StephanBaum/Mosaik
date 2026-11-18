// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { MOTION_EVENT } from "../runtime/motion.ts";
import { bootField, type FieldLayer } from "./bootField.ts";
import type { FieldGl } from "./gl.ts";

function queue(): { raf: (cb: FrameRequestCallback) => number; flush(n: number): void; pending(): number } {
  let cbs: FrameRequestCallback[] = [];
  let now = 0;
  return {
    raf(cb) {
      cbs.push(cb);
      return cbs.length;
    },
    flush(n) {
      for (let i = 0; i < n; i++) {
        const run = cbs;
        cbs = [];
        now += 16;
        for (const cb of run) {
          cb(now);
        }
      }
    },
    pending: () => cbs.length,
  };
}

function fakeGl(canvas: HTMLCanvasElement, opts: { dprCap: number }): FieldGl {
  return {
    canvas,
    u: { time: 0, lensX: 0, lensY: 0, lensR: 0, scroll: 0, wipe: 0, page: 0, heroY: 0, cell: 12, clickX: 0, clickY: 0, clickAge: -1 },
    dpr: 1,
    dprCap: opts.dprCap,
    width: 10,
    height: 10,
    resize: () => undefined,
    setAtlas: () => undefined,
    render: vi.fn(),
    ready: () => true,
    lost: () => false,
  };
}

const STILL_HTML = '<div class="field-still"></div>';

afterEach(() => {
  document.body.innerHTML = "";
  delete document.documentElement.dataset["fxState"];
});

describe("bootField", () => {
  it("draws, then marks the canvas on", () => {
    document.body.innerHTML = STILL_HTML;
    const q = queue();
    const h = bootField(document, "lite", { createGl: fakeGl, raf: q.raf });
    q.flush(2);
    expect(h.state).toBe("running");
    expect(document.querySelector("canvas.field")?.classList.contains("is-on")).toBe(true);
    expect(document.documentElement.dataset["fxState"]).toBe("lite");
    h.stop();
  });

  it("drops to the still on context loss", () => {
    document.body.innerHTML = STILL_HTML;
    const q = queue();
    const h = bootField(document, "full", { createGl: fakeGl, raf: q.raf });
    q.flush(2);
    document.querySelector("canvas.field")?.dispatchEvent(new Event("webglcontextlost"));
    q.flush(1);
    expect(h.state).toBe("still");
    expect(document.querySelector(".field-still")?.hasAttribute("hidden")).toBe(false);
    expect(document.documentElement.dataset["fxState"]).toBe("still");
    expect(q.pending()).toBe(0);
  });

  it("stops when the motion toggle turns off", () => {
    document.body.innerHTML = STILL_HTML;
    const q = queue();
    const h = bootField(document, "lite", { createGl: fakeGl, raf: q.raf });
    q.flush(2);
    document.dispatchEvent(new CustomEvent(MOTION_EVENT, { detail: { off: true } }));
    q.flush(1);
    expect(h.state).toBe("still");
    expect(document.querySelector("canvas.field")?.classList.contains("is-on")).toBe(false);
    expect(q.pending()).toBe(0);
  });

  it("returns a still handle when no context can be created", () => {
    document.body.innerHTML = STILL_HTML;
    const q = queue();
    const h = bootField(document, "lite", { createGl: () => null, raf: q.raf });
    expect(h.state).toBe("still");
    expect(document.querySelector("canvas.field")).toBeNull();
  });

  it("disposes a layer added after the field stopped", () => {
    document.body.innerHTML = STILL_HTML;
    const q = queue();
    const h = bootField(document, "full", { createGl: fakeGl, raf: q.raf });
    h.stop();
    const dispose = vi.fn();
    const layer: FieldLayer = { draw: vi.fn(), setClick: vi.fn(), dispose };
    h.addLayer(layer);
    expect(dispose).toHaveBeenCalled();
  });
});
