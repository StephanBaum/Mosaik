/**
 * Forged change messages from another installed extension (M7b): `rotationResult` ("It worked"),
 * `rotationAddOrigin`, `rotationBegin` and `fillChange` sent to Mosaic over `runtime.sendMessage`
 * and over a port named `popup`, and a content-script-shaped `fillChange` with its own values
 * sent to the tab. Defense: only Mosaic's own popup port carries rotation ops (the worker has no
 * external listeners), and Mosaic's content script takes messages from Mosaic's background only.
 */
import { BANK, PASSWORD, expect, inspect, open, test } from "../fixtures.ts";
import { IDS, NEW_VALUE, rotationConfig, serveRotationPages } from "./rotation-site.ts";

test.use({ evilExtension: true });

test("another extension cannot answer, start, add to or fill a password change", async ({ page, context, fake, ext }) => {
  await fake.reset(rotationConfig());
  await serveRotationPages(page);
  await open(page, `${BANK}/rotation-pages/change.html`);
  const tabId = await fake.tabIdOf(BANK);
  const idOf = (): Promise<string> => page.evaluate(() => document.documentElement.dataset["evilExtensionId"] ?? "");
  await expect.poll(idOf).not.toBe("");
  const evilId = await idOf();

  // Mosaic's content script is in the page (a real change fill), so a forged fill has a target.
  expect(await fake.popupRequest(tabId, { op: "fillChange", ...IDS, confirmFrame: null })).toMatchObject({ newPassword: 2 });
  await page.evaluate(() => {
    for (const el of document.querySelectorAll("input")) {
      el.value = "";
    }
  });
  const before = (await fake.log()).length;

  const evil = await context.newPage();
  try {
    await evil.goto(`chrome-extension://${evilId}/evil.html`);
    const results = await evil.evaluate(
      async ({ mosaicId, tab, origin, ids }) => {
        const requests = [
          { op: "rotationResult", ...ids, outcome: "succeeded" },
          { op: "rotationAddOrigin" },
          { op: "rotationBegin", vaultId: ids.vaultId, itemId: ids.itemId },
          { op: "fillChange", ...ids, confirmFrame: null },
          { op: "fillPending", ...ids, confirmFrame: null },
        ];
        const out: string[] = [];
        for (const [index, request] of requests.entries()) {
          try {
            out.push(`message:${JSON.stringify(await chrome.runtime.sendMessage(mosaicId, { id: index, request }))}`);
          } catch {
            out.push("message:refused");
          }
          out.push(
            await new Promise<string>((resolve) => {
              const port = chrome.runtime.connect(mosaicId, { name: "popup" });
              port.onMessage.addListener((reply) => {
                resolve(`port:${JSON.stringify(reply)}`);
              });
              port.onDisconnect.addListener(() => {
                resolve("port:disconnected");
              });
              port.postMessage({ id: index, request });
              setTimeout(() => {
                resolve("port:open");
              }, 1500);
            }),
          );
        }
        // The exact message Mosaic's content script takes for a change fill, with the attacker's values.
        const fill = { type: "fillChange", fillId: "x", expectedOrigin: origin, shape: "currentAndNew", current: "evil-current", new: "evil-new" };
        await chrome.tabs.sendMessage(tab, fill).catch(() => undefined);
        return out;
      },
      { mosaicId: ext.id, tab: tabId, origin: BANK, ids: IDS },
    );
    expect(new Set(results)).toEqual(new Set(["message:refused", "port:disconnected"]));
  } finally {
    await evil.close();
  }

  // Nothing reached the desktop, the change is still open, and the page got none of the values.
  expect((await fake.log()).slice(before)).toEqual([]);
  expect((await fake.popupState(tabId)).rotations).toHaveLength(1);
  const [frame] = await inspect(page);
  const seen = [...Object.values(frame?.values ?? {}), ...(frame?.leaks ?? []).slice(-20).map((leak) => leak.value)];
  expect(seen).not.toContain("evil-current");
  expect(seen).not.toContain("evil-new");
  expect(Object.values(frame?.values ?? {})).not.toContain(NEW_VALUE);
  expect(Object.values(frame?.values ?? {})).not.toContain(PASSWORD);
});
