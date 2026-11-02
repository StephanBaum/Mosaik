import { afterEach, describe, expect, it, vi } from "vitest";
import { focusRunning, rippleRunning } from "./effects.ts";
import { attachEvents } from "./events.ts";
import { newState } from "./motion.ts";

function setup(options: { reduced?: boolean; focusVisible?: boolean } = {}) {
  const button = document.createElement("button");
  document.body.append(button);
  button.getBoundingClientRect = () => new DOMRect(100, 50, 120, 40);
  vi.spyOn(button, "matches").mockReturnValue(options.focusVisible === true);
  const state = newState();
  attachEvents(button, state, {
    poke: vi.fn(),
    now: () => 0,
    reduced: () => options.reduced === true,
  });
  return { button, state };
}

afterEach(() => {
  document.body.replaceChildren();
});

describe("press ripple trigger", () => {
  it("starts at the pointer-down point in button heights from the centre", () => {
    const { button, state } = setup();
    button.dispatchEvent(new MouseEvent("pointerdown", { button: 0, clientX: 190, clientY: 60 }));
    expect(rippleRunning(state.ripAge)).toBe(true);
    expect(state.ripX).toBeCloseTo((190 - 160) / 40, 10);
    expect(state.ripY).toBeCloseTo(-(60 - 70) / 40, 10);
    expect(button.classList.contains("holo--pressed")).toBe(true);
  });

  it("starts at the centre for a keyboard press, once per press", () => {
    const { button, state } = setup();
    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter" }));
    expect([state.ripX, state.ripY]).toEqual([0, 0]);
    expect(state.ripAge).toBe(0);
    state.ripAge = 0.5;
    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", repeat: true }));
    expect(state.ripAge).toBe(0.5);
  });

  it("does nothing for the secondary mouse button or under reduced motion", () => {
    const first = setup();
    first.button.dispatchEvent(new MouseEvent("pointerdown", { button: 2 }));
    expect(rippleRunning(first.state.ripAge)).toBe(false);
    const second = setup({ reduced: true });
    second.button.dispatchEvent(new MouseEvent("pointerdown", { button: 0 }));
    second.button.dispatchEvent(new KeyboardEvent("keydown", { key: " " }));
    expect(rippleRunning(second.state.ripAge)).toBe(false);
  });
});

describe("focus waves trigger", () => {
  it("runs on keyboard focus (focus-visible)", () => {
    const { button, state } = setup({ focusVisible: true });
    button.dispatchEvent(new FocusEvent("focus"));
    expect(focusRunning(state.focusAge)).toBe(true);
    expect(state.focusT).toBe(1);
  });

  it("never runs on mouse focus", () => {
    const { button, state } = setup({ focusVisible: false });
    button.dispatchEvent(new FocusEvent("focus"));
    expect(focusRunning(state.focusAge)).toBe(false);
    expect(state.focusT).toBe(0);
  });

  it("never runs under reduced motion", () => {
    const { button, state } = setup({ focusVisible: true, reduced: true });
    button.dispatchEvent(new FocusEvent("focus"));
    expect(focusRunning(state.focusAge)).toBe(false);
  });
});
