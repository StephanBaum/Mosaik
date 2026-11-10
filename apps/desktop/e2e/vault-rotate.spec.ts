import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

// "Rotate vault key…" (M7-R2) on the mock: the vault menu, the dialog with the attachments option,
// the master password after the click, the progress as a live region, and the banner that
// finishes a re-seal that was cut off. The mock's steps are paced by `rotationStepMs`.
for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`rotate vault key, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("rotates a vault key and its attachments", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&sync=hostedNoTotp&rotation=none&rotationStepMs=400");
      await expect(page.getByText("GitHub")).toBeVisible();
      await page.getByRole("button", { name: /^Personal/ }).click();

      await page.getByRole("button", { name: "Vault actions" }).click();
      await page.getByRole("menuitem", { name: "Rotate vault key…" }).click();
      const dialog = page.getByRole("dialog", { name: /Rotate the key of/ });
      await expect(dialog).toBeVisible();
      // Cancel has focus first, so Enter never rotates by accident.
      await expect(dialog.getByRole("button", { name: "Cancel" })).toBeFocused();
      await expectAccessible(page);

      await dialog.getByRole("switch", { name: "Also re-encrypt attachments" }).check();
      await dialog.getByRole("button", { name: "Rotate key" }).click();

      // The password comes after the click, not before it.
      const reauth = page.getByRole("dialog", { name: "Confirm with your master password" });
      await expect(reauth.getByText("Rotating a vault key needs your master password.")).toBeVisible();
      await expectAccessible(page);
      await reauth.getByLabel("Master password").fill("mosaic");
      await reauth.getByRole("button", { name: "Confirm" }).click();

      // The progress is said from one live region, then the result.
      const status = dialog.getByRole("status");
      await expect(status).toContainText(/Re-encrypting attachments: \d+ of 8\./);
      await expect(dialog.getByText("The vault key was rotated and the attachments are encrypted again.")).toBeVisible({
        timeout: 15_000,
      });
      await expectAccessible(page);
      await dialog.getByRole("button", { name: "Close" }).click();
      await expect(dialog).toHaveCount(0);
      guard.expectClean();
    });

    test("a wrong password keeps the dialog open and says so", async ({ page }) => {
      await page.goto("/?mock=unlocked&sync=hostedNoTotp&rotation=none&rotationStepMs=0");
      await expect(page.getByText("GitHub")).toBeVisible();
      await page.getByRole("button", { name: /^Personal/ }).click();
      await page.getByRole("button", { name: "Vault actions" }).click();
      await page.getByRole("menuitem", { name: "Rotate vault key…" }).click();
      const dialog = page.getByRole("dialog", { name: /Rotate the key of/ });
      await dialog.getByRole("button", { name: "Rotate key" }).click();
      const reauth = page.getByRole("dialog", { name: "Confirm with your master password" });
      await reauth.getByLabel("Master password").fill("not the password");
      await reauth.getByRole("button", { name: "Confirm" }).click();
      await expect(reauth.getByRole("alert")).not.toBeEmpty();
      await expect(reauth).toBeVisible();
    });

    test("the banner finishes a re-seal that was cut off", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&rotation=resumeVault&rotationStepMs=100");
      await expect(page.getByText("GitHub")).toBeVisible();
      const banner = page.getByRole("region", { name: "Key change not finished" });
      await expect(banner).toBeVisible();
      await expect(banner.getByText(/attachments of a vault was cut off/)).toBeVisible();
      await expectAccessible(page);
      await banner.getByRole("button", { name: "Continue key rotation" }).click();
      await expect(banner).toHaveCount(0, { timeout: 15_000 });
      guard.expectClean();
    });
  });
}
