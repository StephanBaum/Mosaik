import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`settings, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("turning auto-lock off needs the master password", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked");
      await page.getByRole("button", { name: "Settings" }).click();
      await expect(page.getByRole("heading", { name: "Settings" })).toBeVisible();
      await expectAccessible(page);

      await page.getByLabel("Lock after inactivity").selectOption("never");
      await expect(page.getByRole("dialog", { name: "Confirm with your master password" })).toBeVisible();
      await expectAccessible(page);
      await page.getByLabel("Master password", { exact: true }).fill("mosaic");
      await page.getByRole("button", { name: "Confirm" }).click();
      await expect(page.getByText("Settings saved.")).toBeVisible();
      guard.expectClean();
    });

    test("the shortcut field and its buttons stay on screen at 1280 px", async ({ page }) => {
      // M9-UX1: the form once squeezed the field to 17 px and pushed Save off the page.
      const guard = watch(page);
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.goto("/?mock=unlocked");
      await page.getByRole("button", { name: "Settings" }).click();
      const field = page.getByLabel("Shortcut", { exact: true });
      await field.scrollIntoViewIfNeeded();
      const box = await field.boundingBox();
      expect(box?.width ?? 0).toBeGreaterThan(160);
      for (const name of ["Save shortcut", "Use the default"]) {
        const button = await page.getByRole("button", { name }).boundingBox();
        expect(button).not.toBeNull();
        expect((button?.x ?? 0) + (button?.width ?? 0)).toBeLessThanOrEqual(1280);
        expect(button?.x ?? -1).toBeGreaterThanOrEqual(0);
      }
      guard.expectClean();
    });
  });
}
