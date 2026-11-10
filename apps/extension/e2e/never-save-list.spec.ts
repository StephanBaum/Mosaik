/**
 * "Never offered for saving" (M4-R10): the popup lists the `neverSave:<origin>` entries and
 * removes one on request. Only the popup's port can change the list: a message from a content
 * script (a sender with a tab) or from a page is dropped, and the entry stays.
 */
import type { Page, Worker } from "@playwright/test";
import { expectAccessible } from "./axe.ts";
import { BANK, EVIL, expect, open, openPopup, test } from "./fixtures.ts";

const LISTED = ["https://bank.test:4843", "https://shop.test"] as const;

/** Writes the entries as the background's own store does (IndexedDB `mosaic`/`kv`, key `neverSave:<origin>`). */
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
  }, origins);
}

async function openList(popup: Page): Promise<void> {
  await popup.getByRole("button", { name: "More options" }).click();
  await popup.getByRole("menuitem", { name: "Never offered for saving…" }).click();
}

test("lists the never-saved sites and removes one", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await seed(await ext.worker(), LISTED);
  await open(page, `${BANK}/login.html`);
  const popup = await openPopup(context, ext.id, await fake.tabIdOf(BANK));
  await openList(popup);
  const dialog = popup.getByRole("dialog", { name: "Never offered for saving" });
  for (const origin of LISTED) {
    await expect(dialog.getByText(origin, { exact: true })).toBeVisible();
  }
  await expectAccessible(popup);

  await dialog.getByRole("button", { name: `Remove ${LISTED[0]} from the list` }).click();
  await expect(dialog.getByRole("status")).toContainText("removed");
  await expect(dialog.getByText(LISTED[0], { exact: true })).toHaveCount(0);
  await expect(dialog.getByText(LISTED[1], { exact: true })).toBeVisible();

  // The list is the background's: a fresh popup shows the same.
  await popup.close();
  const again = await openPopup(context, ext.id, await fake.tabIdOf(BANK));
  await openList(again);
  const second = again.getByRole("dialog", { name: "Never offered for saving" });
  await expect(second.getByText(LISTED[1], { exact: true })).toBeVisible();
  await expect(second.getByText(LISTED[0], { exact: true })).toHaveCount(0);
  await again.close();
});

test("a content script, a page and the popup's other senders cannot change the list", async ({ page, context, ext, fake }) => {
  await fake.reset({ items: [] });
  await seed(await ext.worker(), LISTED);
  await open(page, `${EVIL}/login.html`);
  const tabId = await fake.tabIdOf(EVIL);

  // An extension page that has a tab is what a content script looks like to the router
  // (a sender with a tab and no popup port): the runtime message is dropped.
  const sender = await openPopup(context, ext.id, tabId);
  await sender.evaluate(async (origins: readonly string[]) => {
    for (const origin of origins) {
      void chrome.runtime.sendMessage({ op: "neverSaveRemove", origin });
      void chrome.runtime.sendMessage({ id: 1, request: { op: "neverSaveRemove", origin } });
    }
    await new Promise((resolve) => setTimeout(resolve, 300));
  }, LISTED);
  await sender.close();

  // A web page has no extension API at all.
  expect(await page.evaluate(() => typeof (globalThis as { chrome?: { runtime?: { sendMessage?: unknown } } }).chrome?.runtime?.sendMessage)).toBe("undefined");

  const popup = await openPopup(context, ext.id, tabId);
  await openList(popup);
  const dialog = popup.getByRole("dialog", { name: "Never offered for saving" });
  for (const origin of LISTED) {
    await expect(dialog.getByText(origin, { exact: true })).toBeVisible();
  }
  await popup.close();
});
