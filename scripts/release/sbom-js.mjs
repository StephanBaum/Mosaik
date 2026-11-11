#!/usr/bin/env node
/**
 * CycloneDX 1.5 helpers for the release SBOMs (M9 T9, ADR-0086). No dependency beyond Node.
 *
 *   node scripts/release/sbom-js.mjs npm --package apps/extension --out sbom.json
 *       The production dependency tree of one pnpm workspace package (`pnpm list --prod --json
 *       --long --depth Infinity`) with the integrity hashes of pnpm-lock.yaml. `npm` is the
 *       default command.
 *   node scripts/release/sbom-js.mjs normalize <file.json>...
 *       Rewrites the SBOMs `cargo cyclonedx` made: Mosaic's own components get the license
 *       NOASSERTION (OD-M9-11) and the timestamp becomes SOURCE_DATE_EPOCH.
 *   node scripts/release/sbom-js.mjs image --sbom server.json --dockerfile deploy/Dockerfile
 *       --image-ref ghcr.io/<ns>/mosaic-server@sha256:<digest> --out image.json
 *       The server SBOM plus the image digest and every base image of the Dockerfile, by digest.
 *
 * Timestamps come from SOURCE_DATE_EPOCH when set (reproducible output), otherwise the clock.
 */
import { Buffer } from "node:buffer";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const NOASSERTION = [{ license: { name: "NOASSERTION" } }];

const fail = (message) => {
  process.stderr.write(`sbom-js: ${message}\n`);
  process.exit(1);
};

const timestamp = () => {
  const sde = process.env["SOURCE_DATE_EPOCH"];
  return (sde && /^\d+$/.test(sde) ? new Date(Number(sde) * 1000) : new Date()).toISOString();
};

/** A stable UUID from the content, so equal inputs give equal files. */
const serialNumber = (value) => {
  const h = createHash("sha256").update(JSON.stringify(value)).digest("hex");
  return `urn:uuid:${h.slice(0, 8)}-${h.slice(8, 12)}-5${h.slice(13, 16)}-a${h.slice(17, 20)}-${h.slice(20, 32)}`;
};

const args = (list) => {
  /** @type {Record<string, string>} */
  const out = {};
  for (let i = 0; i < list.length; i += 2) {
    const key = list[i];
    if (!key?.startsWith("--") || list[i + 1] === undefined) fail(`bad argument near "${key ?? ""}"`);
    out[key.slice(2)] = list[i + 1];
  }
  return out;
};

const purlOf = (name, version) => `pkg:npm/${name.startsWith("@") ? `%40${name.slice(1)}` : name}@${version}`;

/** integrity strings of pnpm-lock.yaml, keyed "name@version" */
function lockIntegrity() {
  const text = readFileSync(join(root, "pnpm-lock.yaml"), "utf8").split(/\r?\n/);
  /** @type {Map<string, string>} */
  const map = new Map();
  let inPackages = false;
  let key = "";
  for (const line of text) {
    if (/^\S/.test(line)) inPackages = line.startsWith("packages:");
    if (!inPackages) continue;
    const head = /^ {2}'?(@?[^@'\s]+@[^':\s(]+)(?:\([^']*)?'?:\s*$/.exec(line);
    if (head) {
      key = head[1] ?? "";
      continue;
    }
    const res = /^ {4}resolution: \{.*integrity: ([A-Za-z0-9+/=-]+)/.exec(line);
    if (res && key) map.set(key, res[1] ?? "");
  }
  return map;
}

/** @param {string} integrity e.g. "sha512-<base64>" */
function hashOf(integrity) {
  const m = /^(sha1|sha256|sha384|sha512)-(.+)$/.exec(integrity);
  if (!m) return [];
  const alg = { sha1: "SHA-1", sha256: "SHA-256", sha384: "SHA-384", sha512: "SHA-512" }[m[1]];
  return [{ alg, content: Buffer.from(m[2], "base64").toString("hex") }];
}

/** @param {string | undefined} license */
function licensesOf(license) {
  if (!license || license === "Unknown") return undefined;
  return /\s(OR|AND|WITH)\s/.test(license) ? [{ expression: license }] : [{ license: { name: license } }];
}

