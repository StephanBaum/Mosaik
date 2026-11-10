/**
 * Writes the CLI catalogs to `crates/cli/i18n/{en,de}.json` (ADR-0075): flat key to string,
 * in catalog order, two-space indent, trailing newline. The Rust binary embeds them.
 * Run with `pnpm --filter @mosaic/i18n export:cli` (Node strips the types of the catalogs).
 */
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { cli as de } from "../src/catalog/de/cli.ts";
import { cli as en } from "../src/catalog/en/cli.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const out = join(root, "crates", "cli", "i18n");
mkdirSync(out, { recursive: true });
for (const [locale, catalog] of [
  ["en", en],
  ["de", de],
]) {
  writeFileSync(join(out, `${locale}.json`), `${JSON.stringify(catalog, null, 2)}\n`);
}
