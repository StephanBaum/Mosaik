/**
 * Fails when a production package (dist/chromium, dist/firefox) holds test code or a manifest
 * that grants more than docs/EXTENSION_SECURITY.md allows. Run after `pnpm build`.
 */
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "dist");
const PRODUCTION = ["chromium", "firefox"];
const FORBIDDEN_TEXT = ["mosaicE2E", "e2eTabId", "onMessageExternal", "onConnectExternal"];
const PERMISSIONS = ["activeTab", "scripting", "nativeMessaging", "contextMenus", "storage"];
const FORBIDDEN_KEYS = ["host_permissions", "web_accessible_resources", "key", "update_url"];
const TEXT_FILE = /\.(js|mjs|html|css|json)$/;

/** @param {string} dir @returns {string[]} */
function walk(dir) {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) =>
    entry.isDirectory() ? walk(join(dir, entry.name)) : [join(dir, entry.name)],
  );
}

/** @param {string} name @returns {string[]} */
function check(name) {
  const dir = join(root, name);
  if (!existsSync(join(dir, "manifest.json"))) {
    return [`${name}: dist/${name}/manifest.json is missing (run pnpm build first)`];
  }
  const problems = [];
  for (const file of walk(dir).filter((f) => TEXT_FILE.test(f))) {
    const text = readFileSync(file, "utf8");
    for (const word of FORBIDDEN_TEXT) {
      if (text.includes(word)) {
        problems.push(`${name}: ${file.slice(dir.length + 1)} contains "${word}"`);
      }
    }
  }
  const manifest = JSON.parse(readFileSync(join(dir, "manifest.json"), "utf8"));
  for (const key of FORBIDDEN_KEYS) {
    if (key in manifest) {
      problems.push(`${name}: manifest has "${key}"`);
    }
  }
  if (JSON.stringify([...(manifest.permissions ?? [])].sort()) !== JSON.stringify([...PERMISSIONS].sort())) {
    problems.push(`${name}: permissions are ${JSON.stringify(manifest.permissions)}`);
  }
  return problems;
}

const problems = PRODUCTION.flatMap(check);
if (problems.length > 0) {
  process.stderr.write(`check-dist failed:\n${problems.map((p) => `  ${p}`).join("\n")}\n`);
  process.exit(1);
}
process.stdout.write(`check-dist ok: ${PRODUCTION.join(", ")}\n`);
