import { describe, expect, it } from "vitest";
import { daysLeft, TRASH_RETENTION_DAYS } from "./trash.ts";

const DAY = 86_400_000;
const NOW = 1_800_000_000_000;

describe("daysLeft", () => {
  it("is the full retention right after deletion", () => {
    expect(daysLeft(NOW, NOW)).toBe(TRASH_RETENTION_DAYS);
  });

  it("does not exceed the retention when the clock is behind the deletion time", () => {
    expect(daysLeft(NOW + 5000, NOW)).toBe(TRASH_RETENTION_DAYS);
  });

  it("counts whole elapsed days", () => {
    expect(daysLeft(NOW - 10 * DAY - 1000, NOW)).toBe(20);
    expect(daysLeft(NOW - 23 * DAY, NOW)).toBe(7);
  });

  it("never goes below zero", () => {
    expect(daysLeft(NOW - 30 * DAY, NOW)).toBe(0);
    expect(daysLeft(NOW - 90 * DAY, NOW)).toBe(0);
  });
});
