/** Demos play while visible, pause when off screen and stop after three loops on their final frame. */
import { expect, test, type Page } from "@playwright/test";

const DEMOS: readonly (readonly [string, string])[] = [
  ["/", "fill"],
  ["/", "agent"],
  ["/", "import"],
  ["/", "phone"],
  ["/features/", "cli"],
  ["/security/", "sync"],
  ["/security/", "cipher"],
  ["/download/next/", "nextSteps"],
];

const html = (page: Page, sel: string): Promise<string> => page.locator(sel).evaluate((e) => e.outerHTML);

for (const [path, name] of DEMOS) {
  test(`${path} ${name}: plays while visible, rests while off screen`, async ({ page }) => {
    const sel = `[data-island="demos"][data-demo="${name}"]`;
    await page.goto(path);
    await page.locator(sel).scrollIntoViewIfNeeded();
    const start = await html(page, sel);
    await expect.poll(() => html(page, sel), { timeout: 8000 }).not.toBe(start);
    // Off screen: it pauses, so two snapshots a moment apart are equal.
    await page.evaluate(() => {
      window.scrollTo(0, document.documentElement.scrollHeight);
    });
    const farAway = await page.locator(sel).evaluate((e) => Math.abs(e.getBoundingClientRect().top) > window.innerHeight + 400);
    if (farAway) {
      await page.waitForTimeout(400);
      const a = await html(page, sel);
      await page.waitForTimeout(700);
      expect(await html(page, sel)).toBe(a);
    }
  });
}

test("the sync demo loops three times, then rests on its final frame", async ({ page }) => {
  test.setTimeout(300_000);
  await page.goto("/security/");
  const warn = page.locator('[data-demo="sync"] .js-warn');
  await warn.scrollIntoViewIfNeeded();
  let high = false;
  let falls = 0;
  // The markup starts on its final frame; the demo's first frame (opacity 0) is where counting begins.
  let started = false;
  const t0 = Date.now();
  // Each loop restarts the warning at opacity 0, so the falls count the restarts: loops 2 and 3 make two.
  while (Date.now() - t0 < 200_000 && !(falls === 2 && high)) {
    const o = Number(await warn.evaluate((e) => getComputedStyle(e).opacity));
    if (!started) {
      started = o < 0.1;
    } else if (o > 0.9) {
      high = true;
    } else if (o < 0.1 && high) {
      falls += 1;
      high = false;
    }
    // Keep the demo on screen: it pauses while it is not visible.
    await warn.evaluate((e) => {
      e.scrollIntoView({ block: "center" });
    });
    await page.waitForTimeout(250);
  }
  expect(falls).toBe(2);
  expect(high).toBe(true);
  // The third loop is under way. For longer than a whole loop nothing restarts it again.
  let restarts = 0;
  let up = true;
  const t1 = Date.now();
  while (Date.now() - t1 < 40_000) {
    await warn.evaluate((e) => {
      e.scrollIntoView({ block: "center" });
    });
    const o = Number(await warn.evaluate((e) => getComputedStyle(e).opacity));
    if (o < 0.1 && up) {
      restarts += 1;
      up = false;
    } else if (o > 0.9) {
      up = true;
    }
    await page.waitForTimeout(250);
  }
  expect(restarts).toBe(0);
});
