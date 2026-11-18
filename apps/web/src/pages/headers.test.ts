import { describe, expect, it } from "vitest";
import { ROUTES } from "./routes.ts";
import { CSP, headersFile, speculationRules } from "./security.ts";

describe("headers", () => {
  it("gives every page path exactly one Cache-Control and none to /*", () => {
    const blocks = headersFile().split(/\n(?=\/)/);
    expect(blocks.find((b) => b.startsWith("/*\n"))).not.toMatch(/Cache-Control/);
    for (const r of ROUTES) {
      for (const p of [r.en, r.de]) {
        const b = blocks.find((x) => x.startsWith(`${p}\n`)) ?? "";
        expect(b.match(/Cache-Control/g)).toHaveLength(1);
        expect(b).toContain('Speculation-Rules: "/speculation-rules.json"');
      }
    }
    expect(headersFile()).toContain("/assets/*\n  Cache-Control: public, max-age=31536000, immutable");
    expect(headersFile()).toContain("/img/*\n  Cache-Control: public, max-age=2592000");
    expect(headersFile()).toContain("/speculation-rules.json\n  Content-Type: application/speculationrules+json");
  });

  it("enforces Trusted Types and never allows inline code", () => {
    expect(CSP).toContain("require-trusted-types-for 'script'; trusted-types 'none'");
    expect(CSP).not.toMatch(/unsafe-|sha256-|nonce-/);
  });

  it("prefetches only, moderately", () => {
    const rules = JSON.parse(speculationRules()) as Record<string, unknown>;
    expect(Object.keys(rules)).toEqual(["prefetch"]);
    expect(JSON.stringify(rules)).toContain('"eagerness":"moderate"');
  });
});
