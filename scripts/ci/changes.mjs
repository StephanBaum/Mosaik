#!/usr/bin/env node
/**
 * Change detection for CI (docs/CI.md, ADR-0169): which jobs a pull request needs.
 *
 *   node scripts/ci/changes.mjs --base <sha> --head <sha> [--github-output <file>]
 *   node scripts/ci/changes.mjs --files <list, one path per line> [--github-output <file>]
 *   node scripts/ci/changes.mjs --full <reason> [--github-output <file>]
 *
 * With --base and --head it diffs the merge base of the two against the head. It prints the plan as
 * JSON and, with --github-output, writes it as step outputs. Every error exits non-zero, so the
 * required jobs fail (fail closed). A file outside the area map, or a run-everything file, makes the
 * plan "full". No dependency beyond Node and git.
 */
import { execFileSync } from "node:child_process";
import { appendFileSync, existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, posix, resolve as resolvePath } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

export const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

export const OSES = ["ubuntu-latest", "windows-latest", "macos-latest"];
const LINUX_WINDOWS = ["ubuntu-latest", "windows-latest"];

/** Coverage jobs (brief §8 and the crates held to the same bar), as in ci.yml before ADR-0169. */
export const COVERAGE = [
  { crate: "mosaic-core" },
  { crate: "mosaic-totp" },
  { crate: "mosaic-generator" },
  { crate: "mosaic-vault" },
  { crate: "mosaic-insights" },
  { crate: "mosaic-import" },
  { crate: "mosaic-ssh-agent" },
  { crate: "mosaic-qr" },
  { crate: "mosaic-origin" },
  { crate: "mosaic-native-messaging" },
  { crate: "mosaic-passkey" },
  // The folder, engine, HTTP and chaos tests need the test utilities, and the hosted engine paths are
  // exercised by the server crate's integration tests; only crates/sync files count.
  {
    crate: "mosaic-sync",
    features:
      "-p mosaic-server --features mosaic-sync/http,mosaic-sync/test-util --ignore-filename-regex '(crates/server/|/testutil/)'",
    alsoWhen: ["mosaic-server"],
  },
  { crate: "mosaic-server" },
  { crate: "mosaic-mobile" },
];

/** Fuzz smoke directories, as in ci.yml before ADR-0169 (crates/agent/fuzz is not in CI). */
export const FUZZ_DIRS = [
  "crates/core/fuzz",
  "crates/vault/fuzz",
  "crates/totp/fuzz",
  "crates/qr/fuzz",
  "crates/generator/fuzz",
  "crates/insights/fuzz",
  "crates/import/fuzz",
  "crates/ssh-agent/fuzz",
  "crates/native-messaging/fuzz",
  "crates/origin/fuzz",
  "crates/passkey/fuzz",
  "crates/sync/fuzz",
  "crates/server/fuzz",
  "crates/cli/fuzz",
  "crates/mobile/fuzz",
  "apps/desktop/src-tauri/fuzz",
];

/** Toolchains, lockfiles, workspace manifests and shared configs: a change runs everything. */
const FULL_FILES = new Set([
  "Cargo.toml",
  "Cargo.lock",
  "rust-toolchain.toml",
  "deny.toml",
  "clippy.toml",
  "rustfmt.toml",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
  ".npmrc",
  "tsconfig.json",
  "tsconfig.base.json",
  "eslint.config.js",
  "vitest.config.ts",
  ".gitattributes",
]);
const FULL_PREFIXES = [".github/", "scripts/ci/", ".cargo/"];
/** Files no CI job reads. */
const NO_EFFECT_FILES = new Set([".gitignore", ".editorconfig", ".prettierignore"]);
const NO_EFFECT_PREFIXES = [".vscode/"];
/** A `*.md` file outside these is documentation. */
const CODE_DIRS = new Set(["crates", "apps", "packages", "vendor", "scripts", "tools", "deploy", ".github"]);
/** These crates count as "all Rust" (everything depends on them). */
const ALL_RUST_CRATES = new Set(["crates/core", "crates/vault"]);
/** Areas with macOS-only code (keychain, Touch ID, native-messaging install paths, CLI sync paths). */
const MAC_PREFIXES = ["crates/platform/", "crates/native-messaging/", "crates/cli/src/", "apps/desktop/src-tauri/"];
const DESKTOP = "mosaic-desktop";
const RELEASE_EXCLUDED = ["mosaic-mobile", "mosaic-mobile-ffi"];
const CRATE_NAME = /^[a-z0-9][a-z0-9_-]*$/;

