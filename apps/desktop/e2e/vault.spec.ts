import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`vault, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("unlock, open an item, reveal a secret, generator and import", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=locked");
      await page.getByLabel("Master password", { exact: true }).fill("mosaic");
      await page.getByRole("button", { name: "Unlock" }).click();
      await expect(page.getByText("GitHub")).toBeVisible();
      await expectAccessible(page);

      await page.getByRole("button", { name: /GitHub/ }).click();
      await expect(page.getByRole("article", { name: "GitHub" })).toBeVisible();
      await page.getByRole("button", { name: "Reveal Password" }).click();
      await expect(page.getByText("correct-horse-battery-staple")).toBeVisible();
      await expectAccessible(page);

      await page.getByRole("button", { name: "Generator" }).click();
      await expect(page.getByRole("heading", { name: "Password generator" })).toBeVisible();
      await expectAccessible(page);

      await page.keyboard.press("Control+k");
      await expect(page.getByRole("dialog", { name: "Command palette" })).toBeVisible();
      await expectAccessible(page);
      // The empty state: the combobox must not point at a list that is not there.
      await page.getByRole("combobox", { name: "Search items and commands" }).fill("zzzz");
      await expect(page.getByText("Nothing matches. Try another word.")).toBeVisible();
      await expectAccessible(page);
      // One Esc closes the palette, whatever the query.
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog", { name: "Command palette" })).toBeHidden();

      // Ctrl+N opens the editor; the screen has exactly one holo button.
      await page.keyboard.press("Control+n");
      await expect(page.getByRole("heading", { name: /^New / })).toBeVisible();
      await expect(page.locator(".holo")).toHaveCount(1);
      await expectAccessible(page);
      // Esc in the (clean) form cancels it.
      await page.getByLabel("Title").click();
      await page.keyboard.press("Escape");
      await expect(page.getByRole("heading", { name: /^New / })).toBeHidden();

      await page.keyboard.press("Control+k");
      await page.getByRole("option", { name: "Import from another app" }).click();
      await expect(page.getByRole("heading", { name: "Import" })).toBeVisible();
      await page.getByRole("button", { name: "Choose the export file" }).click();
      await page.getByRole("button", { name: "Preview" }).click();
      await expect(page.getByRole("checkbox", { name: /GitHub/ })).not.toBeChecked();
      await expect(page.locator(".holo")).toHaveCount(1);
      await expectAccessible(page);

      // Ctrl+Alt+L locks.
      await page.keyboard.press("Control+Alt+l");
      await expect(page.getByRole("heading", { name: "Unlock Mosaic" })).toBeVisible();
      guard.expectClean();
    });
  });
}
