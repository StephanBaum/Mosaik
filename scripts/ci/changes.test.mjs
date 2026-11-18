/**
 * Self-test of the change detection (docs/CI.md, ADR-0169). The `Detect changes` job runs it before
 * it computes the plan, so a broken detection fails the required jobs.
 *
 *   node --test scripts/ci/changes.test.mjs
 *
 * No dependency beyond Node and git.
 */
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { join, posix } from "node:path";
import { test } from "node:test";
import { COVERAGE, FUZZ_DIRS, OSES, ROOT, classify, loadWorkspace, parseManifest, plan } from "./changes.mjs";

const ws = loadWorkspace(ROOT);
const ALL = OSES.join(" ");
const LW = "ubuntu-latest windows-latest";

/** @param {string[]} files */
const p = (files) => plan(files, ws);
/** @param {string} json @returns {string[]} */
const crates = (json) => /** @type {{ crate: string }[]} */ (JSON.parse(json)).map((c) => c.crate);

/** Every required job is a no-op, the wording check aside (it runs in TypeScript (ubuntu-latest) always). */
function assertNothing(/** @type {ReturnType<typeof p>} */ r) {
  assert.equal(r.full, "false");
  for (const k of ["rust_os", "rust_args", "rust_release_args", "cli_os", "desktop_rust_os", "ts_os"]) {
    assert.equal(r[/** @type {"rust_os"} */ (k)], "", k);
  }
  for (const k of ["desktop_frontend", "extension", "web", "server_image", "supply_chain", "release", "ios"]) {
    assert.equal(r[/** @type {"web"} */ (k)], "false", k);
  }
  assert.equal(r.coverage, "[]");
  assert.equal(r.fuzz, "[]");
}

test("documentation and no-effect files run nothing", () => {
  assertNothing(p([]));
  assertNothing(p(["docs/PROGRESS.md", "docs/design/mock.html", "README.md", "SECURITY.md", "CLAUDE.md"]));
  assertNothing(p(["marketing/positioning.md", ".claude/agents/ux-reviewer.md", ".gitignore", ".vscode/extensions.json"]));
});

test("run-everything files and unmapped files give a full plan", () => {
  for (const f of [
    ".github/workflows/ci.yml",
    ".github/dependabot.yml",
    "scripts/ci/changes.mjs",
    "rust-toolchain.toml",
    "Cargo.lock",
    "Cargo.toml",
    "deny.toml",
    "pnpm-lock.yaml",
    "package.json",
    "tsconfig.base.json",
    "eslint.config.js",
    ".cargo/audit.toml",
    "some-new-dir/file.txt",
    "crates/brand-new-crate/src/lib.rs",
    "crates/README.txt",
  ]) {
    const r = p(["docs/BRIEF.md", f]);
    assert.equal(r.full, "true", f);
    assert.equal(r.rust_os, ALL, f);
    assert.deepEqual(crates(r.coverage), COVERAGE.map((c) => c.crate), f);
    assert.deepEqual(JSON.parse(r.fuzz), FUZZ_DIRS, f);
    assert.equal(r.rust_args, "--workspace --exclude mosaic-desktop", f);
  }
});

test("core and vault count as all Rust; macOS stays off", () => {
  for (const f of ["crates/core/src/lib.rs", "crates/vault/src/store.rs"]) {
    const r = p([f]);
    assert.equal(r.full, "false");
    assert.equal(r.rust_os, LW);
    assert.equal(r.rust_args, "--workspace --exclude mosaic-desktop");
    assert.match(r.rust_release_args, /--exclude mosaic-mobile --exclude mosaic-mobile-ffi$/);
    assert.equal(r.cli_os, LW);
    assert.equal(r.desktop_rust_os, LW);
    assert.equal(r.server_image, "true");
    assert.equal(r.ts_os, "");
  }
  assert.deepEqual(JSON.parse(p(["crates/core/src/lib.rs"]).fuzz), FUZZ_DIRS);
});

test("a leaf crate runs only itself and its dependents", () => {
  const r = p(["crates/program-fill/src/lib.rs"]);
  assert.equal(r.rust_args, "-p mosaic-program-fill");
  assert.equal(r.rust_os, LW);
  assert.equal(r.desktop_rust_os, LW);
  assert.equal(r.cli_os, "");
  assert.equal(r.server_image, "false");
  assert.deepEqual(JSON.parse(r.fuzz), ["apps/desktop/src-tauri/fuzz"]);
  assert.deepEqual(crates(r.coverage), []);
});