/**
 * The Rust workspace as a dependency graph, read from the manifests as text.
 * @typedef {{ name: string; dir: string; normal: Set<string>; dev: Set<string> }} Crate
 * @typedef {{ dir: string; deps: Set<string> }} FuzzCrate
 * @typedef {{ crates: Map<string, Crate>; byDir: Map<string, Crate>; fuzz: Map<string, FuzzCrate> }} Workspace
 */

/**
 * The dependency section kind of a manifest table header, or null for any other table.
 * @param {string} section
 * @returns {"normal" | "dev" | null}
 */
function depKind(section) {
  const s = section.replace(/^target\.(?:'[^']*'|"[^"]*"|[^.]+)\./, "");
  if (/^dev-dependencies(?:\.|$)/.test(s)) return "dev";
  if (/^(?:build-)?dependencies(?:\.|$)/.test(s)) return "normal";
  return null;
}

/**
 * Package name and path dependencies of one manifest. An internal crate without `path` throws, so a
 * dependency the parser cannot see never goes unnoticed.
 * @param {string} text
 * @param {string} file for error messages
 * @returns {{ name: string | null; deps: { kind: "normal" | "dev"; path: string }[] }}
 */
export function parseManifest(text, file) {
  let section = "";
  /** @type {string | null} */
  let name = null;
  /** @type {{ kind: "normal" | "dev"; path: string }[]} */
  const deps = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.replace(/\s#.*$/, "").replace(/^#.*$/, "").trim();
    if (line === "") continue;
    const header = /^\[\[?\s*([^\]]+?)\s*\]\]?$/.exec(line);
    if (header) {
      section = header[1];
      continue;
    }
    if (section === "package") {
      const m = /^name\s*=\s*"([^"]+)"/.exec(line);
      if (m) name = m[1];
      continue;
    }
    const kind = depKind(section);
    if (!kind) continue;
    const path = /\bpath\s*=\s*"([^"]+)"/.exec(line);
    if (path) {
      deps.push({ kind, path: path[1] });
      continue;
    }
    const key = /^([A-Za-z0-9_-]+)\s*[.=]/.exec(line);
    const tableDep = /(?:^|\.)(?:build-|dev-)?dependencies\.([A-Za-z0-9_-]+)$/.exec(section);
    if ((key && !tableDep && key[1].startsWith("mosaic-")) || (tableDep?.[1].startsWith("mosaic-") && key?.[1] === "workspace")) {
      throw new Error(`${file}: internal dependency without a path in [${section}]: ${line}`);
    }
  }
  return { name, deps };
}

/**
 * @param {string} root
 * @returns {Workspace}
 */
export function loadWorkspace(root = ROOT) {
  const rootManifest = readFileSync(join(root, "Cargo.toml"), "utf8");
  const members = /^members\s*=\s*\[([^\]]*)\]/m.exec(rootManifest);
  const memberList = members ? [...members[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]) : [];
  if (memberList.join(",") !== "crates/*,apps/desktop/src-tauri") {
    throw new Error(`workspace members changed (${memberList.join(", ")}); update scripts/ci/changes.mjs`);
  }
  const dirs = readdirSync(join(root, "crates"))
    .filter((d) => existsSync(join(root, "crates", d, "Cargo.toml")))
    .map((d) => `crates/${d}`);
  dirs.push("apps/desktop/src-tauri");

  /** @type {Map<string, { name: string; deps: { kind: "normal" | "dev"; path: string }[] }>} */
  const parsed = new Map();
  for (const dir of dirs) {
    const file = `${dir}/Cargo.toml`;
    const m = parseManifest(readFileSync(join(root, file), "utf8"), file);
    if (!m.name || !CRATE_NAME.test(m.name)) throw new Error(`${file}: no valid package name`);
    parsed.set(dir, { name: m.name, deps: m.deps });
  }
  /** @type {Map<string, Crate>} */
  const byDir = new Map();
  /** @type {Map<string, Crate>} */
  const crates = new Map();
  for (const [dir, m] of parsed) {
    const c = { name: m.name, dir, normal: new Set(), dev: new Set() };
    byDir.set(dir, c);
    crates.set(m.name, c);
  }
  /** @param {string} from @param {string} rel @param {string} file */
  const resolve = (from, rel, file) => {
    const to = posix.normalize(posix.join(from, rel));
    const target = byDir.get(to);
    if (!target) throw new Error(`${file}: path dependency ${rel} is not a workspace crate`);
    return target.name;
  };
  for (const [dir, m] of parsed) {
    const c = /** @type {Crate} */ (byDir.get(dir));
    for (const d of m.deps) {
      const target = resolve(dir, d.path, `${dir}/Cargo.toml`);
      if (target !== c.name) c[d.kind].add(target);
    }
  }
  /** @type {Map<string, FuzzCrate>} */
  const fuzz = new Map();
  for (const dir of dirs) {
    const fdir = `${dir}/fuzz`;
    const file = `${fdir}/Cargo.toml`;
    if (!existsSync(join(root, file))) continue;
    const m = parseManifest(readFileSync(join(root, file), "utf8"), file);
    // A fuzz crate always exercises its parent, also through modules included by #[path].
    const deps = new Set([/** @type {Crate} */ (byDir.get(dir)).name]);
    for (const d of m.deps) deps.add(resolve(fdir, d.path, file));
    fuzz.set(fdir, { dir: fdir, deps });
  }
  return { crates, byDir, fuzz };
}

