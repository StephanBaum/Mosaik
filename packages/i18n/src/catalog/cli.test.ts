/// <reference types="node" />
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { cli as de } from "./de/cli.ts";
import { cli as en } from "./en/cli.ts";

const here = dirname(fileURLToPath(import.meta.url));
const json = join(here, "..", "..", "..", "..", "crates", "cli", "i18n");
const placeholders = (s: string): string[] =>
  [...s.matchAll(/\{([^}]*)\}/g)].map((m) => m[1] ?? "").sort();

describe("CLI catalog", () => {
  it("has the same keys and placeholders in EN and DE", () => {
    expect(Object.keys(de)).toEqual(Object.keys(en));
    for (const [key, value] of Object.entries(en)) {
      const german = (de as Record<string, string>)[key] ?? "";
      expect(placeholders(german), key).toEqual(placeholders(value));
    }
  });

  it("uses only plain {name} placeholders (no ICU plural or select)", () => {
    for (const value of [...Object.values(en), ...Object.values(de)]) {
      for (const name of placeholders(value)) expect(name).toMatch(/^[a-zA-Z]+$/);
    }
  });

  it("addresses the user informally in German", () => {
    for (const value of Object.values(de)) expect(value).not.toMatch(/\b(Sie|Ihr|Ihre|Ihnen)\b/);
  });

  it.each([
    ["en", en],
    ["de", de],
  ] as const)("the committed %s.json matches the catalog", (locale, catalog) => {
    const committed = readFileSync(join(json, `${locale}.json`), "utf8");
    expect(committed).toBe(`${JSON.stringify(catalog, null, 2)}\n`);
  });
});
