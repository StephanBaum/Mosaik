/**
 * The hostile-page server of the extension's end-to-end tests. It serves `pages/` on
 * `https://*.test:4843` (the test-only certificate in certs/) and `http://*.test:4880`; the
 * test browser maps `*.test` to 127.0.0.1. Page content never depends on the host name: pages
 * that need another origin build its URL from `location`.
 *
 *   node apps/extension/test/hostile-site/server.mjs      (stays up until Ctrl+C)
 */
import { readFileSync } from "node:fs";
import { createServer as createHttp } from "node:http";
import { createServer as createHttps } from "node:https";
import { readFile } from "node:fs/promises";
import { Buffer } from "node:buffer";
import { randomBytes } from "node:crypto";
import { dirname, join } from "node:path";
import process from "node:process";
import { URL, fileURLToPath } from "node:url";

export const HTTPS_PORT = 4843;
export const HTTP_PORT = 4880;

const here = dirname(fileURLToPath(import.meta.url));
const PAGE = /^(passkey\/)?[a-z0-9_][a-z0-9_.-]*\.(html|js)$/;
const TYPES = { ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8" };

/**
 * The relying party of the passkey tests (M5): a single-use challenge per request and a verify
 * endpoint that checks what the page got with an independent WebAuthn verifier (injected by
 * the test harness; `test/webauthn/verifier.ts`). A challenge is consumed by the first verify
 * that names it, whatever the outcome, so a replayed response fails.
 *
 * @typedef {{ verifyRegistration: (input: object) => { jwk: object, credentialId: Uint8Array, flags: number, signCount: number },
 *   verifyAssertion: (input: object) => { flags: number, signCount: number } }} PasskeyVerifier
 */
function relyingParty(/** @type {PasskeyVerifier | undefined} */ verifier) {
  const challenges = new Set();
  const credentials = new Map();
  const bytes = (/** @type {string} */ value) => new Uint8Array(Buffer.from(value, "base64url"));
  const json = (/** @type {import("node:http").ServerResponse} */ res, status, /** @type {object} */ body) => {
    res.writeHead(status, { "content-type": "application/json", "cache-control": "no-store" });
    res.end(JSON.stringify(body));
  };
  const readBody = (/** @type {import("node:http").IncomingMessage} */ req) =>
    new Promise((resolve) => {
      /** @type {Buffer[]} */
      const chunks = [];
      req.on("data", (chunk) => chunks.push(chunk));
      req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    });
  /** The verify step proper; throws the verifier's message on a failed check. */
  const check = (host, body) => {
    const expectedOrigin = `https://${host}`;
    const expectedRpId = host.split(":")[0];
    const common = {
      expectedOrigin,
      expectedRpId,
      expectedChallenge: bytes(body.challenge),
      clientDataJSON: bytes(body.clientDataJSON),
    };
    if (body.kind === "registration") {
      const result = verifier.verifyRegistration({ ...common, attestationObject: bytes(body.attestationObject) });
      credentials.set(`${host}/${Buffer.from(result.credentialId).toString("base64url")}`, result.jwk);
      return { flags: result.flags, signCount: result.signCount, credentialId: Buffer.from(result.credentialId).toString("base64url") };
    }
    const jwk = credentials.get(`${host}/${body.credentialId}`);
    if (jwk === undefined) {
      throw new Error("unknown credential");
    }
    const result = verifier.verifyAssertion({
      ...common,
      authenticatorData: bytes(body.authenticatorData),
      signature: bytes(body.signature),
      jwk,
    });
    return { flags: result.flags, signCount: result.signCount, credentialId: body.credentialId };
  };
  return async (/** @type {import("node:http").IncomingMessage} */ req, /** @type {import("node:http").ServerResponse} */ res, /** @type {string} */ name) => {
    if (name === "passkey/challenge") {
      const challenge = randomBytes(32).toString("base64url");
      challenges.add(challenge);
      json(res, 200, { challenge });
    } else if (verifier === undefined) {
      json(res, 501, { ok: false, error: "no verifier" });
    } else {
      const body = JSON.parse(String(await readBody(req)));
      // Single use, even when the check below fails.
      if (!challenges.delete(body.challenge)) {
        json(res, 409, { ok: false, error: "challenge unknown or already used" });
        return;
      }
      try {
        json(res, 200, { ok: true, ...check(req.headers.host ?? "", body) });
      } catch (error) {
        json(res, 422, { ok: false, error: error instanceof Error ? error.message : "failed" });
      }
    }
  };
}

/**
 * @param {import("node:http").IncomingMessage} req
 * @param {import("node:http").ServerResponse} res
 * @param {ReturnType<typeof relyingParty>} passkey
 */
async function handle(req, res, passkey) {
  const host = (req.headers.host ?? "").split(":")[0] ?? "";
  const name = new URL(req.url ?? "/", "http://placeholder.test").pathname.slice(1);
  if (host.endsWith(".test") && (name === "passkey/challenge" || name === "passkey/verify")) {
    await passkey(req, res, name);
    return;
  }
  if (!host.endsWith(".test") || !PAGE.test(name) || name.includes("..")) {
    res.writeHead(host.endsWith(".test") ? 404 : 421).end();
    return;
  }
  try {
    const body = await readFile(join(here, "pages", name));
    /** @type {Record<string, string>} */
    const headers = {
      "content-type": TYPES[name.slice(name.lastIndexOf("."))] ?? "text/plain",
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
    };
    if (name.startsWith("csp-sandbox")) {
      // How a site hosts untrusted HTML on its own origin: the document gets an opaque origin.
      headers["content-security-policy"] = "sandbox allow-scripts allow-forms";
    }
    res.writeHead(200, headers);
    res.end(body);
  } catch {
    res.writeHead(404).end();
  }
}

/** @param {import("node:http").Server} server @param {number} port */
const listen = (server, port) =>
  new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(port, "127.0.0.1", () => {
      resolve(undefined);
    });
  });

/**
 * Starts both servers; `close()` stops them. `options.passkeyVerifier` is the WebAuthn verifier
 * behind `/passkey/verify` (the test harness passes `test/webauthn/verifier.ts`).
 *
 * @param {{ passkeyVerifier?: PasskeyVerifier }} [options]
 */
export async function startServer(options = {}) {
  const passkey = relyingParty(options.passkeyVerifier);
  const route = (/** @type {import("node:http").IncomingMessage} */ req, /** @type {import("node:http").ServerResponse} */ res) =>
    handle(req, res, passkey);
  const tls = {
    key: readFileSync(join(here, "certs", "key.pem")),
    cert: readFileSync(join(here, "certs", "cert.pem")),
  };
  const servers = [createHttps(tls, route), createHttp(route)];
  await Promise.all([listen(servers[0], HTTPS_PORT), listen(servers[1], HTTP_PORT)]);
  return {
    httpsPort: HTTPS_PORT,
    httpPort: HTTP_PORT,
    close: async () => {
      for (const server of servers) {
        server.closeAllConnections();
      }
      await Promise.all(servers.map((server) => new Promise((resolve) => server.close(() => resolve(undefined)))));
    },
  };
}

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1]) {
  await startServer();
  process.stdout.write(`hostile-site: https://*.test:${String(HTTPS_PORT)} and http://*.test:${String(HTTP_PORT)}\n`);
}
