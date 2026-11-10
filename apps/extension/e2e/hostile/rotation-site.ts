/**
 * Shared parts of the hostile guided-password-change suite (e2e/hostile/rotation-*.spec.ts).
 *
 * The hostile pages live next to the specs in `rotation-pages/` and are served by a page route
 * on any host and port (`https://<any>/rotation-pages/<name>.html`), so a test can put them on a
 * sibling subdomain, a look-alike, an IDN host, another port, a private public suffix or http.
 * Requests the route does not name go to the hostile-site server as usual.
 */
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import type { Page } from "@playwright/test";
import { BANK, bankItem, expect, inspect, type Fake } from "../fixtures.ts";

const PAGES = resolve(import.meta.dirname, "rotation-pages");
const LEAKS = resolve(import.meta.dirname, "../../test/hostile-site/pages/_leaks.js");

export const IDS = { vaultId: "v1", itemId: "i1", attemptId: "a0000000000000001.e2e.0" } as const;
export const NEW_VALUE = "Gen-Pw-7731!xYzQ-1";

export const attempt = (over: Partial<{ attemptId: string; value: string; itemId: string }> = {}) => ({
  ...IDS,
  value: NEW_VALUE,
  stage: "prepared" as const,
  ...over,
});

/** The fake desktop with one open change for the bank login (or the given item). */
export const rotationConfig = (item = bankItem(), attempts = [attempt()]) => ({
  items: [item],
  rotation: { attempts, begin: "started" as const, result: "confirmed" as const, addOrigin: "added" as const },
});

/** Serves `rotation-pages/` (and `_leaks.js`) on every host of this page, plus the given 302 redirects. */
export async function serveRotationPages(page: Page, redirects: Record<string, string> = {}): Promise<void> {
  await page.route(/\/rotation-pages\/[a-z0-9-]+\.html(\?.*)?$/, async (route) => {
    const name = new URL(route.request().url()).pathname.split("/").pop() ?? "";
    await route.fulfill({
      status: 200,
      contentType: "text/html; charset=utf-8",
      headers: { "cache-control": "no-store" },
      body: await readFile(join(PAGES, name)),
    });
  });
  await page.route(/\/_leaks\.js$/, async (route) => {
    await route.fulfill({ status: 200, contentType: "text/javascript; charset=utf-8", body: await readFile(LEAKS) });
  });
  // A fulfilled 302 is followed outside the route (to the network), so the site's redirect is a
  // page that replaces itself: the next address is a new request the route serves again.
  for (const [from, to] of Object.entries(redirects)) {
    await page.route(from, async (route) => {
      await route.fulfill({
        status: 200,
        contentType: "text/html; charset=utf-8",
        headers: { "cache-control": "no-store" },
        body: `<!-- attempts: send the tab on to another address (the site's change-page redirect). -->\n<script>location.replace(${JSON.stringify(to)})</script>`,
      });
    });
  }
}

/** "Fill change form" for `ids` on `tabId`, as the popup sends it; settles to "filled" or the error text. */
export const changeOutcome = (
  fake: Fake,
  tabId: number,
  over: Partial<{ attemptId: string; itemId: string; confirmFrame: { frameId: number; documentId: string | null } | null }> = {},
): Promise<string> =>
  fake
    .popupRequest(tabId, { op: "fillChange", ...IDS, confirmFrame: null, ...over })
    .then(() => "filled", (error: unknown) => (error instanceof Error ? error.message : String(error)));

export const asked = async (fake: Fake, op: string): Promise<number> =>
  (await fake.log()).filter((entry) => entry.op === op).length;

/** No input of any frame, and no value a page script saw arrive, holds a secret of the change. */
export async function expectNoSecretInPage(page: Page, secrets: readonly string[] = [NEW_VALUE, bankItem().password]): Promise<void> {
  for (const frame of await inspect(page)) {
    for (const secret of secrets) {
      expect(Object.values(frame.values), `values in ${frame.url}`).not.toContain(secret);
      expect(frame.leaks.map((leak) => leak.value), `leaks in ${frame.url}`).not.toContain(secret);
    }
  }
}

/** The ids of the inputs (in every frame) that hold `secret`. */
export async function fieldsHolding(page: Page, secret: string): Promise<string[]> {
  return (await inspect(page)).flatMap((frame) =>
    Object.entries(frame.values)
      .filter(([, value]) => value === secret)
      .map(([id]) => id),
  );
}

export { BANK };
