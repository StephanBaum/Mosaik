// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { createPanelRings, ringBand } from "./panelRings.ts";

describe("panel rings", () => {
  it("peaks on the ring and fades outside", () => {
    expect(ringBand(100, 100, 9, 70)).toBeCloseTo(1.25);
    expect(ringBand(109, 100, 9, 70)).toBeCloseTo(Math.exp(-1));
    expect(ringBand(200, 100, 9, 70)).toBeLessThan(1e-6);
  });

  it("creates no canvas until a panel is pressed, and ignores presses while motion is off", () => {
    document.body.innerHTML = '<div class="card">x</div>';
    const rings = createPanelRings(document);
    expect(document.querySelector("canvas.panel-fx")).toBeNull();
    document.documentElement.dataset["motion"] = "off";
    document.querySelector(".card")?.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true, button: 0 }));
    expect(document.querySelector("canvas.panel-fx")).toBeNull();
    delete document.documentElement.dataset["motion"];
    rings.dispose();
  });
});
