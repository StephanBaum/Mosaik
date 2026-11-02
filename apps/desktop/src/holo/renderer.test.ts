import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HoloGl } from "./gl.ts";
import { createHoloRenderer, type HoloRenderer } from "./renderer.ts";

type Mock = ReturnType<typeof vi.fn>;

interface Harness {
  renderer: HoloRenderer;
  gl: Record<string, Mock>;
  glCanvas: HTMLCanvasElement;
  queue: FrameRequestCallback[];
  flush: () => void;
  context: { clearRect: Mock; drawImage: Mock };
  makeButton: () => { button: HTMLButtonElement; foil: HTMLCanvasElement };
}

function fakeGl(compiles: boolean): Record<string, Mock> {
  const names = [
    "createShader",
    "shaderSource",
    "compileShader",
    "getShaderInfoLog",
    "deleteShader",
    "attachShader",
    "linkProgram",
    "getProgramInfoLog",
    "bindVertexArray",
    "createVertexArray",
    "disable",
    "useProgram",
    "viewport",
    "clearColor",
    "clear",
    "uniform1f",
    "uniform2f",
    "uniform4f",
    "drawArrays",
    "getExtension",
  ];
  const gl: Record<string, Mock> = {};
  for (const name of names) {
    gl[name] = vi.fn();
  }
  gl["createShader"] = vi.fn(() => ({}));
  gl["createProgram"] = vi.fn(() => ({}));
  gl["getShaderParameter"] = vi.fn(() => compiles);
  gl["getProgramParameter"] = vi.fn(() => true);
  gl["getUniformLocation"] = vi.fn((_program: unknown, name: string) => ({ name }));
  gl["getExtension"] = vi.fn(() => null);
  return gl;
}

function setup(options: { compiles?: boolean; reduced?: boolean } = {}): Harness {
  const queue: FrameRequestCallback[] = [];
  const gl = fakeGl(options.compiles ?? true);
  const glCanvas = document.createElement("canvas");
  const context = { clearRect: vi.fn(), drawImage: vi.fn() };
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockReturnValue(
    context as unknown as CanvasRenderingContext2D,
  );
  const renderer = createHoloRenderer({
    createCanvas: () => glCanvas,
    createGl: () => gl as unknown as HoloGl,
    raf: (cb) => queue.push(cb),
    caf: () => undefined,
    now: () => 10_000,
    dpr: () => 2,
    matchMedia: () =>
      ({
        matches: options.reduced === true,
        addEventListener: () => undefined,
        removeEventListener: () => undefined,
      }) as unknown as MediaQueryList,
  });
  return {
    renderer,
    gl,
    glCanvas,
    queue,
    context,
    flush: () => {
      for (const cb of queue.splice(0)) {
        cb(10_000);
      }
    },
    makeButton: () => {
      const button = document.createElement("button");
      Object.defineProperty(button, "offsetWidth", { value: 100 });
      Object.defineProperty(button, "offsetHeight", { value: 34 });
      document.body.append(button);
      return { button, foil: document.createElement("canvas") };
    },
  };
}

let harness: Harness;

afterEach(() => {
  harness.renderer.dispose();
  vi.restoreAllMocks();
  document.body.replaceChildren();
});