/**
 * What one changed file affects.
 * @typedef {{
 *   crates: Set<string>; tests: Set<string>; fuzz: Set<string>; allRust: boolean; rustAny: boolean;
 *   mac: boolean; ts: boolean; tsUbuntu: boolean; desktopFrontend: boolean; extension: boolean;
 *   web: boolean; serverImage: boolean; supplyChain: boolean; release: boolean; ios: boolean; docs: boolean
 * }} Effects
 * @returns {Effects}
 */
function emptyEffects() {
  return {
    crates: new Set(), // source changes: every crate that depends on them is affected
    tests: new Set(), // test inputs only (a test reads the file): just that crate's tests
    fuzz: new Set(),
    allRust: false,
    rustAny: false,
    mac: false,
    ts: false,
    tsUbuntu: false,
    desktopFrontend: false,
    extension: false,
    web: false,
    serverImage: false,
    supplyChain: false,
    release: false,
    ios: false,
    docs: false,
  };
}

/**
 * Areas of one file, or null when the file runs everything (a run-everything file, or outside the map).
 * @param {string} file repository-relative, forward slashes
 * @param {Workspace} ws
 * @returns {Effects | null}
 */
export function classify(file, ws) {
  if (FULL_FILES.has(file) || FULL_PREFIXES.some((p) => file.startsWith(p))) return null;
  const e = emptyEffects();
  const seg = file.split("/");
  const base = seg[seg.length - 1];
  // Add-ons that never make a file "known" by themselves.
  if (base === "Cargo.toml" || base === "Cargo.lock") e.supplyChain = e.release = true;
  if (base === "package.json") e.release = true;
  // ESLint lints every script in the repository but the mobile apps and the design references.
  if (/\.(?:[cm]?[jt]s|tsx|jsx)$/.test(file) && !file.startsWith("apps/mobile/") && !file.startsWith("docs/design/")) {
    e.ts = true;
  }

  if (NO_EFFECT_FILES.has(file) || NO_EFFECT_PREFIXES.some((p) => file.startsWith(p))) return e;
  if (file.startsWith("docs/") || file.startsWith("marketing/")) {
    e.docs = true;
    return e;
  }
  if (!CODE_DIRS.has(seg[0]) && (file.endsWith(".md") || (seg.length === 1 && base.startsWith("LICENSE")))) {
    e.docs = true;
    return e;
  }
  if (file.startsWith("apps/web/")) {
    e.ts = e.web = true;
    return e;
  }
  if (file.startsWith("apps/extension/")) {
    e.ts = e.extension = true;
    // crates/native-messaging derives the dev extension ID from the committed key in a test.
    if (file.startsWith("apps/extension/keys/")) e.tests.add("mosaic-native-messaging");
    return e;
  }
  if (file.startsWith("apps/desktop/src-tauri/")) {
    e.rustAny = true;
    if (file.startsWith("apps/desktop/src-tauri/fuzz/")) {
      e.fuzz.add("apps/desktop/src-tauri/fuzz");
      return e;
    }
    e.crates.add(DESKTOP);
    e.mac = true;
    // Tests of other crates read these files.
    if (file.startsWith("apps/desktop/src-tauri/wix/")) e.tests.add("mosaic-native-messaging");
    if (/^apps\/desktop\/src-tauri\/tauri(?:\.[a-z]+)?\.conf\.json$/.test(file)) {
      e.tests.add("mosaic-platform");
      e.desktopFrontend = true; // vite.config.ts reads tauri.conf.json
    }
    return e;
  }
  if (file.startsWith("apps/desktop/")) {
    e.ts = e.desktopFrontend = true;
    // The Rust contract test compares the generated fixtures with this file.
    if (file === "apps/desktop/src/ipc/contract.fixtures.ts") e.tests.add(DESKTOP);
    // The extension popup imports the desktop icon paths.
    if (file.startsWith("apps/desktop/src/components/icons/")) e.extension = true;
    return e;
  }
  if (file.startsWith("apps/mobile/")) {
    e.tsUbuntu = true; // the mobile export drift check of the TypeScript job (one OS is enough)
    if (file.startsWith("apps/mobile/ios/")) e.ios = true;
    return e;
  }
  if (file.startsWith("packages/")) {
    e.ts = e.desktopFrontend = e.extension = e.web = true;
    if (file.startsWith("packages/i18n/") || file.startsWith("packages/design-tokens/")) e.ios = true;
    // Rust tests that compare their texts with the catalogs.
    if (/^packages\/i18n\/src\/catalog\/[^/]+\/insights\.ts$/.test(file)) e.tests.add("mosaic-insights");
    if (/^packages\/i18n\/src\/catalog\/[^/]+\/(?:sync|sharing)\.ts$/.test(file)) e.tests.add(DESKTOP);
    return e;
  }
  if (file.startsWith("crates/")) {
    const dir = seg.slice(0, 2).join("/");
    const crate = ws.byDir.get(dir);
    if (!crate || seg.length < 3) return null; // a new or unknown crate: run everything
    e.rustAny = true;
    if (seg[2] === "fuzz") {
      if (ws.fuzz.has(`${dir}/fuzz`)) e.fuzz.add(`${dir}/fuzz`);
      return e;
    }
    e.crates.add(crate.name);
    if (ALL_RUST_CRATES.has(dir)) e.allRust = true;
    if (MAC_PREFIXES.some((p) => file.startsWith(p))) e.mac = true;
    if (dir === "crates/mobile" || dir === "crates/mobile-ffi") e.ios = true;
    // The extension's tests and e2e pages read these.
    if (
      file.startsWith("crates/server/assets/") ||
      file.startsWith("crates/passkey/fixtures/") ||
      file.startsWith("crates/native-messaging/fixtures/")
    ) {
      e.ts = e.extension = true;
    }
    return e;
  }
  if (file.startsWith("vendor/") || file.startsWith("tools/")) {
    e.allRust = e.rustAny = true;
    return e;
  }
  if (file.startsWith("deploy/") || file === ".dockerignore") {
    e.serverImage = true;
    return e;
  }
  if (file.startsWith("scripts/release/")) {
    e.release = true;
    return e;
  }
  if (file.startsWith("scripts/mobile/")) {
    if (!base.includes("android")) e.ios = true;
    return e;
  }
  if (file.startsWith("scripts/preview/")) return e; // preview.yml only
  return null;
}

