/** Page transitions: the veil wipe, keyboard and modifier clicks, bfcache, the download jump and prefetch. */
import { expect, test, type Page } from "@playwright/test";
import { forceField, REAL_GPU } from "./fixtures.ts";

/** Remembers across navigations whether a `.veil.is-on` ever appeared. */
async function watchVeil(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const check = (): void => {
      if (document.querySelector(".veil.is-on") !== null) {
        try {
          sessionStorage.setItem("veil-seen", "1");
        } catch {
          // storage blocked: the test fails visibly
        }
      }
    };
    new MutationObserver(check).observe(document, { subtree: true, attributes: true, attributeFilter: ["class"], childList: true });
  });
}

const veilSeen = (page: Page): Promise<boolean> => page.evaluate(() => sessionStorage.getItem("veil-seen") === "1");

async function ready(page: Page): Promise<void> {
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-fx-state", "full");
  await page.waitForSelector("canvas.veil", { state: "attached", timeout: 15_000 });
  await page.waitForTimeout(1500);
}

const FEATURES_LINK = 'header a[data-nav][href="/features/"]';

// Software WebGL needs seconds for a frame of the veil; a GPU needs its 320 ms.
const LAND_MS = REAL_GPU ? 1500 : 20_000;

test.describe("forced field, desktop", () => {
  test.use({ viewport: { width: 1000, height: 640 } });
  test.beforeEach(async ({ page }) => {
    await forceField(page);
    await watchVeil(page);
  });

  test("a pointer click on a nav link wipes with the veil, then lands within 1.5 s", async ({ page }) => {
    await ready(page);
    const t0 = Date.now();
    await page.locator(FEATURES_LINK).first().click();
    await page.waitForURL("**/features/", { timeout: LAND_MS });
    expect(Date.now() - t0).toBeLessThan(LAND_MS + 200);
    expect(await veilSeen(page)).toBe(true);
  });

  test("Enter on a focused nav link navigates without the veil", async ({ page }) => {
    await ready(page);
    await page.locator(FEATURES_LINK).first().focus();
    await page.keyboard.press("Enter");
    // Native navigation (no wipe); software WebGL keeps the main thread busy, so allow LAND_MS.
    await page.waitForURL("**/features/", { timeout: LAND_MS });
    expect(await veilSeen(page)).toBe(false);
  });

  test("Ctrl+click opens a new page and leaves this one", async ({ page, context }) => {
    await ready(page);
    const popup = context.waitForEvent("page");
    await page.locator(FEATURES_LINK).first().click({ modifiers: ["Control"] });
    const next = await popup;
    expect(next.url()).toContain("/features/");
    expect(new URL(page.url()).pathname).toBe("/");
    expect(await veilSeen(page)).toBe(false);
  });

  test("back after a wipe never leaves the veil up", async ({ page }) => {
    await ready(page);
    await page.locator(FEATURES_LINK).first().click();
    await page.waitForURL("**/features/");
    await page.goBack();
    await page.waitForURL((u) => u.pathname === "/");
    await page.waitForTimeout(500);
    await expect(page.locator(".veil.is-on")).toHaveCount(0);
  });
});

test.describe("download jump", () => {
  // No detected system, so the hero button stays the in-page jump instead of a file link.
  test.use({ userAgent: "Mozilla/5.0 (PlayStation; PlayStation 5/2.26) AppleWebKit/605.1.15" });

  test("the hero button glides down and a wheel stops it", async ({ page }) => {
    await page.goto("/download/");
    await expect(page.locator("a[data-jump]").first()).toBeVisible();
    await page.waitForTimeout(1500);
    const target = await page.evaluate(() => (document.getElementById("platforms")?.getBoundingClientRect().top ?? 0) + window.scrollY);
    await page.locator("a[data-jump]").first().click();
    // Software WebGL in the holo button delays the first frame of the glide by seconds.
    await expect.poll(() => page.evaluate(() => window.scrollY), { timeout: 10_000, intervals: [50] }).toBeGreaterThan(40);
    await page.mouse.wheel(0, 1);
    await page.waitForTimeout(300);
    const y1 = await page.evaluate(() => window.scrollY);
    await page.waitForTimeout(900);
    const y2 = await page.evaluate(() => window.scrollY);
    expect(Math.abs(y2 - y1)).toBeLessThanOrEqual(2);
    expect(y2).toBeLessThan(target - 5);
  });
});

test("hovering a nav link prefetches its page", async ({ page }) => {
  await page.goto("/");
  await page.waitForTimeout(1000);
  const hit = page.waitForRequest((r) => r.url().endsWith("/features/") && r.headers()["sec-purpose"]?.includes("prefetch") === true, { timeout: 8000 });
  const link = page.locator(FEATURES_LINK).first();
  await link.hover();
  await page.mouse.move(5, 5);
  await link.hover();
  await hit;
});
