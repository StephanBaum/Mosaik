import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`SSH approval, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("Deny has focus and Allow waits one second", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked&sshRequest=1");
      const deny = page.getByRole("button", { name: "Deny" });
      await expect(deny).toBeVisible({ timeout: 5000 });
      await expect(deny).toBeFocused();
      await expect(page.getByRole("button", { name: "Allow once" })).toBeDisabled();
      await expectAccessible(page);
      await expect(page.getByRole("button", { name: "Allow once" })).toBeEnabled({ timeout: 3000 });
      await page.getByRole("button", { name: "Allow once" }).click();
      await expect(deny).toBeHidden();
      guard.expectClean();
    });
  });
}
