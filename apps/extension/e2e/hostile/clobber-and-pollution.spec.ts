import type { Page } from "@playwright/test";
import { BANK, PASSWORD, USERNAME, bankItem, expect, inspect, open, test } from "../fixtures.ts";

async function filledOnly(page: Page): Promise<[string, string][]> {
  const [frame] = await inspect(page);
  return Object.entries(frame?.values ?? {}).filter(([, value]) => value !== "");
}

test("DOM clobbering of the content script's globals neither stops it nor widens what it fills", async ({
  page,
  fake,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/clobber-globals.html`);
  // The clobbering is real in the page's world.
  expect(await page.evaluate(() => (window as unknown as { __clobbered: unknown }).__clobbered)).toEqual({
    mosaicContent: "[object HTMLImageElement]",
    documentElement: "[object HTMLImageElement]",
  });
  const tabId = await fake.tabIdOf(BANK);

  // Twice: the first gesture injects the script, the second finds it installed.
  for (let round = 0; round < 2; round++) {
    expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({
      ok: true,
      summary: { username: 1, password: 1 },
    });
  }
  expect(await filledOnly(page)).toEqual([
    ["user", USERNAME],
    ["pass", PASSWORD],
  ]);
});

test("prototype pollution in the page's world does not reach the content script", async ({ page, fake }) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/prototype-pollution.html`);
  const tabId = await fake.tabIdOf(BANK);

  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({
    ok: true,
    summary: { username: 1, password: 1 },
  });
  expect(await filledOnly(page)).toEqual([
    ["user", USERNAME],
    ["pass", PASSWORD],
  ]);
  // None of the page's replaced functions ran for the extension.
  expect(await page.evaluate(() => (window as unknown as { __pollution: unknown[] }).__pollution)).toEqual([]);
});