/**
 * Crates whose normal (build) dependency closure contains one of `seeds`, the seeds included.
 * @param {Workspace} ws
 * @param {Iterable<string>} seeds
 */
function reverseClosure(ws, seeds) {
  /** @type {Map<string, string[]>} */
  const users = new Map();
  for (const c of ws.crates.values()) {
    for (const d of c.normal) users.set(d, [...(users.get(d) ?? []), c.name]);
  }
  const out = new Set(seeds);
  const queue = [...out];
  while (queue.length > 0) {
    const next = /** @type {string} */ (queue.pop());
    for (const u of users.get(next) ?? []) {
      if (!out.has(u)) {
        out.add(u);
        queue.push(u);
      }
    }
  }
  return out;
}

/**
 * @typedef {{
 *   ok: "true"; full: "true" | "false"; reason: string; summary: string;
 *   rust_os: string; rust_args: string; rust_release_args: string; cli_os: string; desktop_rust_os: string;
 *   ts_os: string; desktop_frontend: string; extension: string; web: string; server_image: string;
 *   supply_chain: string; release: string; ios: string; coverage: string; fuzz: string
 * }} Plan
 */

/**
 * @param {Workspace} ws
 * @param {string} reason
 * @returns {Plan}
 */
export function fullPlan(ws, reason) {
  const all = OSES.join(" ");
  return {
    ok: "true",
    full: "true",
    reason,
    summary: `full run (${reason})`,
    rust_os: all,
    rust_args: `--workspace --exclude ${DESKTOP}`,
    rust_release_args: `--workspace --exclude ${DESKTOP} ${RELEASE_EXCLUDED.map((c) => `--exclude ${c}`).join(" ")}`,
    cli_os: all,
    desktop_rust_os: all,
    ts_os: all,
    desktop_frontend: "true",
    extension: "true",
    web: "true",
    server_image: "true",
    supply_chain: "true",
    release: "true",
    ios: "true",
    coverage: JSON.stringify(COVERAGE.map((c) => ({ crate: c.crate, features: c.features ?? "" }))),
    fuzz: JSON.stringify(FUZZ_DIRS),
  };
}

