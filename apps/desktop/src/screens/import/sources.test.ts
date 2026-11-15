import { describe, expect, it } from "vitest";
import { catalogs } from "@mosaic/i18n";
import { formatFor, importSources, isMismatch, sourceById } from "./sources.ts";

const none = { source: null, format: null, syncedFolder: null } as const;

describe("import sources", () => {
  it("lists browsers, then managers, then other", () => {
    const groups = importSources.map((s) => s.group);
    expect(groups).toEqual([...groups].sort((a, b) => ["browser", "manager", "other"].indexOf(a) - ["browser", "manager", "other"].indexOf(b)));
    expect(new Set(importSources.map((s) => s.id)).size).toBe(importSources.length);
    expect(importSources).toHaveLength(21);
  });

  it("has every guide step and source name in EN and DE", () => {
    for (const source of importSources) {
      for (const locale of ["en", "de"] as const) {
        const catalog: Record<string, string> = catalogs[locale];
        expect(catalog[`import.source.${source.id}`], `${locale} name ${source.id}`).toBeTruthy();
        for (let n = 1; n <= source.steps; n += 1) {
          expect(catalog[`import.guide.${source.id}.${String(n)}`], `${locale} ${source.id}.${String(n)}`).toBeTruthy();
        }
      }
    }
  });

  it("picks the detected format", () => {
    expect(formatFor(sourceById("chrome"), { source: "bitwarden", format: "bitwarden", syncedFolder: null })).toBe("bitwarden");
  });

  it("reads a headerless Keeper CSV as keeperCsv", () => {
    expect(formatFor(sourceById("keeper"), { source: "csv", format: "csv", syncedFolder: null })).toBe("keeperCsv");
  });

  it("returns null when nothing was detected", () => {
    expect(formatFor(sourceById("chrome"), none)).toBeNull();
  });

  it("flags a mismatch only for another product", () => {
    const chromium = { source: "chromium", format: "csv", syncedFolder: null } as const;
    expect(isMismatch(sourceById("chrome"), chromium)).toBe(false);
    expect(isMismatch(sourceById("chrome"), { source: "bitwarden", format: "bitwarden", syncedFolder: null })).toBe(true);
    expect(isMismatch(sourceById("otherFile"), { source: "keeper", format: "keeper", syncedFolder: null })).toBe(false);
    expect(isMismatch(sourceById("chrome"), none)).toBe(false);
  });
});
