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
import { dirname, join } from "node:path";
import process from "node:process";
import { URL, fileURLToPath } from "node:url";

export const HTTPS_PORT = 4843;
export const HTTP_PORT = 4880;

const here = dirname(fileURLToPath(import.meta.url));
const PAGE = /^[a-z0-9_][a-z0-9_.-]*\.(html|js)$/;
const TYPES = { ".html": "text/html; charset=utf-8", ".js": "text/javascript; charset=utf-8" };

/** @param {import("node:http").IncomingMessage} req @param {import("node:http").ServerResponse} res */
async function handle(req, res) {
  const host = (req.headers.host ?? "").split(":")[0] ?? "";
  const name = new URL(req.url ?? "/", "http://placeholder.test").pathname.slice(1);
  if (!host.endsWith(".test") || !PAGE.test(name) || name.includes("..")) {
    res.writeHead(host.endsWith(".test") ? 404 : 421).end();
    return;
  }
  try {
    const body = await readFile(join(here, "pages", name));
    res.writeHead(200, {
      "content-type": TYPES[name.slice(name.lastIndexOf("."))] ?? "text/plain",
      "cache-control": "no-store",
      "x-content-type-options": "nosniff",
    });
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

/** Starts both servers; `close()` stops them. */
export async function startServer() {
  const tls = {
    key: readFileSync(join(here, "certs", "key.pem")),
    cert: readFileSync(join(here, "certs", "cert.pem")),
  };
  const servers = [createHttps(tls, handle), createHttp(handle)];
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
