import { describe, expect, it } from "vitest";
import { detectPlatform } from "./platform.ts";

describe("detectPlatform", () => {
  it.each([
    ["Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/126.0 Edg/126.0", "windows"],
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)", "mac"],
    ["Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0", "linux"],
  ] as const)("%s", (ua, expected) => {
    expect(detectPlatform(ua)).toBe(expected);
  });
});
