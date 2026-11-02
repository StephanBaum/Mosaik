import { describe, expect, it } from "vitest";
import { splitRuns } from "./generatorRuns.ts";

describe("splitRuns", () => {
  it("groups letters and non-letters into alternating runs", () => {
    expect(splitRuns("ab12cd!!e")).toEqual([
      { text: "ab", minor: false },
      { text: "12", minor: true },
      { text: "cd", minor: false },
      { text: "!!", minor: true },
      { text: "e", minor: false },
    ]);
  });

  it("treats non-ASCII letters as letters and keeps the text intact", () => {
    const runs = splitRuns("Äpfel-7ß");
    expect(runs.map((run) => run.text).join("")).toBe("Äpfel-7ß");
    expect(runs[0]).toEqual({ text: "Äpfel", minor: false });
    expect(runs[1]).toEqual({ text: "-7", minor: true });
  });

  it("returns nothing for an empty string", () => {
    expect(splitRuns("")).toEqual([]);
  });
});
