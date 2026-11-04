import { describe, expect, it } from "vitest";
import { lockFrame, unlockFrame } from "./frames.ts";
import { sceneFor } from "./scene.ts";
import { BAND_PIXEL_BUDGET, GLASS, lockDur } from "./timing.ts";

const scene = sceneFor({ width: 1280, height: 800 }, [300, 200], 1, BAND_PIXEL_BUDGET);

describe("unlockFrame", () => {
  it("clips to a polygon with the band on while the lens grows", () => {
    const f = unlockFrame(0.3, scene);
    expect(f.clip).toMatch(/^polygon\(/);
    expect(f.bandOn).toBe(true);
  });

  it("applies the end state once the lens and its band have left the stage", () => {
    const f = unlockFrame(GLASS.duration, scene);
    expect(f.clip).toBeNull();
    expect(f.bandOn).toBe(false);
  });
});

describe("lockFrame", () => {
  it("clips to a polygon while the lens closes", () => {
    const f = lockFrame(0.5, scene);
    expect(f.clip).toMatch(/^polygon\(/);
    expect(f.gone).toBe(false);
  });

  it("is gone when the lens has closed", () => {
    const f = lockFrame(lockDur(), scene);
    expect(f.gone).toBe(true);
    expect(f.clip).toBeNull();
  });
});
