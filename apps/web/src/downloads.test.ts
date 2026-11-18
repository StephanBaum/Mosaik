import { describe, expect, it } from "vitest";
import { detectPlatform } from "./downloads.ts";

describe("detectPlatform", () => {
  it.each([
    ["Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/140.0 Safari/537.36", "Win32", "windows"],
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 Safari/605.1.15", "MacIntel", "macos"],
    ["Mozilla/5.0 (X11; Linux x86_64; rv:140.0) Gecko/20100101 Firefox/140.0", "Linux x86_64", "linux"],
    ["Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148", "iPhone", "iphone"],
    ["Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 Chrome/140.0 Mobile Safari/537.36", "Linux armv8l", "android"],
  ] as const)("%s", (ua, platform, expected) => {
    expect(detectPlatform(ua, platform)).toBe(expected);
  });

  it("returns null for an unknown system", () => {
    expect(detectPlatform("curl/8.0", "")).toBeNull();
  });
});
