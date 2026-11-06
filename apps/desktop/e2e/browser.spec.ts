import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`Browser extension prompts, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("the pairing dialog shows the code, focuses Deny and lets Allow wait a second", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserPair=1");
      const deny = page.getByRole("button", { name: "Deny" });
      await expect(deny).toBeVisible({ timeout: 5000 });
      await expect(deny).toBeFocused();
      await expect(page.getByText("4F7-K2Q")).toBeVisible();
      await expect(page.getByRole("button", { name: "Allow", exact: true })).toBeDisabled();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Allow", exact: true })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Allow", exact: true }).click();
      await expect(deny).toBeHidden();
      guard.expectClean();
    });

    test("the save dialog for a new login on an unencrypted page", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserSave=new");
      const cancel = page.getByRole("button", { name: "Cancel" });
      await expect(cancel).toBeVisible({ timeout: 5000 });
      await expect(cancel).toBeFocused();
      await expect(page.getByText("••••••••")).toBeVisible();
      await expect(page.getByRole("switch", { name: "Allow filling on this unencrypted page" })).toBeVisible();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Save", exact: true })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Save", exact: true }).click();
      await expect(cancel).toBeHidden();
      guard.expectClean();
    });

    test("the save dialog for an update", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&browserSave=update");
      await expect(page.getByRole("combobox", { name: "Save as" })).toBeVisible({ timeout: 5000 });
      await expectAccessible(page);
      await page.getByRole("button", { name: "Cancel" }).click();
      await expect(page.getByRole("combobox", { name: "Save as" })).toBeHidden();
      guard.expectClean();
    });

    test("Settings lists the paired browsers", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked");
      await page.getByRole("button", { name: "Settings" }).click();
      await expect(page.getByRole("heading", { name: "Browsers" })).toBeVisible();
      await expect(page.getByRole("button", { name: "Disconnect Chrome" })).toBeVisible();
      await expectAccessible(page);
      await page.getByRole("button", { name: "Disconnect Chrome" }).click();
      await expect(page.getByText("Disconnect “Chrome”?")).toBeVisible();
      await expectAccessible(page);
      guard.expectClean();
    });
  });
}
