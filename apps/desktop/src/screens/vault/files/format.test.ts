import { formatMessage } from "@mosaic/i18n";
import { describe, expect, it } from "vitest";
import { formatSize, iconFor } from "./format.ts";

const en = (key: Parameters<typeof formatMessage>[1], values?: Parameters<typeof formatMessage>[2]) =>
  formatMessage("en", key, values);

describe("formatSize", () => {
  it.each([
    [0, "0 B"],
    [812, "812 B"],
    [1023, "1,023 B"],
    [1024, "1 KB"],
    [3482, "3.4 KB"],
    [1024 * 1024 - 1, "1,024 KB"],
    [1_200_000, "1.1 MB"],
    [50 * 1024 * 1024, "50 MB"],
  ])("%d bytes is %s", (bytes, expected) => {
    expect(formatSize(bytes, en, "en")).toBe(expected);
  });

  it("formats the number in the locale", () => {
    expect(formatSize(1536, (key, values) => formatMessage("de", key, values), "de")).toBe("1,5 KB");
  });
});

describe("iconFor", () => {
  it("picks an icon by type", () => {
    expect(iconFor("image/png")).toBe("image");
    expect(iconFor("image/heic")).toBe("image");
    expect(iconFor("application/pdf")).toBe("fileText");
    expect(iconFor("text/plain")).toBe("fileText");
    expect(iconFor("application/zip")).toBe("file");
    expect(iconFor("")).toBe("file");
  });
});
