/**
 * M4-R10 red team (M9 T12): only the popup's own port reads or changes the "never offered for
 * saving" list. Each test seeds the list, attacks it, and reads the background's store directly:
 * - another installed extension's page: runtime.sendMessage and a port named "popup" to Mosaic;
 * - another extension's content script in a web page, with the same calls;
 * - the web page itself: postMessage in the popup envelope's shape and in the passkey bridge's
 *   shapes (the bridge is the one Mosaic script that listens to page messages);
 * - the popup page framed by the web page (to reach its port or its Remove buttons).
 * Expected defense: every attempt is refused or dropped, and the list is unchanged.
 */
import type { Page, Worker } from "@playwright/test";
import { EVIL, expect, open, test } from "../fixtures.ts";

test.use({ evilExtension: true });

const LISTED = ["https://bank.test:4843", "https://shop.test"] as const;

/** Opens the extension's IndexedDB (`mosaic`/`kv`) in the worker and runs one transaction. */
async function seed(worker: Worker, origins: readonly string[]): Promise<void> {
  await worker.evaluate(async (list: readonly string[]) => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const opening = indexedDB.open("mosaic", 1);
      opening.onsuccess = () => {
        resolve(opening.result);
      };
      opening.onerror = () => {
        reject(new Error("indexeddb"));
      };
    });
    const tx = db.transaction("kv", "readwrite");
    for (const origin of list) {
      tx.objectStore("kv").put(true, `neverSave:${origin}`);
    }
    await new Promise<void>((resolve) => {
      tx.oncomplete = () => {
        resolve();
      };
    });
    db.close();
  }, origins);
}

/** The origins the background's store holds as `neverSave:<origin>`, sorted. */
async function listed(worker: Worker): Promise<string[]> {
  return worker.evaluate(async () => {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const opening = indexedDB.open("mosaic", 1);
      opening.onsuccess = () => {
        resolve(opening.result);
      };
      opening.onerror = () => {
        reject(new Error("indexeddb"));
      };
    });
    const keys = await new Promise<IDBValidKey[]>((resolve, reject) => {
      const request = db.transaction("kv", "readonly").objectStore("kv").getAllKeys();
      request.onsuccess = () => {
        resolve(request.result);
      };
      request.onerror = () => {
        reject(new Error("keys"));
      };
    });
    db.close();
    return keys
      .filter((k): k is string => typeof k === "string" && k.startsWith("neverSave:"))
      .map((k) => k.slice("neverSave:".length))
      .sort();
  });
}

async function evilExtensionId(page: Page): Promise<string> {
  const idOf = (): Promise<string> => page.evaluate(() => document.documentElement.dataset["evilExtensionId"] ?? "");
  await expect.poll(idOf).not.toBe("");
  return idOf();
}

test("another extension's page can neither read nor change the never-save list", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  const worker = await ext.worker();
  await seed(worker, LISTED);
  await open(page, `${EVIL}/login.html`);
  const evilId = await evilExtensionId(page);

  const evil = await context.newPage();
  try {
    await evil.goto(`chrome-extension://${evilId}/evil.html`);
    const attempts = await evil.evaluate(
      async ({ mosaicId, origins }) => {
        const send = (message: unknown): Promise<string> =>
          chrome.runtime.sendMessage(mosaicId, message).then(
            (reply: unknown) => `reply:${JSON.stringify(reply)}`,
            () => "refused",
          );
        const out: string[] = [await send({ op: "neverSaveList" }), await send({ id: 1, request: { op: "neverSaveList" } })];
        for (const origin of origins) {
          out.push(await send({ op: "neverSaveRemove", origin }), await send({ id: 1, request: { op: "neverSaveRemove", origin } }));
        }
        out.push(
          await new Promise<string>((resolve) => {
            const port = chrome.runtime.connect(mosaicId, { name: "popup" });
            port.onMessage.addListener((reply: unknown) => {
              resolve(`reply:${JSON.stringify(reply)}`);
            });
            port.onDisconnect.addListener(() => {
              resolve("disconnected");
            });
            port.postMessage({ id: 2, request: { op: "neverSaveList" } });
            for (const origin of origins) {
              port.postMessage({ id: 3, request: { op: "neverSaveRemove", origin } });
            }
            setTimeout(() => {
              resolve("open");
            }, 2000);
          }),
        );
        return out;
      },
      { mosaicId: ext.id, origins: [...LISTED] },
    );
    expect(attempts.slice(0, -1).every((a) => a === "refused"), JSON.stringify(attempts)).toBe(true);
    expect(attempts.at(-1)).toBe("disconnected");
  } finally {
    await evil.close();
  }
  expect(await listed(worker)).toEqual([...LISTED].sort());
});

test("a web page, its frames and another extension's content script cannot change the never-save list", async ({
  page,
  ext,
  fake,
}) => {
  await fake.reset({ items: [] });
  const worker = await ext.worker();
  await seed(worker, LISTED);
  const query = new URLSearchParams({ ext: ext.id, origins: LISTED.join(",") });
  await page.goto(`${EVIL}/never-save-spoof.html?${query.toString()}`);
  await page.waitForLoadState("load");

  // The other extension's content script tried with its own API: nothing reached Mosaic.
  const evilOutcome = (): Promise<string> => page.evaluate(() => document.documentElement.dataset["evilNeverSave"] ?? "");
  await expect.poll(evilOutcome, { timeout: 15_000 }).not.toBe("");
  const outcome = JSON.parse(await evilOutcome()) as string[];
  expect(outcome.slice(0, -1).every((a) => a === "refused"), JSON.stringify(outcome)).toBe(true);
  expect(outcome.at(-1)).toBe("disconnected");

  // The framed popup page did not load as the extension: no frame has the extension API.
  const framed = page.frames().filter((f) => f !== page.mainFrame());
  expect(framed).toHaveLength(2);
  for (const frame of framed) {
    const api = await frame
      .evaluate(() => typeof (globalThis as { chrome?: { runtime?: { connect?: unknown } } }).chrome?.runtime?.connect)
      .catch(() => "unreachable");
    expect(api, frame.url()).not.toBe("function");
  }

  // The passkey bridge answered none of the page's messages (it acknowledges only the shim's shapes).
  await page.waitForTimeout(1000);
  const replies = await page.evaluate(() => (window as unknown as { __neverSaveReplies: unknown[] }).__neverSaveReplies);
  const bridgeTypes = replies
    .map((r) => (typeof r === "object" && r !== null ? (r as { type?: unknown }).type : undefined))
    .filter((t) => t === "mosaic-passkey-ack" || t === "mosaic-passkey-res");
  expect(bridgeTypes).toEqual([]);

  expect(await listed(worker)).toEqual([...LISTED].sort());
});
