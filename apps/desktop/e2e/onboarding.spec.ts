import { expect, test } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

for (const colorScheme of ["light", "dark"] as const) {
  test.describe(`onboarding, ${colorScheme}`, () => {
    test.use({ colorScheme, locale: "en-US" });

    test("create, save and verify the kit, import, open the vault", async ({ page }) => {
      const guard = watch(page);
      await page.goto("/");
      await expect(page.getByRole("heading", { name: "Create your account" })).toBeVisible();
      await expectAccessible(page);

      await page.getByLabel("Email").fill("alex@example.com");
      await page.getByLabel("Master password", { exact: true }).fill("correct horse");
      await page.getByLabel("Confirm master password").fill("correct horse");
      await page.getByRole("button", { name: "Create account" }).click();

      await expect(page.getByRole("heading", { name: "Save your Emergency Kit" })).toBeVisible();
      await expectAccessible(page);
      const secretKey = (await page.locator(".secret-key").textContent()) ?? "";
      await page.getByRole("button", { name: "Save Emergency Kit (PDF)" }).click();
      await expect(page.getByText("Saved. Keep the file somewhere safe.")).toBeVisible();
      await page.getByRole("button", { name: "Continue" }).click();

      await expect(page.getByRole("heading", { name: "Your Secret Key" })).toBeVisible();
      await expectAccessible(page);
      await page.getByLabel("Last group of your Secret Key").fill(secretKey.split("-").at(-1) ?? "");
      await page.getByRole("button", { name: "Confirm" }).click();

      await expect(page.getByRole("heading", { name: "Bring your passwords along" })).toBeVisible();
      await expectAccessible(page);
      await page.getByRole("button", { name: "Skip for now" }).click();
      await page.getByRole("button", { name: "Open my vault" }).click();
      await expect(page.getByRole("navigation", { name: "Vaults" })).toBeVisible();
      guard.expectClean();
    });
  });
}
