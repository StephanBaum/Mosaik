/**
 * The self-check's questions, scoring and next steps (website v2 §2.1). Plain data, shared by
 * the page (build time) and the island (browser), so no React reaches the client.
 */
export type TipId = "twofa" | "reuse" | "store" | "leak" | "create" | "browser" | "many" | "review" | "keep";

/** One answer: its label, its points (higher is more exposed) and the next step it calls for. */
type Option = readonly [suffix: string, points: number, tip?: TipId];

export const QUESTIONS: readonly (readonly [n: number, options: readonly Option[]])[] = [
  [1, [["a", 0], ["b", 1, "many"], ["c", 1, "many"], ["d", 1, "many"]]],
  [2, [["a", 0], ["b", 1, "reuse"], ["c", 3, "reuse"], ["d", 3, "reuse"]]],
  [3, [["a", 0], ["b", 1, "browser"], ["c", 2, "store"], ["d", 2, "store"]]],
  [4, [["a", 0], ["b", 1, "twofa"], ["c", 3, "twofa"], ["d", 2, "twofa"]]],
  [5, [["a", 0], ["b", 2, "leak"], ["c", 2, "leak"]]],
  [6, [["a", 0], ["b", 1], ["c", 2, "create"], ["d", 3, "create"]]],
  [7, [["a", 0], ["b", 1, "review"], ["c", 1, "review"]]],
];

/** Next steps in priority order; the result shows the first three that apply. */
export const TIPS: readonly TipId[] = ["twofa", "reuse", "store", "leak", "create", "browser", "many", "review", "keep"];

/** Score bands: up to 3 points is low, up to 8 mid, above that high (maximum 15). */
export const LEVELS = [
  ["low", 3],
  ["mid", 8],
  ["high", Number.POSITIVE_INFINITY],
] as const;

export function levelFor(score: number): (typeof LEVELS)[number][0] {
  return (LEVELS.find(([, max]) => score <= max) ?? LEVELS[2])[0];
}
