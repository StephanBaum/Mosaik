/**
 * Red-team tests of sbom-js.mjs (RT-release, whole-app red team 2026-11-13).
 *
 *   node --test scripts/release/sbom-js.test.mjs
 *
 * Every case runs every time (RT-release-4 is fixed and stays as a regression test). No dependency
 * beyond Node.
 */
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { test } from "node:test";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const script = join(here, "sbom-js.mjs");
const DIGEST = "a".repeat(64);

/** @param {string[]} args @returns {{ status: number; out: string }} */
function run(args, cwd) {
  try {
    const out = execFileSync(process.execPath, [script, ...args], { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
    return { status: 0, out };
  } catch (error) {
    const e = /** @type {{ status: number | null; stdout?: string; stderr?: string }} */ (error);
    return { status: e.status ?? 1, out: `${e.stdout ?? ""}${e.stderr ?? ""}` };
  }
}

function scratch() {
  const dir = mkdtempSync(join(tmpdir(), "rt-sbom-"));
  return { dir, done: () => rmSync(dir, { recursive: true, force: true }) };
}

function serverSbom(dir) {
  const file = join(dir, "server.cdx.json");
  writeFileSync(file, JSON.stringify({ bomFormat: "CycloneDX", specVersion: "1.5", version: 1, metadata: {}, components: [] }));
  return file;
}

test("image: refuses an image reference that is not pinned by digest", () => {
  const { dir, done } = scratch();
  try {
    writeFileSync(join(dir, "Dockerfile"), `FROM rust:1.99.1@sha256:${DIGEST} AS builder\nFROM scratch@sha256:${DIGEST}\n`);
    for (const ref of ["ghcr.io/x/mosaic-server:1.0.0", `ghcr.io/x/mosaic-server@sha256:${DIGEST.slice(1)}`, `ghcr.io/x/mosaic-server@sha512:${DIGEST}`]) {
      const r = run(["image", "--sbom", serverSbom(dir), "--dockerfile", join(dir, "Dockerfile"), "--image-ref", ref, "--out", join(dir, "o.json")]);
      assert.notEqual(r.status, 0, `accepted ${ref}`);
    }
  } finally {
    done();
  }
});

test("image: refuses a Dockerfile whose FROM lines are not pinned by digest", () => {
  const { dir, done } = scratch();
  try {
    writeFileSync(join(dir, "Dockerfile"), "FROM rust:1.99.1-bookworm AS builder\nFROM gcr.io/distroless/cc-debian12:nonroot\n");
    const r = run(["image", "--sbom", serverSbom(dir), "--dockerfile", join(dir, "Dockerfile"), "--image-ref", `ghcr.io/x/mosaic-server@sha256:${DIGEST}`, "--out", join(dir, "o.json")]);
    assert.notEqual(r.status, 0);
    assert.match(r.out, /digest-pinned FROM/);
  } finally {
    done();
  }
});

test("image: records every base image by digest and the image digest itself", () => {
  const { dir, done } = scratch();
  try {
    const b = "b".repeat(64);
    writeFileSync(join(dir, "Dockerfile"), `FROM rust:1.99.1@sha256:${DIGEST} AS builder\nFROM gcr.io/distroless/cc-debian12:nonroot@sha256:${b}\n`);
    const out = join(dir, "o.json");
    const r = run(["image", "--sbom", serverSbom(dir), "--dockerfile", join(dir, "Dockerfile"), "--image-ref", `ghcr.io/x/mosaic-server@sha256:${DIGEST}`, "--out", out]);
    assert.equal(r.status, 0, r.out);
    const bom = JSON.parse(readFileSync(out, "utf8"));
    const containers = bom.components.filter((c) => c.type === "container");
    assert.deepEqual(containers.map((c) => c.hashes[0].content).sort(), [DIGEST, b]);
    assert.ok(bom.metadata.properties.some((p) => p.name === "mosaic:imageDigest" && p.value === `sha256:${DIGEST}`));
  } finally {
    done();
  }
});

test("arguments: a dangling or malformed flag is refused", () => {
  assert.notEqual(run(["image", "--sbom"]).status, 0);
  assert.notEqual(run(["image", "sbom", "x"]).status, 0);
  assert.notEqual(run(["bogus"]).status, 0);
  assert.notEqual(run(["normalize"]).status, 0);
});

test("normalize: only Mosaic's own components lose their license, third parties keep theirs", () => {
  const { dir, done } = scratch();
  try {
    const file = join(dir, "n.json");
    writeFileSync(
      file,
      JSON.stringify({
        metadata: { timestamp: "x", component: { name: "mosaic-cli", purl: "pkg:cargo/mosaic-cli@0.0.0", licenses: [{ license: { id: "MIT" } }] } },
        components: [
          { name: "serde", purl: "pkg:cargo/serde@1.0.0?repository_url=https://crates.io", licenses: [{ license: { id: "MIT" } }] },
          { name: "mosaic-core", purl: "pkg:cargo/mosaic-core@0.0.0", licenses: [{ license: { id: "MIT" } }] },
        ],
      }),
    );
    const r = run(["normalize", file], undefined);
    assert.equal(r.status, 0, r.out);
    const bom = JSON.parse(readFileSync(file, "utf8"));
    assert.equal(bom.metadata.component.licenses[0].license.name, "NOASSERTION");
    assert.equal(bom.components[0].licenses[0].license.id, "MIT");
    assert.equal(bom.components[1].licenses[0].license.name, "NOASSERTION");
  } finally {
    done();
  }
});

// RT-release-4: when `pnpm list` resolves nothing (the package is not installed, or pnpm answers
// with an empty tree), sbom-js used to write an SBOM with zero components and exit 0. It now fails.
test("npm: a declared dependency that is not resolved is an error, not an empty SBOM", () => {
  const { dir, done } = scratch();
  try {
    writeFileSync(join(dir, "package.json"), JSON.stringify({ name: "rt-probe", version: "1.0.0", dependencies: { react: "19.3.1" } }));
    const out = join(dir, "out.cdx.json");
    const r = run(["--package", dir, "--out", out]);
    assert.notEqual(r.status, 0, `sbom-js exited 0: ${r.out}`);
  } finally {
    done();
  }
});