/**
 * The plan for a list of changed files.
 * @param {string[]} files
 * @param {Workspace} ws
 * @returns {Plan}
 */
export function plan(files, ws) {
  const acc = emptyEffects();
  for (const file of files) {
    const e = classify(file, ws);
    if (e === null) return fullPlan(ws, `run-everything or unmapped file: ${file}`);
    for (const key of /** @type {(keyof Effects)[]} */ (Object.keys(acc))) {
      const v = e[key];
      if (v instanceof Set) for (const x of v) /** @type {Set<string>} */ (acc[key]).add(x);
      else if (v) /** @type {Record<string, unknown>} */ (acc)[key] = true;
    }
  }

  const allNames = [...ws.crates.keys()];
  const changed = reverseClosure(ws, acc.allRust ? allNames : acc.crates);
  // Tests of a crate also run when a dev-dependency (or anything it builds on) changed.
  const affected = new Set([...changed, ...acc.tests]);
  for (const c of ws.crates.values()) {
    if ([...c.dev].some((d) => changed.has(d))) affected.add(c.name);
  }
  const scope = [...affected].filter((c) => c !== DESKTOP).sort();
  const everyCrate = allNames.filter((c) => c !== DESKTOP).every((c) => affected.has(c));
  const releaseScope = scope.filter((c) => !RELEASE_EXCLUDED.includes(c));
  const pkgs = (/** @type {string[]} */ list) => list.map((c) => `-p ${c}`).join(" ");

  /** @param {boolean} run */
  const rustOses = (run) => (run ? [...LINUX_WINDOWS, ...(acc.mac ? ["macos-latest"] : [])] : []);
  const rustOs = scope.length > 0 ? rustOses(true) : acc.rustAny ? ["ubuntu-latest"] : [];
  const tsOs = acc.ts ? OSES : acc.tsUbuntu ? ["ubuntu-latest"] : [];
  const coverage = COVERAGE.filter((c) => affected.has(c.crate) || (c.alsoWhen ?? []).some((a) => affected.has(a)));
  const fuzz = FUZZ_DIRS.filter((d) => {
    const f = ws.fuzz.get(d);
    return acc.fuzz.has(d) || (f !== undefined && [...f.deps].some((x) => changed.has(x)));
  });

  /** @type {Plan} */
  const p = {
    ok: "true",
    full: "false",
    reason: files.length === 0 ? "no changed files" : `${files.length} changed file(s)`,
    summary: "",
    rust_os: rustOs.join(" "),
    rust_args: scope.length === 0 ? "" : everyCrate ? `--workspace --exclude ${DESKTOP}` : pkgs(scope),
    rust_release_args:
      releaseScope.length === 0
        ? ""
        : everyCrate
          ? fullPlan(ws, "").rust_release_args
          : pkgs(releaseScope),
    cli_os: rustOses(affected.has("mosaic-cli")).join(" "),
    desktop_rust_os: rustOses(affected.has(DESKTOP)).join(" "),
    ts_os: tsOs.join(" "),
    desktop_frontend: String(acc.desktopFrontend),
    extension: String(acc.extension),
    web: String(acc.web),
    server_image: String(acc.serverImage || changed.has("mosaic-server")),
    supply_chain: String(acc.supplyChain),
    release: String(acc.release),
    ios: String(acc.ios),
    coverage: JSON.stringify(coverage.map((c) => ({ crate: c.crate, features: c.features ?? "" }))),
    fuzz: JSON.stringify(fuzz),
  };
  p.summary = summarize(p);
  return p;
}