test("dev-dependencies count for tests, but not transitively", () => {
  // passkey is a dev-dependency of the server: the server tests run, the tests of its users do not.
  const r = p(["crates/passkey/src/lib.rs"]);
  const scope = r.rust_args.split(" ").filter((x) => x !== "-p");
  assert.ok(scope.includes("mosaic-server"));
  assert.ok(scope.includes("mosaic-import")); // normal dependency
  assert.ok(!scope.includes("mosaic-sync")); // builds on the server only in its own tests
  assert.ok(!scope.includes("mosaic-platform"));
  assert.equal(r.server_image, "false"); // the image needs normal dependencies only
  // The server is a dev-dependency of sync: sync's coverage (which runs the server tests) and the CLI run.
  const s = p(["crates/server/src/lib.rs"]);
  assert.ok(crates(s.coverage).includes("mosaic-sync"));
  assert.ok(crates(s.coverage).includes("mosaic-server"));
  assert.equal(s.server_image, "true");
  assert.equal(s.cli_os, LW);
  assert.ok(!s.rust_args.includes("mosaic-platform")); // platform builds on sync, not on the server
});

test("macOS runs only for macOS areas", () => {
  for (const f of ["crates/platform/src/keystore.rs", "crates/native-messaging/src/install.rs", "crates/cli/src/cmd/sync.rs"]) {
    assert.equal(p([f]).rust_os, ALL, f);
  }
  assert.equal(p(["crates/cli/src/main.rs"]).cli_os, ALL);
  assert.equal(p(["crates/cli/i18n/en.json"]).cli_os, LW);
  const d = p(["apps/desktop/src-tauri/src/lib.rs"]);
  assert.equal(d.desktop_rust_os, ALL);
  assert.equal(d.rust_os, "ubuntu-latest"); // fmt and the unsafe check; no other crate builds on the app
  assert.equal(d.rust_args, "");
  assert.deepEqual(JSON.parse(d.fuzz), ["apps/desktop/src-tauri/fuzz"]);
});

test("TypeScript areas", () => {
  const web = p(["apps/web/src/main.ts", "apps/web/public/img/a.png"]);
  assert.equal(web.ts_os, ALL);
  assert.equal(web.web, "true");
  assert.equal(web.desktop_frontend, "false");
  assert.equal(web.extension, "false");
  assert.equal(web.rust_os, "");
  const ext = p(["apps/extension/src/popup/main.tsx"]);
  assert.equal(ext.extension, "true");
  assert.equal(ext.desktop_frontend, "false");
  const desk = p(["apps/desktop/src/App.tsx"]);
  assert.equal(desk.desktop_frontend, "true");
  assert.equal(desk.extension, "false");
  assert.equal(desk.desktop_rust_os, "");
  for (const f of ["packages/i18n/src/index.ts", "packages/design-tokens/src/tokens.ts"]) {
    const r = p([f]);
    assert.equal(r.ts_os, ALL);
    assert.equal(r.desktop_frontend, "true");
    assert.equal(r.extension, "true");
    assert.equal(r.web, "true");
    assert.equal(r.ios, "true");
    assert.equal(r.rust_os, "");
  }
});

test("files other areas' tests read", () => {
  assert.equal(p(["packages/i18n/src/catalog/de/insights.ts"]).rust_args, "-p mosaic-insights");
  assert.equal(p(["packages/i18n/src/catalog/en/sync.ts"]).desktop_rust_os, LW);
  assert.equal(p(["packages/i18n/src/catalog/en/web/home.ts"]).desktop_rust_os, "");
  assert.equal(p(["apps/desktop/src/ipc/contract.fixtures.ts"]).desktop_rust_os, LW);
  assert.equal(p(["apps/desktop/src/components/icons/paths.ts"]).extension, "true");
  assert.equal(p(["apps/extension/keys/dev-public-key.b64"]).rust_args, "-p mosaic-native-messaging");
  assert.ok(p(["apps/desktop/src-tauri/tauri.conf.json"]).rust_args.includes("mosaic-platform"));
  assert.equal(p(["apps/desktop/src-tauri/tauri.conf.json"]).desktop_frontend, "true");
  assert.equal(p(["crates/passkey/fixtures/vectors.json"]).extension, "true");
  assert.equal(p(["crates/server/assets/webauthn/page.js"]).ts_os, ALL);
});

