import { BANK, PASSWORD, USERNAME, bankItem, expect, inspect, open, test } from "../fixtures.ts";

test.use({ evilExtension: true });

interface Attempts {
  message: string;
  port: string;
  popupPort: string;
  seesMosaic: unknown;
}

test("another installed extension cannot message, connect to or fill through Mosaic", async ({
  page,
  context,
  fake,
  ext,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/login.html`);
  const tabId = await fake.tabIdOf(BANK);
  const idOf = (): Promise<string> => page.evaluate(() => document.documentElement.dataset["evilExtensionId"] ?? "");
  await expect.poll(idOf).not.toBe("");
  const evilId = await idOf();
  expect(evilId).not.toBe(ext.id);

  // Mosaic fills once, so its content script is in the page while the other extension tries.
  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });

  const evil = await context.newPage();
  try {
    await evil.goto(`chrome-extension://${evilId}/evil.html`);
    const attempts = await evil.evaluate(
      async ({ mosaicId, tab, origin }): Promise<Attempts> => {
        let message: string;
        try {
          const request = { op: "fill", vaultId: "v1", itemId: "i1", confirmFrame: true };
          message = `reply:${JSON.stringify(await chrome.runtime.sendMessage(mosaicId, { id: 1, request }))}`;
        } catch {
          message = "refused";
        }
        const tryPort = (name: string): Promise<string> =>
          new Promise((resolve) => {
            const port = chrome.runtime.connect(mosaicId, { name });
            port.onMessage.addListener((reply) => {
              resolve(`reply:${JSON.stringify(reply)}`);
            });
            port.onDisconnect.addListener(() => {
              resolve("disconnected");
            });
            port.postMessage({ id: 1, request: { op: "getState" } });
            setTimeout(() => {
              resolve("open");
            }, 2000);
          });
        const port = await tryPort("evil");
        const popupPort = await tryPort("popup");
        // The exact message Mosaic's content script accepts, sent to the tab by another extension.
        const fill = { type: "fill", fillId: "x", expectedOrigin: origin, username: "evil", password: "evil", otp: null };
        await chrome.tabs.sendMessage(tab, fill).catch(() => undefined);
        const seesMosaic: unknown = await chrome.tabs.sendMessage(tab, { check: true }).catch(() => "no reply");
        return { message, port, popupPort, seesMosaic };
      },
      { mosaicId: ext.id, tab: tabId, origin: BANK },
    );
    expect(attempts).toEqual({ message: "refused", port: "disconnected", popupPort: "disconnected", seesMosaic: "undefined" });
  } finally {
    await evil.close();
  }

  // The page holds the real credential, never the other extension's values, and the desktop
  // was asked for nothing more.
  const [frame] = await inspect(page);
  expect(frame?.values).toMatchObject({ user: USERNAME, pass: PASSWORD });
  expect(frame?.leaks.map((leak) => leak.value)).not.toContain("evil");
  expect(await fake.fills()).toHaveLength(1);
});
