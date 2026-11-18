import { catalogs } from "@mosaic/i18n";
import { SITE_LOCALES } from "../src/pages/routes.ts";
import { describe, expect, it } from "vitest";
import { subsetDmSans, webCharset } from "./fonts.ts";

describe("DM Sans subset", () => {
  it("covers every character the site prints", () => {
    const set = new Set(webCharset());
    for (const c of "äöüÄÖÜß„“”–…€→") {
      expect(set.has(c), c).toBe(true);
    }
    for (const locale of SITE_LOCALES) {
      for (const [key, value] of Object.entries(catalogs[locale])) {
        if (key.startsWith("web.")) {
          for (const c of value.replace(/\{[a-zA-Z]+\}/g, "")) {
            expect(set.has(c), `${locale} ${key}: ${c}`).toBe(true);
          }
        }
      }
    }
  });

  it("subsets DM Sans under the preload budget", async () => {
    const buf = await subsetDmSans(webCharset());
    expect(buf.length).toBeLessThanOrEqual(40_000);
    expect(buf.subarray(0, 4).toString("latin1")).toBe("wOF2");
  }, 30_000);
});