function npmSbom(options) {
  const pkgDir = resolve(root, options["package"] ?? fail("--package is required"));
  const outFile = options["out"] ?? fail("--out is required");
  const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
  const raw = execFileSync(pnpm, ["list", "--prod", "--json", "--long", "--depth", "Infinity"], {
    cwd: pkgDir,
    encoding: "utf8",
    maxBuffer: 512 * 1024 * 1024,
    shell: process.platform === "win32",
  });
  const project = JSON.parse(raw)[0];
  if (!project) fail("pnpm list returned no project");
  const integrity = lockIntegrity();
  const own = JSON.parse(readFileSync(join(pkgDir, "package.json"), "utf8"));

  /** @type {Map<string, object>} */
  const components = new Map();
  /** @type {Map<string, Set<string>>} */
  const edges = new Map();

  const rootRef = purlOf(own.name, own.version);
  const walk = (parentRef, deps) => {
    for (const [name, dep] of Object.entries(deps ?? {})) {
      let version = String(dep.version ?? "").replace(/\(.*$/, "");
      const isLink = version.startsWith("link:");
      if (isLink) {
        // A workspace package: take its own version, and say NOASSERTION for the license.
        const linked = JSON.parse(readFileSync(join(dep.path, "package.json"), "utf8"));
        version = String(linked.version ?? "0.0.0");
      }
      const ref = purlOf(name, version);
      (edges.get(parentRef) ?? edges.set(parentRef, new Set()).get(parentRef)).add(ref);
      if (!components.has(ref)) {
        const scoped = name.startsWith("@") ? name.split("/") : null;
        components.set(ref, {
          type: "library",
          "bom-ref": ref,
          ...(scoped ? { group: scoped[0], name: scoped[1] } : { name }),
          version,
          purl: ref,
          hashes: isLink ? undefined : hashOf(integrity.get(`${name}@${version}`) ?? ""),
          licenses: isLink ? NOASSERTION : licensesOf(dep.license),
          ...(isLink ? { properties: [{ name: "mosaic:workspace", value: "true" }] } : {}),
        });
        walk(ref, dep.dependencies);
      }
    }
  };
  walk(rootRef, project.dependencies);

  const list = [...components.values()]
    .map((c) => JSON.parse(JSON.stringify(c)))
    .sort((a, b) => a["bom-ref"].localeCompare(b["bom-ref"]));
  const bom = {
    bomFormat: "CycloneDX",
    specVersion: "1.5",
    version: 1,
    metadata: {
      timestamp: timestamp(),
      tools: { components: [{ type: "application", name: "sbom-js.mjs", version: "1" }] },
      component: {
        type: "application",
        "bom-ref": rootRef,
        name: own.name,
        version: own.version,
        purl: rootRef,
        licenses: NOASSERTION,
      },
    },
    components: list,
    dependencies: [rootRef, ...list.map((c) => c["bom-ref"])].map((ref) => ({
      ref,
      dependsOn: [...(edges.get(ref) ?? [])].sort(),
    })),
  };
  bom.serialNumber = serialNumber(bom);
  writeFileSync(outFile, `${JSON.stringify(bom, null, 2)}\n`);
  process.stdout.write(`sbom-js: ${list.length} components for ${own.name} -> ${outFile}\n`);
}

function normalizeRust(files) {
  if (files.length === 0) fail("normalize needs at least one file");
  for (const file of files) {
    const bom = JSON.parse(readFileSync(file, "utf8"));
    const isOwn = (c) => typeof c?.name === "string" && c.name.startsWith("mosaic-") && !c.purl?.includes("crates.io");
    const fix = (c) => {
      if (isOwn(c)) c.licenses = NOASSERTION;
    };
    if (bom.metadata) {
      bom.metadata.timestamp = timestamp();
      fix(bom.metadata.component);
    }
    for (const c of bom.components ?? []) fix(c);
    writeFileSync(file, `${JSON.stringify(bom, null, 2)}\n`);
    process.stdout.write(`sbom-js: normalized ${file}\n`);
  }
}

function imageSbom(options) {
  const sbomFile = options["sbom"] ?? fail("--sbom is required");
  const dockerfile = options["dockerfile"] ?? fail("--dockerfile is required");
  const imageRef = options["image-ref"] ?? fail("--image-ref is required");
  const outFile = options["out"] ?? fail("--out is required");
  if (!/^[a-z0-9./_-]+@sha256:[0-9a-f]{64}$/.test(imageRef)) fail("--image-ref must be name@sha256:<digest>");
  const bom = JSON.parse(readFileSync(sbomFile, "utf8"));
  const bases = [];
  for (const line of readFileSync(dockerfile, "utf8").split(/\r?\n/)) {
    const m = /^FROM\s+(\S+?)@sha256:([0-9a-f]{64})(?:\s+AS\s+(\S+))?\s*$/i.exec(line.trim());
    if (m) bases.push({ name: m[1], digest: m[2], stage: m[3] ?? "runtime" });
  }
  if (bases.length === 0) fail("no digest-pinned FROM line in the Dockerfile");
  const baseComponents = bases.map((b) => ({
    type: "container",
    "bom-ref": `${b.name}@sha256:${b.digest}`,
    name: b.name,
    version: `sha256:${b.digest}`,
    hashes: [{ alg: "SHA-256", content: b.digest }],
    properties: [{ name: "mosaic:dockerStage", value: b.stage }],
  }));
  bom.metadata = bom.metadata ?? {};
  bom.metadata.timestamp = timestamp();
  bom.metadata.properties = [
    ...(bom.metadata.properties ?? []),
    { name: "mosaic:image", value: imageRef },
    { name: "mosaic:imageDigest", value: imageRef.split("@")[1] },
  ];
  bom.components = [...baseComponents, ...(bom.components ?? [])];
  writeFileSync(outFile, `${JSON.stringify(bom, null, 2)}\n`);
  process.stdout.write(`sbom-js: image SBOM with ${bases.length} base image(s) -> ${outFile}\n`);
}

const [first, ...rest] = process.argv.slice(2);
if (first === "normalize") normalizeRust(rest);
else if (first === "image") imageSbom(args(rest));
else if (first === "npm") npmSbom(args(rest));
else if (first?.startsWith("--")) npmSbom(args([first, ...rest]));
else fail("usage: sbom-js.mjs [npm] --package <dir> --out <file> | normalize <file>... | image --sbom ... --out ...");