test("fuzz, supply chain, release and server image", () => {
  const f = p(["crates/totp/fuzz/Cargo.lock"]);
  assert.deepEqual(JSON.parse(f.fuzz), ["crates/totp/fuzz"]);
  assert.equal(f.supply_chain, "true");
  assert.equal(f.release, "true");
  assert.equal(f.rust_os, "ubuntu-latest");
  assert.equal(f.rust_args, "");
  assert.equal(p(["crates/agent/fuzz/fuzz_targets/x.rs"]).fuzz, "[]"); // not in CI before ADR-0169 either
  assert.equal(p(["crates/sync/Cargo.toml"]).supply_chain, "true");
  assert.equal(p(["scripts/release/sign.sh"]).release, "true");
  assert.equal(p(["scripts/release/sign.sh"]).supply_chain, "false");
  assert.equal(p(["scripts/release/sbom-js.mjs"]).ts_os, ALL); // ESLint
  assert.equal(p(["deploy/Dockerfile"]).server_image, "true");
  assert.equal(p(["deploy/Dockerfile"]).rust_os, "");
  assert.equal(p(["vendor/rqrr/src/lib.rs"]).rust_args, "--workspace --exclude mosaic-desktop");
  assert.equal(p(["tools/check-unsafe-confined.sh"]).rust_os, LW);
});

test("mobile", () => {
  const a = p(["apps/mobile/android/app/src/main/kotlin/Main.kt"]);
  assert.equal(a.ts_os, "ubuntu-latest"); // the export drift check
  assert.equal(a.ios, "false");
  assert.equal(p(["apps/mobile/ios/Mosaic/App.swift"]).ios, "true");
  assert.equal(p(["crates/mobile-ffi/src/lib.rs"]).ios, "true");
  assert.equal(p(["scripts/mobile/build-ios-xcframework.sh"]).ios, "true");
  assert.equal(p(["scripts/mobile/build-android-native.sh"]).ios, "false");
  assert.equal(p(["crates/sync/src/lib.rs"]).ios, "false"); // the host tests and Android cover it; main runs iOS
  const m = p(["crates/mobile/src/lib.rs"]);
  assert.equal(m.rust_args, "-p mosaic-mobile -p mosaic-mobile-ffi");
  assert.equal(m.rust_release_args, ""); // the mobile crates have no ignored tests
});

test("manifest parsing", () => {
  const m = parseManifest(
    [
      '[package]\nname = "x"',
      "[dependencies]",
      'a = { path = "../a" } # comment',
      "serde.workspace = true",
      "[target.'cfg(windows)'.dependencies]",
      'b = { path = "../b", optional = true }',
      "[dev-dependencies.c]",
      'path = "../c"',
      "[features]",
      'f = ["mosaic-a/x"]',
    ].join("\n"),
    "t",
  );
  assert.equal(m.name, "x");
  assert.deepEqual(m.deps, [
    { kind: "normal", path: "../a" },
    { kind: "normal", path: "../b" },
    { kind: "dev", path: "../c" },
  ]);
  assert.throws(() => parseManifest("[dependencies]\nmosaic-core.workspace = true", "t"), /without a path/);
  assert.throws(() => parseManifest('[dependencies]\nmosaic-core = "1"', "t"), /without a path/);
});

test("the lists match the repository", () => {
  for (const c of COVERAGE) assert.ok(ws.crates.has(c.crate), c.crate);
  for (const d of FUZZ_DIRS) assert.ok(ws.fuzz.has(d) && existsSync(join(ROOT, d, "Cargo.toml")), d);
  assert.ok(ws.crates.has("mosaic-desktop"));
  assert.ok(ws.crates.has("mosaic-cli"));
  assert.ok(ws.crates.has("mosaic-server"));
});

test("every tracked file maps without error", () => {
  const files = execFileSync("git", ["ls-files", "-z"], { cwd: ROOT, encoding: "utf8", maxBuffer: 64 << 20 })
    .split("\0")
    .filter((f) => f !== "");
  assert.ok(files.length > 100);
  for (const f of files) classify(f, ws);
});

/**
 * Cross-area file references: a source file in one area that reads a file of another area. Each must
 * be covered by the map (a change to the target runs the job of the referring file); this test fails
 * when a new one appears, so the map gets updated in the same pull request.
 */
