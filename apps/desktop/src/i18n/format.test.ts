import { describe, expect, it } from "vitest";
import { formatBytes, formatRelative } from "./format.ts";

const NOW = 1_800_000_000_000;
const DAY = 86_400_000;

describe("formatRelative", () => {
  it("formats days in both locales", () => {
    expect(formatRelative("en", NOW - 3 * DAY, NOW)).toBe("3 days ago");
    expect(formatRelative("de", NOW - 3 * DAY, NOW)).toBe("vor 3 Tagen");
  });

  it("says now for the current second", () => {
    expect(formatRelative("en", NOW, NOW)).toBe("now");
    expect(formatRelative("de", NOW, NOW)).toBe("jetzt");
  });

  it("picks coarser units as the gap grows", () => {
    expect(formatRelative("en", NOW - 5 * 60_000, NOW)).toBe("5 minutes ago");
    expect(formatRelative("en", NOW - 2 * 3_600_000, NOW)).toBe("2 hours ago");
    expect(formatRelative("en", NOW - 90 * DAY, NOW)).toBe("3 months ago");
    expect(formatRelative("en", NOW - 800 * DAY, NOW)).toBe("2 years ago");
  });
});

describe("formatBytes", () => {
  it("scales units", () => {
    expect(formatBytes("en", 500)).toMatch(/500\s?byte/);
    expect(formatBytes("en", 1500)).toMatch(/^1\.5\s?kB$/);
    expect(formatBytes("en", 2_500_000)).toMatch(/^2\.5\s?MB$/);
  });
});