describe("holo renderer", () => {
  beforeEach(() => {
    harness = setup();
  });

  it("marks the button live and paints it once, copying into its canvas", () => {
    const { button, foil } = harness.makeButton();
    harness.renderer.register(button, foil);
    expect(button.classList.contains("holo--live")).toBe(true);
    harness.flush();
    expect(harness.gl["drawArrays"]).toHaveBeenCalledOnce();
    expect(harness.context.drawImage).toHaveBeenCalledOnce();
    // 100 x 34 CSS px plus the 8 px margin on each side, at dpr 2.
    expect(foil.width).toBe(232);
    expect(foil.height).toBe(100);
    expect(harness.queue).toHaveLength(0);
  });

  it("falls back to static buttons when the context is lost", () => {
    const { button, foil } = harness.makeButton();
    harness.renderer.register(button, foil);
    const event = new Event("webglcontextlost", { cancelable: true });
    harness.glCanvas.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(true);
    expect(button.classList.contains("holo--live")).toBe(false);
    expect(harness.renderer.live).toBe(false);
  });

  it("stops reacting and removes its classes once unregistered", () => {
    const { button, foil } = harness.makeButton();
    const unregister = harness.renderer.register(button, foil);
    harness.flush();
    unregister();
    expect(button.classList.contains("holo--live")).toBe(false);
    button.dispatchEvent(new MouseEvent("pointerenter"));
    expect(harness.queue).toHaveLength(0);
  });

  it("uses the dark uniform after setDark", () => {
    const { button, foil } = harness.makeButton();
    harness.renderer.register(button, foil);
    harness.flush();
    harness.renderer.setDark(true);
    harness.flush();
    const calls = harness.gl["uniform1f"]?.mock.calls ?? [];
    const last = calls.filter(([loc]) => (loc as { name: string }).name === "uDark").at(-1);
    expect(last?.[1]).toBe(1);
  });
});

function rippleCalls(gl: Record<string, Mock>): number[][] {
  return (gl["uniform4f"]?.mock.calls ?? [])
    .filter(([loc]) => (loc as { name: string }).name === "uRipple")
    .map(([, ...rest]) => rest as number[]);
}

describe("holo renderer effects", () => {
  beforeEach(() => {
    harness = setup();
  });

  it("paints a press ripple from the pointer and keeps a frame queued while it runs", () => {
    const { button, foil } = harness.makeButton();
    harness.renderer.register(button, foil);
    harness.flush();
    button.getBoundingClientRect = () => new DOMRect(0, 0, 100, 34);
    button.dispatchEvent(new MouseEvent("pointerdown", { button: 0, clientX: 80, clientY: 17 }));
    harness.flush();
    harness.flush();
    const live = rippleCalls(harness.gl).filter(([, , , weight]) => (weight ?? 0) > 0);
    expect(live.length).toBeGreaterThan(0);
    expect(live[0]?.[0]).toBeCloseTo((80 - 50) / 34, 5);
    expect(live[0]?.[1]).toBeCloseTo(0, 5);
    expect(harness.queue.length).toBeGreaterThan(0);
  });

  it("starts no ripple under reduced motion", () => {
    harness.renderer.dispose();
    harness = setup({ reduced: true });
    const { button, foil } = harness.makeButton();
    harness.renderer.register(button, foil);
    harness.flush();
    button.dispatchEvent(new MouseEvent("pointerdown", { button: 0 }));
    harness.flush();
    const weights = rippleCalls(harness.gl).map(([, , , weight]) => weight);
    expect(weights.every((weight) => weight === 0)).toBe(true);
  });
});

describe("holo renderer without a working shader", () => {
  it("stays not live and leaves the button alone", () => {
    harness = setup({ compiles: false });
    const { button, foil } = harness.makeButton();
    expect(() => harness.renderer.register(button, foil)).not.toThrow();
    expect(harness.renderer.live).toBe(false);
    expect(button.classList.contains("holo--live")).toBe(false);
  });
});

describe("holo renderer with reduced motion", () => {
  it("repaints once on hover and queues nothing more", () => {
    harness = setup({ reduced: true });
    const { button, foil } = harness.makeButton();
    harness.renderer.register(button, foil);
    harness.flush();
    const before = harness.gl["drawArrays"]?.mock.calls.length ?? 0;
    button.dispatchEvent(new MouseEvent("pointerenter"));
    harness.flush();
    expect((harness.gl["drawArrays"]?.mock.calls.length ?? 0) - before).toBe(1);
    expect(harness.queue).toHaveLength(0);
  });
});
