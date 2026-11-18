/**
 * Writes the CLI catalogs to `crates/cli/i18n/<locale>.json` (ADR-0075, ADR-0168): flat key to
 * string, in catalog order, two-space indent, trailing newline. The Rust binary embeds them.
 * Run with `pnpm --filter @mosaic/i18n export:cli` (Node strips the types of the catalogs).
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { locales } from "../src/locales.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..", "..");
const out = join(root, "crates", "cli", "i18n");
mkdirSync(out, { recursive: true });
for (const locale of locales) {
  const { cli } = await import(pathToFileURL(join(here, "..", "src", "catalog", locale, "cli.ts")).href);
  writeFileSync(join(out, `${locale}.json`), `${JSON.stringify(cli, null, 2)}\n`);
}
