/** The claims registry names real keys, covers every claim-like web string and blocks production while open. */
import { catalogs } from "@mosaic/i18n";
import { SITE_LOCALES } from "./routes.ts";
import { describe, expect, it } from "vitest";
import { CLAIM_TERMS, CLAIMS, isGated, unverifiedClaims } from "./claims.ts";

describe("claims registry", () => {
  it("names only keys that exist", () => {
    for (const claim of CLAIMS) {
      for (const key of claim.keys) {
        expect(
          Object.keys(catalogs.en).some((k) => k === key || k.startsWith(key)),
          `${claim.id}: ${key}`,
        ).toBe(true);
      }
    }
  });

  it("has unique ids and a check for every gate", () => {
    expect(new Set(CLAIMS.map((c) => c.id)).size).toBe(CLAIMS.length);
    for (const claim of CLAIMS) {
      expect(claim.check.length, claim.id).toBeGreaterThan(20);
    }
  });

  it.each(SITE_LOCALES)("%s: every claim-like web string is gated", (locale) => {
    // Sourced third-party facts keep the comparison's n/v guard; the Mosaic cells are gated explicitly.
    const thirdParty = /^web\.(cmp|compare|vs1password|vsBitwarden|where)\./;
    const loose = Object.entries(catalogs[locale])
      .filter(([key, text]) => key.startsWith("web.") && !thirdParty.test(key) && CLAIM_TERMS.test(text) && !isGated(key))
      .map(([key]) => key);
    expect(loose).toEqual([]);
  });

  it("blocks production while a claim is open", () => {
    expect(unverifiedClaims().map((c) => c.id)).toContain("G1");
  });
});
