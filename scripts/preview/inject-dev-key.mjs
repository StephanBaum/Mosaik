/**
 * Preview extension (ADR-0153): copies the production Chromium build and adds the dev public key
 * to the copy's manifest, so a locally loaded unpacked extension gets the dev extension ID that a
 * preview native host trusts.
 *
 *   node scripts/preview/inject-dev-key.mjs <dist/chromium> <outdir>
 *
 * The production build itself is never changed (check-dist.mjs forbids `key` there). The
 * script derives the ID from the key the way Chromium does (SHA-256 of the DER key, the first 16
 * bytes, each nibble as a letter a to p) and fails unless it equals DEV_CHROMIUM_EXTENSION_ID in
 * crates/native-messaging/src/ids.rs. It prints the ID.
 */
import { Buffer } from "node:buffer";
import { createHash } from "node:crypto";
import { cpSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const [src, out] = process.argv.slice(2);
if (src === undefined || out === undefined) {
  process.stderr.write("usage: inject-dev-key.mjs <dist/chromium> <outdir>\n");
  process.exit(2);
}
if (!existsSync(join(src, "manifest.json"))) {
  process.stderr.write(`error: ${src}/manifest.json is missing (build the extension first)\n`);
  process.exit(1);
}

const key = readFileSync(join(root, "apps/extension/keys/dev-public-key.b64"), "utf8").trim();
const digest = createHash("sha256").update(Buffer.from(key, "base64")).digest();
let id = "";
for (const byte of digest.subarray(0, 16)) {
  id += String.fromCharCode(97 + (byte >> 4)) + String.fromCharCode(97 + (byte & 15));
}
const rust = readFileSync(join(root, "crates/native-messaging/src/ids.rs"), "utf8");
const expected = /DEV_CHROMIUM_EXTENSION_ID: &str = "([a-p]{32})"/.exec(rust)?.[1];
if (expected !== id) {
  process.stderr.write(`error: the dev key gives ${id}, the native host expects ${String(expected)}\n`);
  process.exit(1);
}

mkdirSync(out, { recursive: true });
cpSync(src, out, { recursive: true });
const path = join(out, "manifest.json");
const manifest = JSON.parse(readFileSync(path, "utf8"));
if ("key" in manifest) {
  process.stderr.write("error: the production manifest already has a key\n");
  process.exit(1);
}
manifest.key = key;
writeFileSync(path, `${JSON.stringify(manifest, null, 2)}\n`);
process.stdout.write(`dev extension ID: ${id}\n`);
