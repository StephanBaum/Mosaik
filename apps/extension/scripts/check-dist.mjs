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
// Names from src/background/e2e.ts and e2e-desktop.ts (the fake desktop), and the external listeners.
const FORBIDDEN_TEXT = [
  "mosaicE2E",
  "e2eTabId",
  "FakeDesktop",
  "pauseAfterProbe",
  "onMessageExternal",
  "onConnectExternal",
  // The developer command of the "host missing" screen (src/popup/env.ts): dev and e2e builds only.
  "host:install",
  // `storage.session` holds pending saves with their passwords; content scripts must keep no access to it.
  "setAccessLevel",
];
const PERMISSIONS = ["activeTab", "scripting", "nativeMessaging", "contextMenus", "storage"];
const FORBIDDEN_KEYS = [
  "host_permissions",
  "content_scripts",
  "web_accessible_resources",
  "optional_permissions",
  "key",
  "update_url",
];
const OPTIONAL_HOSTS = ["http://*/*", "https://*/*"];
const TEXT_FILE = /\.(js|mjs|html|css|json)$/;
// The package's own entry points: the capture script is registered at run time (never a
// `content_scripts` entry), so it must be shipped as a file but not be in the manifest.
const REQUIRED_FILES = ["manifest.json", "popup.html", "background.js", "content.js", "capture.js"];
// Page scripts never listen to page messages (docs/EXTENSION_SECURITY.md, "who may talk to whom").
const PAGE_SCRIPTS = ["content.js", "capture.js"];
const MESSAGE_LISTENER = /addEventListener\(\s*["'`]message["'`]|\.onmessage\b|\bonmessage\s*=/;

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
  for (const required of REQUIRED_FILES) {
    if (!existsSync(join(dir, required))) {
      problems.push(`${name}: ${required} is missing`);
    }
  }
  for (const script of PAGE_SCRIPTS) {
    if (existsSync(join(dir, script)) && MESSAGE_LISTENER.test(readFileSync(join(dir, script), "utf8"))) {
      problems.push(`${name}: ${script} listens to page messages`);
    }
  }
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
  if (JSON.stringify([...(manifest.optional_host_permissions ?? [])].sort()) !== JSON.stringify(OPTIONAL_HOSTS)) {
    problems.push(`${name}: optional_host_permissions are ${JSON.stringify(manifest.optional_host_permissions)}`);
  }
  // Chromium: no other extension may connect (Firefox has no such key and allows none by default).
  const external = JSON.stringify(manifest.externally_connectable);
  if (name === "chromium" ? external !== '{"ids":[]}' : external !== undefined) {
    problems.push(`${name}: externally_connectable is ${String(external)}`);
  }
  return problems;
}

const problems = PRODUCTION.flatMap(check);
if (problems.length > 0) {
  process.stderr.write(`check-dist failed:\n${problems.map((p) => `  ${p}`).join("\n")}\n`);
  process.exit(1);
}
process.stdout.write(`check-dist ok: ${PRODUCTION.join(", ")}\n`);