test("cross-area file references are covered by the map", () => {
  const files = execFileSync("git", ["ls-files", "-z", "--", "crates", "apps", "packages"], {
    cwd: ROOT,
    encoding: "utf8",
    maxBuffer: 64 << 20,
  })
    .split("\0")
    .filter((f) => /\.(?:rs|[cm]?[jt]s|tsx)$/.test(f) && !f.startsWith("apps/mobile/"));
  const tracked = new Set(
    execFileSync("git", ["ls-files", "-z"], { cwd: ROOT, encoding: "utf8", maxBuffer: 64 << 20 }).split("\0"),
  );
  const trackedList = [...tracked];
  const trackedDirs = new Set(trackedList.flatMap((t) => t.split("/").slice(0, -1).map((_, i, a) => a.slice(0, i + 1).join("/"))));
  /** Directory references whose code reads only these files (checked instead of every file below). */
  const DIR_READS = {
    "packages/i18n/src/catalog": ["packages/i18n/src/catalog/en/sync.ts", "packages/i18n/src/catalog/de/sharing.ts"],
  };
  /** @param {string} f */
  const areaOf = (f) => {
    const s = f.split("/");
    if (s[0] === "apps" && s[1] === "desktop" && s[2] === "src-tauri") return s[3] === "fuzz" ? s.slice(0, 4).join("/") : s.slice(0, 3).join("/");
    if (s[0] === "crates" && s[2] === "fuzz") return s.slice(0, 3).join("/");
    return s.slice(0, 2).join("/");
  };
  /** The jobs of the referring file that a change of `target` must turn on. @param {string} from @param {string} target */
  const covered = (from, target) => {
    const r = p([target]);
    if (r.full === "true") return true;
    if (from.endsWith(".rs")) {
      const area = areaOf(from);
      if (area.endsWith("/fuzz")) return JSON.parse(r.fuzz).includes(area);
      const crate = ws.byDir.get(area)?.name;
      if (crate === "mosaic-desktop") return r.desktop_rust_os !== "";
      if (crate === "mosaic-cli") return r.cli_os !== "";
      return crate !== undefined && r.rust_args.split(" ").includes(crate) || r.rust_args.startsWith("--workspace");
    }
    if (from.startsWith("apps/extension/")) return r.extension === "true";
    if (from.startsWith("apps/desktop/")) return r.desktop_frontend === "true";
    if (from.startsWith("apps/web/")) return r.web === "true";
    return r.ts_os !== "";
  };
  /** @type {string[]} */
  const uncovered = [];
  let found = 0;
  for (const from of files) {
    const text = readFileSync(join(ROOT, from), "utf8");
    const area = areaOf(from);
    const crateRoot = ws.byDir.get(area)?.dir ?? (from.startsWith("apps/") || from.startsWith("packages/") ? from.split("/").slice(0, 2).join("/") : area);
    for (const line of text.split(/\r?\n/)) {
      if (/^\s*(?:\/\/|\*|\/\*)/.test(line)) continue;
      for (const m of line.matchAll(/["'](\/?\.\.?\/[^"'\s]*)["']/g)) {
        const rel = m[1].replace(/^\//, "");
        // Relative to the file (include_str!, #[path], imports) or to the crate or package root
        // (CARGO_MANIFEST_DIR joins).
        for (const baseDir of [posix.dirname(from), crateRoot]) {
          const target = posix.normalize(posix.join(baseDir, rel)).replace(/\/$/, "");
          if (target.startsWith("..") || areaOf(target) === area) continue;
          const isFile = tracked.has(target);
          const isDir = !isFile && trackedDirs.has(target);
          if (!isFile && !isDir) continue;
          // TypeScript importing a shared package is covered by the packages rule; a fuzz crate builds on its parent.
          if (!from.endsWith(".rs") && target.startsWith("packages/")) continue;
          if (area.endsWith("/fuzz") && target.startsWith(area.slice(0, -"/fuzz".length))) continue;
          found += 1;
          const targets = isFile
            ? [target]
            : (DIR_READS[/** @type {keyof typeof DIR_READS} */ (target)] ?? trackedList.filter((t) => t.startsWith(`${target}/`)));
          for (const t of targets) if (!covered(from, t)) uncovered.push(`${from} reads ${t}`);
          break;
        }
      }
    }
  }
  assert.ok(found >= 8, `expected the known cross-area references, found ${found}`);
  assert.deepEqual(uncovered, [], "update classify() in scripts/ci/changes.mjs (and docs/CI.md)");
});
