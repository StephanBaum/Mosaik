#!/usr/bin/env node
/**
 * Fails when a shipped or user-facing text holds a claim Mosaic never makes (M9 T9, OD-M9-18).
 *
 *   node scripts/release/wording-check.mjs
 *
 * The banned terms are in scripts/release/banned-claims.txt. Scanned: README.md, SECURITY.md,
 * the release, security and challenge documents, the UI string catalogs, the CLI messages, the
 * handoff page, tauri.conf.json and the extension manifest sources. Not scanned:
 * docs/DECISIONS.md and docs/THREAT_MODEL.md (they quote the terms to forbid them) and
 * marketing/ (OD-M9-18). Missing files are skipped. No dependency beyond Node.
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..", "..");

/** Files (relative to the root) and directories (a trailing slash, scanned recursively). */
const TARGETS = [
  "README.md",
  "SECURITY.md",
  "docs/RELEASE.md",
  "docs/VERIFY.md",
  "docs/REPRODUCIBLE_BUILDS.md",
  "docs/AUDIT_PREP.md",
  "docs/BUG_BOUNTY.md",
  "docs/CHALLENGE.md",
  "docs/HALL_OF_FAME.md",
  "docs/DEPENDENCIES.md",
  "packages/i18n/src/catalog/",
  "crates/cli/i18n/",
  "crates/server/assets/webauthn/",
  "apps/desktop/src-tauri/tauri.conf.json",
  "apps/extension/scripts/manifest.ts",
  "apps/extension/package.json",
];
const TEXT = /\.(md|json|ts|tsx|js|mjs|html|css|toml|txt)$/;
// Tests quote the banned terms to check that the texts avoid them; they ship nothing.
const TEST_FILE = /\.(test|spec)\.[a-z]+$/;

/** @param {string} s */
const normalize = (s) => s.toLowerCase().replace(/[-\s]+/g, " ");

/** @returns {{ term: string; pattern: RegExp }[]} */
function loadTerms() {
  const lines = readFileSync(join(here, "banned-claims.txt"), "utf8").split(/\r?\n/);
  return lines
    .map((l) => l.trim())
    .filter((l) => l !== "" && !l.startsWith("#"))
    .map((term) => {
      const escaped = normalize(term).replace(/[.*+?^${}()|[\]\\%]/g, "\\$&");
      // Word boundaries on letters and digits only, so "unaudited" and "pre-audit" do not match "audited".
      return { term, pattern: new RegExp(`(?<![a-z0-9])${escaped}(?![a-z0-9])`, "g") };
    });
}

/** @param {string} path @param {string[]} out */
function collect(path, out) {
  if (!existsSync(path)) return;
  const info = statSync(path);
  if (info.isDirectory()) {
    for (const name of readdirSync(path).sort()) {
      if (name === "node_modules") continue;
      collect(join(path, name), out);
    }
  } else if (TEXT.test(path) && !TEST_FILE.test(path)) {
    out.push(path);
  }
}

const terms = loadTerms();
if (terms.length === 0) {
  process.stderr.write("wording-check: banned-claims.txt lists no terms\n");
  process.exit(2);
}

/** @type {string[]} */
const files = [];
for (const t of TARGETS) collect(join(root, t), files);

let findings = 0;
for (const file of files) {
  const lines = readFileSync(file, "utf8").split(/\r?\n/);
  lines.forEach((line, i) => {
    // Normalize per line; a term split across a line break is not matched (none of the terms is long).
    const text = normalize(line);
    for (const { term, pattern } of terms) {
      pattern.lastIndex = 0;
      if (pattern.test(text)) {
        findings += 1;
        process.stderr.write(`${relative(root, file)}:${i + 1}: banned claim "${term}"\n`);
      }
    }
  });
}

if (findings > 0) {
  process.stderr.write(`wording-check: ${findings} banned claim(s) in ${files.length} files\n`);
  process.exit(1);
}
process.stdout.write(`wording-check: ${files.length} files scanned, ${terms.length} terms, no banned claims\n`);