/** @param {Plan} p */
export function summarize(p) {
  const parts = [];
  if (p.rust_os) parts.push(`Rust on ${p.rust_os} (${p.rust_args || "fmt and unsafe check only"})`);
  if (p.cli_os) parts.push(`CLI on ${p.cli_os}`);
  if (p.desktop_rust_os) parts.push(`desktop Rust on ${p.desktop_rust_os}`);
  if (p.ts_os) parts.push(`TypeScript on ${p.ts_os}`);
  for (const k of /** @type {const} */ (["desktop_frontend", "extension", "web", "server_image", "supply_chain", "release", "ios"])) {
    if (p[k] === "true") parts.push(k.replace("_", " "));
  }
  const cov = /** @type {unknown[]} */ (JSON.parse(p.coverage)).length;
  const fz = /** @type {unknown[]} */ (JSON.parse(p.fuzz)).length;
  if (cov > 0) parts.push(`${cov} coverage job(s)`);
  if (fz > 0) parts.push(`${fz} fuzz job(s)`);
  return `${p.reason}: ${parts.length === 0 ? "nothing to run (documentation or no-effect files only)" : parts.join("; ")}`;
}

/** @param {string[]} args */
function git(args) {
  return execFileSync("git", args, { cwd: ROOT, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], maxBuffer: 64 << 20 });
}

/**
 * Files changed on the head side since the merge base (renames as delete plus add).
 * @param {string} base
 * @param {string} head
 */
export function changedFiles(base, head) {
  for (const sha of [base, head]) if (!/^[0-9a-f]{40}$/.test(sha)) throw new Error(`not a commit SHA: "${sha}"`);
  const mergeBase = git(["merge-base", base, head]).trim();
  if (!/^[0-9a-f]{40}$/.test(mergeBase)) throw new Error("no merge base");
  return git(["diff", "--name-only", "--no-renames", "-z", mergeBase, head]).split("\0").filter((f) => f !== "");
}

/** @param {string[]} argv */
function main(argv) {
  /** @type {Record<string, string>} */
  const opt = {};
  for (let i = 0; i < argv.length; i += 2) {
    const k = argv[i];
    const v = argv[i + 1];
    if (!["--base", "--head", "--files", "--full", "--github-output"].includes(k) || v === undefined) {
      throw new Error(`usage: changes.mjs (--base SHA --head SHA | --files FILE | --full REASON) [--github-output FILE]`);
    }
    opt[k.slice(2)] = v;
  }
  const ws = loadWorkspace(ROOT);
  /** @type {Plan} */
  let p;
  if (opt.full !== undefined) {
    p = fullPlan(ws, opt.full || "requested");
  } else if (opt.files !== undefined) {
    p = plan(readFileSync(opt.files, "utf8").split(/\r?\n/).map((l) => l.trim()).filter((l) => l !== ""), ws);
  } else if (opt.base !== undefined && opt.head !== undefined) {
    p = plan(changedFiles(opt.base, opt.head), ws);
  } else {
    throw new Error("give --base and --head, --files or --full");
  }
  for (const [k, v] of Object.entries(p)) {
    if (/[\r\n]/.test(v)) throw new Error(`output ${k} has a line break`);
  }
  if (opt["github-output"]) {
    appendFileSync(opt["github-output"], Object.entries(p).map(([k, v]) => `${k}=${v}\n`).join(""));
  }
  if (process.env.GITHUB_STEP_SUMMARY) {
    const rows = Object.entries(p).map(([k, v]) => `| \`${k}\` | \`${v.replaceAll("|", "\\|")}\` |`);
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `### CI plan\n\n${p.summary}\n\n| output | value |\n|---|---|\n${rows.join("\n")}\n`);
  }
  process.stdout.write(`${JSON.stringify(p, null, 2)}\n`);
}

if (process.argv[1] && resolvePath(fileURLToPath(import.meta.url)) === resolvePath(process.argv[1])) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    process.stderr.write(`::error::change detection failed: ${error instanceof Error ? error.message : String(error)}\n`);
    process.exit(1);
  }
}
