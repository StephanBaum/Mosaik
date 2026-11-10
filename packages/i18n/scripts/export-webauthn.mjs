/**
 * Writes the security-key page's files that come from shared packages:
 *  - `crates/server/assets/webauthn/strings.json`: `{ "en": {...}, "de": {...} }`, two-space
 *    indent, trailing newline (the server embeds it);
 *  - the design tokens (light and dark) between the `tokens:start` and `tokens:end` markers of
 *    `crates/server/assets/webauthn/page.css`, so the page styles use only token values.
 * Run with `pnpm --filter @mosaic/i18n export:webauthn` (Node strips the types of the sources).
 */
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { cssVariables } from "../../design-tokens/src/css.ts";
import { webauthn as de } from "../src/catalog/de/webauthn.ts";
import { webauthn as en } from "../src/catalog/en/webauthn.ts";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const out = join(root, "crates", "server", "assets", "webauthn");

writeFileSync(join(out, "strings.json"), `${JSON.stringify({ en, de }, null, 2)}\n`);

const start = "/* tokens:start (written by pnpm --filter @mosaic/i18n export:webauthn) */";
const end = "/* tokens:end */";
const cssPath = join(out, "page.css");
const css = readFileSync(cssPath, "utf8").replaceAll("\r\n", "\n");
const from = css.indexOf(start);
const to = css.indexOf(end);
if (from < 0 || to < from) throw new Error("page.css has no token markers");
writeFileSync(cssPath, `${css.slice(0, from)}${start}\n${cssVariables()}\n${css.slice(to)}`);
