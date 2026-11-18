// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { isMotionOff, MOTION_EVENT, MOTION_KEY, setMotionOff, wireMotionToggle } from "./motion.ts";

afterEach(() => {
  localStorage.clear();
  delete document.documentElement.dataset["motion"];
  document.body.replaceChildren();
});

describe("motion switch", () => {
  it("treats a throwing storage as motion on", () => {
    expect(
      isMotionOff({
        getItem: () => {
          throw new Error("blocked");
        },
      }),
    ).toBe(false);
  });

  it("persists, marks the root and notifies", () => {
    const seen: boolean[] = [];
    document.addEventListener(MOTION_EVENT, (e) => seen.push((e as CustomEvent<{ off: boolean }>).detail.off));
    setMotionOff(true, localStorage, document);
    expect(localStorage.getItem(MOTION_KEY)).toBe("off");
    expect(document.documentElement.dataset["motion"]).toBe("off");
    setMotionOff(false, localStorage, document);
    expect(localStorage.getItem(MOTION_KEY)).toBeNull();
    expect(document.documentElement.dataset["motion"]).toBeUndefined();
    expect(seen).toEqual([true, false]);
  });

  it("still switches the page when storage throws on write", () => {
    const broken = {
      setItem: () => {
        throw new Error("quota");
      },
      removeItem: () => undefined,
    };
    setMotionOff(true, broken, document);
    expect(document.documentElement.dataset["motion"]).toBe("off");
  });

  it("wires the footer button: unhides, toggles label and aria-pressed", () => {
    const button = document.createElement("button");
    button.hidden = true;
    button.dataset["motionToggle"] = "";
    button.dataset["labelPause"] = "Pause";
    button.dataset["labelPlay"] = "Play";
    button.textContent = "Pause";
    document.body.append(button);
    wireMotionToggle(document, localStorage);
    expect(button.hidden).toBe(false);
    expect(button.getAttribute("aria-pressed")).toBe("false");
    button.click();
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(button.textContent).toBe("Play");
    expect(document.documentElement.dataset["motion"]).toBe("off");
    button.click();
    expect(button.textContent).toBe("Pause");
  });
});
