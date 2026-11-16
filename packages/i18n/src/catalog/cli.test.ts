/// <reference types="node" />
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { locales, type Locale } from "../locales.ts";
import { cli as de } from "./de/cli.ts";
import { cli as en } from "./en/cli.ts";
import { cli as es } from "./es/cli.ts";
import { cli as fr } from "./fr/cli.ts";
import { cli as it_ } from "./it/cli.ts";
import { cli as ja } from "./ja/cli.ts";
import { cli as ko } from "./ko/cli.ts";
import { cli as nl } from "./nl/cli.ts";
import { cli as pl } from "./pl/cli.ts";
import { cli as pt } from "./pt/cli.ts";
import { cli as tr } from "./tr/cli.ts";
import { cli as zh } from "./zh/cli.ts";

const here = dirname(fileURLToPath(import.meta.url));
const json = join(here, "..", "..", "..", "..", "crates", "cli", "i18n");
const placeholders = (s: string): string[] =>
  [...s.matchAll(/\{([^}]*)\}/g)].map((m) => m[1] ?? "").sort();

const catalogs: Record<Locale, Record<string, string>> = {
  en,
  de,
  es,
  fr,
  it: it_,
  pt,
  nl,
  pl,
  tr,
  ja,
  ko,
  zh,
};

describe("CLI catalog", () => {
  it.each(locales)("%s has the English keys, in order, and the same placeholders", (locale) => {
    const catalog = catalogs[locale];
    expect(Object.keys(catalog)).toEqual(Object.keys(en));
    for (const [key, value] of Object.entries(en)) {
      expect(placeholders(catalog[key] ?? ""), key).toEqual(placeholders(value));
    }
  });

  it("uses only plain {name} placeholders (no ICU plural or select)", () => {
    for (const value of Object.values(catalogs).flatMap((catalog) => Object.values(catalog))) {
      for (const name of placeholders(value)) expect(name).toMatch(/^[a-zA-Z]+$/);
    }
  });

  it("addresses the user informally in German", () => {
    for (const value of Object.values(de)) expect(value).not.toMatch(/\b(Sie|Ihr|Ihre|Ihnen)\b/);
  });

  it.each(locales)("the committed %s.json matches the catalog", (locale) => {
    const catalog = catalogs[locale];
    const raw = readFileSync(join(json, `${locale}.json`), "utf8");
    const committed = raw.replaceAll("\r\n", "\n");
    expect(committed).toBe(`${JSON.stringify(catalog, null, 2)}\n`);
  });
});
