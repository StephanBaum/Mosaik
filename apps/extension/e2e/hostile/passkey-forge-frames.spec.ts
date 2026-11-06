/**
 * Only the window the bridge serves may post requests to it (`event.source === window`). Frames
 * of another site, sandboxed frames with an opaque origin and a popup's opener hold a reference to
 * that window and can post into it, but the browser names them as the source, so the bridge drops
 * their requests and the desktop never hears of them.
 */
import { BANK, EVIL, expect, open, test } from "../fixtures.ts";

interface Seen {
  seen: unknown[];
}

test("frames of another site and sandboxed frames cannot post requests into the top window's bridge", async ({ page, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, `${BANK}/passkey/forge-frames.html`, 3);
  // Both frames have posted into parent and top, directly and with a MessagePort (reaching into the top's realm threw).
  await expect
    .poll(() => Promise.all(page.frames().slice(1).map((frame) => frame.title())))
    .toEqual(["forged 4", "forged 4"]);
  await page.waitForTimeout(600);
  expect(await page.evaluate(() => (window as unknown as Seen).seen)).toEqual([]);
  expect(await fake.passkeyLog()).toEqual([]);
});

test("a popup's opener cannot post requests into the popup's bridge", async ({ page, context, fake }) => {
  await fake.reset();
  await fake.setPasskeys(true);
  await open(page, `${EVIL}/passkey/forge-popup.html`);
  const [popup] = await Promise.all([
    context.waitForEvent("page"),
    page.evaluate(() => {
      (window as unknown as { openBank(): void }).openBank();
    }),
  ]);
  await popup.waitForLoadState("load");
  await popup.evaluate(() => {
    const seen: unknown[] = [];
    (window as unknown as Seen).seen = seen;
    window.addEventListener("message", (event) => {
      const type = (event.data as { type?: unknown } | null)?.type;
      if (type === "mosaic-passkey-ack" || type === "mosaic-passkey-res") {
        seen.push(event.data);
      }
    });
  });
  await popup.bringToFront();
  await page.evaluate(() => {
    (window as unknown as { forge(): void }).forge();
  });
  await popup.waitForTimeout(600);
  expect(await popup.evaluate(() => (window as unknown as Seen).seen)).toEqual([]);
  expect(await fake.passkeyLog()).toEqual([]);
  await popup.close();
});
