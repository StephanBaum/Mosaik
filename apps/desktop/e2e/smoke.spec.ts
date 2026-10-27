/**
 * Smoke test: the mock-mode production build under the production CSP. Every screen is
 * checked with axe (WCAG 2.2 AA tags) in light and dark, and any CSP violation fails.
 */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

async function expectAccessible(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
  expect(results.violations.map((v) => `${v.id}: ${v.help}`)).toEqual([]);
}

function watchCsp(page: Page): string[] {
  const violations: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error" && /Content Security Policy/i.test(message.text())) {
      violations.push(message.text());
    }
  });
  return violations;
}

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`${colorScheme} theme`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("onboarding, unlock and vault render accessibly under the CSP", async ({ page }) => {
      const csp = watchCsp(page);

      await page.goto("/");
      await expect(page.getByRole("heading", { name: "Create your account" })).toBeVisible();
      await expectAccessible(page);

      await page.goto("/?mock=locked");
      await expect(page.getByRole("heading", { name: "Unlock Mosaic" })).toBeVisible();
      await expectAccessible(page);

      await page.getByLabel("Master password").fill("mosaic");
      await page.getByRole("button", { name: "Unlock" }).click();
      await expect(page.getByRole("navigation", { name: "Vaults" })).toBeVisible();
      await expect(page.getByText("GitHub")).toBeVisible();
      await expectAccessible(page);

      expect(csp).toEqual([]);
    });
  });
}

test("fonts are bundled, never fetched from elsewhere", async ({ page }) => {
  const external: string[] = [];
  page.on("request", (request) => {
    if (!request.url().startsWith("http://localhost:1421/")) {
      external.push(request.url());
    }
  });
  await page.goto("/?mock=locked");
  await expect(page.getByRole("heading", { name: "Unlock Mosaic" })).toBeVisible();
  await page.evaluate(() => document.fonts.ready);
  expect(await page.evaluate(() => document.fonts.check('16px "DM Sans"'))).toBe(true);
  expect(external).toEqual([]);
});
