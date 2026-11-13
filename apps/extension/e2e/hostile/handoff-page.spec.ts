/**
 * The sync server's security-key handoff page as a web target (M9 T12; CRYPTO_SPEC §9.3,
 * ADR-0077). The committed assets (crates/server/assets/webauthn) are served on
 * https://sync.test with the headers of crates/server/src/api/page.rs (the server test
 * `page_headers_are_strict` pins the same values), and the API is answered by the test:
 * - XSS: a hostile device name and account name from the options answer stay text;
 * - leaks: the page's requests carry no Referer, no cookie and no handoff ID in a URL;
 * - history: the handoff ID does not stay in the address bar (and so in history) once read;
 * - framing: another site cannot frame the page.
 * No extension is loaded: plain Chromium.
 */
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test, type Page, type Request, type Route } from "@playwright/test";

const ASSETS = resolve(import.meta.dirname, "../../../../crates/server/assets/webauthn");
const FRAMER = resolve(import.meta.dirname, "../../test/hostile-site/pages/handoff-framer.html");
const SYNC = "https://sync.test";
const EVIL = "https://evil.test";
const HANDOFF_ID = "AAECAwQFBgcICQoLDA0ODw";
const CODE = "123456";

/** crates/server/src/api/page.rs `HEADERS`. */
const PAGE_HEADERS: Record<string, string> = {
  "content-security-policy":
    "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
  "referrer-policy": "no-referrer",
  "cache-control": "no-store",
  "x-content-type-options": "nosniff",
  "cross-origin-opener-policy": "same-origin",
  "permissions-policy": "publickey-credentials-get=(self), publickey-credentials-create=(self)",
};

const FILES: Record<string, [string, string]> = {
  "/webauthn/": ["index.html", "text/html; charset=utf-8"],
  "/webauthn/page.js": ["page.js", "text/javascript; charset=utf-8"],
  "/webauthn/page.css": ["page.css", "text/css; charset=utf-8"],
  "/webauthn/strings.json": ["strings.json", "application/json"],
};

const HOSTILE_DEVICE = `<img src=x onerror="window.__xss='device'">Laptop`;
const HOSTILE_ACCOUNT = `"><svg onload="window.__xss='account'"></svg>@example.com`;

interface Seen {
  url: string;
  method: string;
  headers: Record<string, string>;
  body: string | null;
}

/** Serves the page on SYNC, answers the two handoff routes, and records every request. */
async function serve(page: Page): Promise<Seen[]> {
  const seen: Seen[] = [];
  await page.context().addCookies([{ name: "session", value: "ambient", domain: "sync.test", path: "/", secure: true }]);
  await page.route(`${SYNC}/**`, async (route: Route, request: Request) => {
    const url = new URL(request.url());
    seen.push({ url: request.url(), method: request.method(), headers: await request.allHeaders(), body: request.postData() });
    const file = FILES[url.pathname];
    if (file !== undefined && request.method() === "GET") {
      await route.fulfill({
        status: 200,
        headers: { ...PAGE_HEADERS, "content-type": file[1] },
        body: readFileSync(resolve(ASSETS, file[0])),
      });
      return;
    }
    if (url.pathname === "/v1/webauthn/handoff/options" && request.method() === "POST") {
      await route.fulfill({
        status: 200,
        headers: { "content-type": "application/json", "cache-control": "no-store" },
        body: JSON.stringify({
          purpose: "confirm",
          deviceName: HOSTILE_DEVICE,
          requestedAt: 1_800_000_000_000,
          publicKey: {
            challenge: "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8",
            rpId: "sync.test",
            timeout: 300_000,
            userVerification: "preferred",
            allowCredentials: [{ type: "public-key", id: "AQIDBAUGBwgJCgsMDQ4PEA" }],
            user: { id: "AQIDBAUGBwgJCgsMDQ4PEA", name: HOSTILE_ACCOUNT, displayName: HOSTILE_ACCOUNT },
          },
        }),
      });
      return;
    }
    await route.fulfill({ status: 404, headers: { "content-type": "application/json" }, body: '{"error":"not_found"}' });
  });
  return seen;
}

test("hostile device and account names stay text, and requests carry no ID, Referer or cookie", async ({ page }) => {
  const seen = await serve(page);
  await page.goto(`${SYNC}/webauthn/#${HANDOFF_ID}`);
  await page.locator("#code").fill(CODE);
  await page.locator("#code-submit").click();
  await expect(page.locator("#fact-device")).toHaveText(HOSTILE_DEVICE);
  await expect(page.locator("#fact-account")).toHaveText(HOSTILE_ACCOUNT);
  expect(await page.locator("#app img, #app svg").count()).toBe(0);
  await page.waitForTimeout(300);
  expect(await page.evaluate(() => (window as unknown as { __xss?: string }).__xss)).toBeUndefined();

  const posts = seen.filter((s) => s.method === "POST");
  expect(posts.map((p) => new URL(p.url).pathname)).toEqual(["/v1/webauthn/handoff/options"]);
  for (const s of seen) {
    expect(s.url.startsWith(`${SYNC}/`), s.url).toBe(true);
    expect(s.url.includes(HANDOFF_ID), `ID in the URL of ${s.url}`).toBe(false);
    expect(s.headers["referer"], `Referer on ${s.url}`).toBeUndefined();
  }
  for (const p of posts) {
    expect(p.headers["cookie"], "the page sends no ambient cookie").toBeUndefined();
    expect(JSON.parse(p.body ?? "{}")).toEqual({ handoffId: HANDOFF_ID, code: CODE });
  }
});

// M9-F72: the page reads the handoff ID from the fragment and leaves it there, so the address bar,
// the browser history (and a synced history) keep `/webauthn/#<id>` after the page has read it.
// Defense: page.js `start()` clears the fragment with `history.replaceState` after reading the ID; the
// desktop's Re-open passes the ID again.
test("the handoff ID leaves the address bar and history once the page has read it (M9-F72)", async ({ page }) => {
  await serve(page);
  await page.goto(`${SYNC}/webauthn/#${HANDOFF_ID}`);
  await expect(page.locator("#code-form")).toBeVisible();
  expect(page.url()).not.toContain(HANDOFF_ID);
  expect(await page.evaluate(() => location.hash)).toBe("");
});

test("another site cannot frame the handoff page", async ({ page }) => {
  await serve(page);
  await page.route(`${EVIL}/**`, async (route: Route) => {
    await route.fulfill({ status: 200, headers: { "content-type": "text/html; charset=utf-8" }, body: readFileSync(FRAMER) });
  });
  const target = `${SYNC}/webauthn/#${HANDOFF_ID}`;
  await page.goto(`${EVIL}/handoff-framer.html?${new URLSearchParams({ target }).toString()}`);
  await page.waitForLoadState("load");
  await page.waitForTimeout(500);
  const framed = page.frames().filter((f) => f !== page.mainFrame());
  expect(framed).toHaveLength(1);
  // A blocked frame holds Chromium's error document: no page script ran, no code field exists.
  const rendered = await framed[0]
    ?.evaluate(() => document.getElementById("code-form") !== null || document.getElementById("heading")?.textContent !== undefined)
    .catch(() => false);
  expect(rendered).toBe(false);
});
