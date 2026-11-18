import { describe, expect, it } from "vitest";
import { fxTier, type CapabilityEnv, type FxTier } from "./capability.ts";

const base: CapabilityEnv = {
  reducedMotion: false,
  motionOff: false,
  saveData: false,
  effectiveType: "4g",
  cores: 8,
  memoryGb: 8,
  coarsePointer: false,
  viewportWidth: 1440,
  webgl2: true,
};

describe("fxTier", () => {
  it.each<[Partial<CapabilityEnv>, FxTier]>([
    [{}, "full"],
    [{ reducedMotion: true }, "none"],
    [{ motionOff: true }, "none"],
    [{ saveData: true }, "none"],
    [{ effectiveType: "2g" }, "none"],
    [{ effectiveType: "slow-2g" }, "none"],
    [{ webgl2: false }, "none"],
    [{ cores: 2 }, "none"],
    [{ memoryGb: 1 }, "none"],
    [{ effectiveType: "3g" }, "lite"],
    [{ coarsePointer: true }, "lite"],
    [{ cores: 4 }, "lite"],
    [{ memoryGb: 4 }, "lite"],
    [{ viewportWidth: 699 }, "lite"],
    [{ cores: undefined, memoryGb: undefined }, "full"],
    [{ coarsePointer: true, reducedMotion: true }, "none"],
  ])("%o gives %s", (patch, tier) => {
    expect(fxTier({ ...base, ...patch })).toBe(tier);
  });
});
