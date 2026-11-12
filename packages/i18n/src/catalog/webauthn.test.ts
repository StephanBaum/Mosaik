import { describe, expect, it } from "vitest";
import { webauthn as de } from "./de/webauthn.ts";
import { webauthn as en } from "./en/webauthn.ts";

const placeholders = (s: string): string[] =>
  [...s.matchAll(/\{([^}]*)\}/g)].map((m) => m[1] ?? "").sort();

describe("security-key page catalog", () => {
  it("has the same keys and placeholders in EN and DE", () => {
    expect(Object.keys(de)).toEqual(Object.keys(en));
    for (const [key, value] of Object.entries(en)) {
      const german = (de as Record<string, string>)[key] ?? "";
      expect(placeholders(german), key).toEqual(placeholders(value));
    }
  });

  it("addresses the user informally in German", () => {
    for (const value of Object.values(de)) expect(value).not.toMatch(/\b(Sie|Ihr|Ihre|Ihnen)\b/);
  });

  it("makes no claim of audits, bank-grade security or being unhackable", () => {
    for (const value of [...Object.values(en), ...Object.values(de)]) {
      expect(value).not.toMatch(/audit|bank-grade|bankgrade|unhackable|unhackbar/i);
    }
  });

  it("holds no markup (the page writes every string as text)", () => {
    for (const value of [...Object.values(en), ...Object.values(de)]) {
      expect(value).not.toMatch(/[<>]/);
    }
  });
});
