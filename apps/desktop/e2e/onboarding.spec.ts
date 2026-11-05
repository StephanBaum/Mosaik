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

      // D06: the optional browser extension step.
      await expect(page.getByRole("heading", { name: "Install the browser extension" })).toBeVisible();
      await expect(page.getByText("Chrome connected")).toBeVisible();
      await expectAccessible(page);
      await page.getByRole("button", { name: "Done" }).click();
      await expect(page.getByRole("navigation", { name: "Vaults" })).toBeVisible();
      guard.expectClean();
    });
  });
}

test.describe("onboarding browser step, pairing", () => {
  test.use({ locale: "en-US" });

  test("a browser that asks to connect is confirmed over the step", async ({ page }) => {
    const guard = watch(page);
    await page.goto("/?browserPair=1");
    await page.getByLabel("Email").fill("alex@example.com");
    await page.getByLabel("Master password", { exact: true }).fill("correct horse");
    await page.getByLabel("Confirm master password").fill("correct horse");
    await page.getByRole("button", { name: "Create account" }).click();
    await expect(page.getByRole("heading", { name: "Save your Emergency Kit" })).toBeVisible();
    const secretKey = (await page.locator(".secret-key").textContent()) ?? "";
    await page.getByRole("button", { name: "Save Emergency Kit (PDF)" }).click();
    await page.getByRole("button", { name: "Continue" }).click();
    await page.getByLabel("Last group of your Secret Key").fill(secretKey.split("-").at(-1) ?? "");
    await page.getByRole("button", { name: "Confirm" }).click();
    await page.getByRole("button", { name: "Skip for now" }).click();

    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("4F7-K2Q")).toBeVisible();
    await expectAccessible(page);
    await expect(dialog.getByRole("button", { name: "Allow" })).toBeEnabled();
    await dialog.getByRole("button", { name: "Allow" }).click();
    await expect(page.getByText("Edge connected")).toBeVisible();
    guard.expectClean();
  });
});
