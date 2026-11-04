import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`otp setup, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("add a one-time code by scanning and by key", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked");
      await expect(page.getByText("GitHub")).toBeVisible();
      await page.keyboard.press("Control+n");
      await page.getByLabel("Title").fill("Example");
      await page.getByRole("button", { name: "Scan screen" }).click();
      await expect(page.getByRole("heading", { name: "Is this the right code?" })).toBeVisible();
      await expect(page.getByText("492 039")).toBeVisible();
      await expectAccessible(page);
      await page.getByRole("button", { name: "Use this code" }).click();
      await expect(page.getByText("Saved together with this item.")).toBeVisible();
      await expectAccessible(page);
      await page.getByRole("button", { name: "Save" }).click();
      // The detail pane shows the one-time code row of the new item.
      await expect(page.getByRole("article", { name: "Example" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Show code" })).toBeVisible();

      // The second path: "Can't scan?" with a typed key.
      await page.keyboard.press("Control+n");
      await page.getByLabel("Title").fill("Example 2");
      await page.getByRole("button", { name: "Can't scan? Enter the key" }).click();
      await expect(page.getByRole("heading", { name: "Enter the setup key" })).toBeVisible();
      await expectAccessible(page);
      await page.getByLabel("Setup key or otpauth:// link").fill("JBSW Y3DP EHPK 3PXP");
      await page.getByRole("button", { name: "Continue" }).click();
      await expect(page.getByRole("heading", { name: "Is this the right code?" })).toBeVisible();
      await expect(page.getByLabel("Service")).toHaveValue("Example 2");
      await page.getByRole("button", { name: "Use this code" }).click();
      await page.getByRole("button", { name: "Save" }).click();
      await expect(page.getByRole("article", { name: "Example 2" })).toBeVisible();
      guard.expectClean();
    });
  });
}
