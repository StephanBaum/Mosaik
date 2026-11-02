import { describe, expect, it } from "vitest";
import { generatorHandlers, WORDS } from "./generator.ts";
import type { MockState } from "./state.ts";

function state(): MockState {
  const partial: Partial<MockState> = { lastGenerated: null };
  return partial as MockState;
}

function phrase(appendDigit: boolean): number {
  const result = generatorHandlers.generate_password?.(state(), {
    kind: "passphrase",
    words: 5,
    separator: "-",
    capitalize: false,
    appendDigit,
  });
  return result?.entropyBits ?? Number.NaN;
}

describe("mock generator", () => {
  it("has a list of distinct words", () => {
    expect(new Set(WORDS).size).toBe(WORDS.length);
    expect(WORDS.length).toBeGreaterThanOrEqual(64);
  });

  it("computes entropy from the list actually used", () => {
    expect(phrase(false)).toBeCloseTo(5 * 6);
    expect(phrase(true)).toBeCloseTo(30 + Math.log2(10));
  });
});
