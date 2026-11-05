import { BANK, PASSWORD, USERNAME, bankItem, expect, expectNothingFilled, inspect, open, test } from "../fixtures.ts";

test("a page cannot reach the extension: no extension API, no pages, no accepted messages", async ({
  page,
  fake,
  ext,
}) => {
  await fake.reset({ items: [bankItem()] });
  await open(page, `${BANK}/spoof-messages.html`);

  // The page sees no extension API (externally_connectable names no web origin).
  expect(["no chrome", "undefined"]).toContain(await page.evaluate(() => (window as unknown as { __spoofApi: string }).__spoofApi));
  expect(await page.evaluate(() => typeof (globalThis as { chrome?: { runtime?: { connect?: unknown } } }).chrome?.runtime?.connect)).toBe("undefined");

  // Nothing of the extension is reachable from the page (no web_accessible_resources).
  const reachable = await page.evaluate(async (id) => {
    const results: string[] = [];
    for (const path of ["popup.html", "manifest.json", "content.js", "background.js"]) {
      try {
        const response = await fetch(`chrome-extension://${id}/${path}`);
        results.push(`${path}:${String(response.status)}`);
      } catch {
        results.push(`${path}:blocked`);
      }
    }
    return results;
  }, ext.id);
  expect(reachable).toEqual(["popup.html:blocked", "manifest.json:blocked", "content.js:blocked", "background.js:blocked"]);

  // The page's postMessage spam (fill-shaped, capture-shaped, popup-shaped) changes nothing.
  await page.waitForTimeout(1000);
  await expectNothingFilled(page);
  expect(await fake.log()).toEqual([]);

  // With the content script injected, the spam goes on; the real credential still wins and the
  // page's own values never arrive.
  const tabId = await fake.tabIdOf(BANK);
  expect(await fake.gesture({ kind: "shortcut", tabId })).toMatchObject({ ok: true });
  await page.waitForTimeout(1000);
  const [frame] = await inspect(page);
  expect(frame?.values).toEqual({ user: USERNAME, pass: PASSWORD });
  expect(frame?.leaks.map((leak) => leak.value)).not.toContain("evil");
  expect(await fake.fills()).toHaveLength(1);
});
