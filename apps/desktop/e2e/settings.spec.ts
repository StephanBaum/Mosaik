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
  });
}

test.describe("set up a phone", () => {
  test.use({ locale: "en-US" });

  test("setup-code dialog hides on blur", async ({ page }) => {
    const guard = watch(page);
    await page.goto("/?mock=unlocked&sync=hosted");
    await page.getByRole("button", { name: "Settings" }).click();
    await page.getByRole("button", { name: "Set up a phone" }).click();
    await page.getByLabel("Master password", { exact: true }).fill("mosaic");
    await page.getByRole("button", { name: "Confirm" }).click();
    const code = page.getByRole("img", { name: "Setup code for your phone" });
    await expect(code).toBeVisible();
    await expectAccessible(page);

    // Blur hides it.
    await page.evaluate(() => {
      window.dispatchEvent(new Event("blur"));
    });
    await expect(code).toBeHidden();
    await expect(page.getByText("You left this window, so the code is hidden.")).toBeVisible();

    guard.expectClean();
  });
});
