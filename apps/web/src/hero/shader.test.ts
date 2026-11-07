import { describe, expect, it } from "vitest";
import { CELLS, FRAG, oddEvenMergeNetwork } from "./shader.ts";

describe("odd-even merge network", () => {
  const network = oddEvenMergeNetwork(CELLS);

  it("sorts every 0-1 input, so it sorts every input (0-1 principle)", () => {
    for (let mask = 0; mask < 2 ** CELLS; mask++) {
      const cells = Array.from({ length: CELLS }, (_, i) => (mask >> i) & 1);
      for (const [a, b] of network) {
        if ((cells[a] ?? 0) > (cells[b] ?? 0)) {
          [cells[a], cells[b]] = [cells[b] ?? 0, cells[a] ?? 0];
        }
      }
      expect(cells.every((v, i) => i === 0 || (cells[i - 1] ?? 0) <= v)).toBe(true);
    }
  });

  it("is Batcher's size for 16 inputs and lands in the shader", () => {
    expect(network).toHaveLength(63);
    expect(FRAG.match(/CX\(\d+, \d+\)/g)).toHaveLength(63);
  });
});
