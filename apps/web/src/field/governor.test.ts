import { describe, expect, it } from "vitest";
import { createGovernor } from "./governor.ts";

describe("governor", () => {
  it("degrades in two steps", () => {
    const g = createGovernor();
    const run = (ms: number, n: number): string => {
      let r = "keep";
      for (let i = 0; i < n; i++) {
        r = g.sample(ms);
      }
      return r;
    };
    expect(run(16, 120)).toBe("keep");
    expect(run(30, 90)).toBe("lite");
    expect(run(50, 90)).toBe("stop");
  });
  it("ignores slow warm-up frames", () => {
    const g = createGovernor();
    for (let i = 0; i < 30; i++) {
      expect(g.sample(400)).toBe("keep");
    }
  });
});
