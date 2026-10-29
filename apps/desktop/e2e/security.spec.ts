import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`security report, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("shows the report and the 2FA attribution", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/?mock=unlocked");
      await page.getByRole("button", { name: "Security" }).click();
      await expect(page.getByRole("heading", { name: "Security" })).toBeVisible();
      await expect(page.getByText(/2FA Directory/)).toBeVisible();
      await expectAccessible(page);
      guard.expectClean();
    });
  });
}
