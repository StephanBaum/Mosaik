import { expect, test, type Page } from "@playwright/test";
import { expectAccessible, watch } from "./helpers.ts";

async function openPassport(page: Page) {
  await page.goto("/?mock=unlocked");
  await page.getByRole("button", { name: /Passport scan/ }).click();
  const article = page.getByRole("article", { name: "Passport scan" });
  await expect(article).toBeVisible();
  return article;
}

test.describe("file attachments", () => {
  test.use({ locale: "en-US" });

  test("a document shows its files first, with preview, save as, add and remove", async ({ page }) => {
    const guard = watch(page);
    const article = await openPassport(page);

    // 1. The Files section comes first and has two rows.
    const files = article.getByRole("region", { name: "Files" });
    await expect(files.getByRole("listitem")).toHaveCount(2);
    const filesTop = (await files.boundingBox())?.y ?? 0;
    const notesTop = (await article.getByText("Scans for the visa application.").boundingBox())?.y ?? 0;
    expect(filesTop).toBeLessThan(notesTop);
    await expectAccessible(page);

    // 2. Preview the image: a canvas with pixels; Esc closes and focus returns to the button.
    const previewButton = files.getByRole("button", { name: "Preview passport.png" });
    await previewButton.click();
    const dialog = page.getByRole("dialog", { name: "passport.png" });
    await expect(dialog).toBeVisible();
    const canvas = dialog.getByRole("img", { name: "Preview of passport.png" });
    await expect(canvas).toBeVisible();
    const size = await canvas.evaluate((el) => [(el as HTMLCanvasElement).width, (el as HTMLCanvasElement).height]);
    expect(size).toEqual([64, 48]);
    const nonBlank = await canvas.evaluate((el) => {
      const context = (el as HTMLCanvasElement).getContext("2d");
      return context?.getImageData(10, 10, 1, 1).data.some((value) => value !== 0) ?? false;
    });
    expect(nonBlank).toBe(true);
    await expect(dialog.locator("[data-secret]")).toHaveCount(1);
    await expect(dialog.locator("[data-mask]")).toHaveCount(0);
    await expectAccessible(page);
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(previewButton).toBeFocused();

    // 3. The PDF has no Preview button; Save as shows the toast.
    await expect(files.getByRole("button", { name: "Preview contract.pdf" })).toHaveCount(0);
    await files.getByRole("button", { name: "Save contract.pdf as…" }).click();
    await expect(page.getByText("File saved.")).toBeVisible();

    // 4. Add a file.
    await files.getByRole("button", { name: "Add file…" }).click();
    await expect(page.getByText("File added.")).toBeVisible();
    await expect(files.getByRole("listitem")).toHaveCount(3);
    await expect(files.getByText("scan-2.png")).toBeVisible();

    // 5. Remove one: the confirmation keeps the file by default.
    await files.getByRole("button", { name: "Remove contract.pdf" }).click();
    const confirm = page.getByRole("dialog", { name: "Remove contract.pdf?" });
    await expect(confirm.getByRole("button", { name: "Keep file" })).toBeFocused();
    await expectAccessible(page);
    await confirm.getByRole("button", { name: "Remove file" }).click();
    await expect(files.getByRole("listitem")).toHaveCount(2);
    await expect(page.getByText("File removed.")).toBeVisible();
    await expect(files.getByText("contract.pdf")).toHaveCount(0);
    guard.expectClean();
  });

  test("locking closes the preview and an unlock does not bring it back", async ({ page }) => {
    const guard = watch(page);
    const article = await openPassport(page);
    await article.getByRole("button", { name: "Preview passport.png" }).click();
    const dialog = page.getByRole("dialog", { name: "passport.png" });
    await expect(dialog.getByRole("img", { name: "Preview of passport.png" })).toBeVisible();

    await page.keyboard.press("Control+Alt+l");
    await expect(page.getByRole("heading", { name: "Unlock Mosaic" })).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.locator("canvas[aria-label]")).toHaveCount(0);

    await page.getByLabel("Master password", { exact: true }).fill("mosaic");
    await page.getByRole("button", { name: "Unlock" }).click();
    await expect(page.getByText("GitHub")).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.locator("canvas[aria-label]")).toHaveCount(0);
    guard.expectClean();
  });

  test("the trash shows files read-only", async ({ page }) => {
    const article = await openPassport(page);
    await article.getByRole("button", { name: "More actions" }).click();
    await page.getByRole("menuitem", { name: "Move to Trash" }).click();
    await page.getByRole("button", { name: "Trash" }).click();
    await page.getByRole("button", { name: /Passport scan/ }).click();
    const files = page.getByRole("article", { name: "Passport scan" }).getByRole("region", { name: "Files" });
    await expect(files.getByRole("listitem")).toHaveCount(2);
    await expect(files.getByRole("button", { name: "Add file…" })).toHaveCount(0);
    await expect(files.getByRole("button", { name: /^Remove/ })).toHaveCount(0);
    await expect(files.getByRole("button", { name: "Preview passport.png" })).toBeVisible();
  });

  test("a failed add says what to do", async ({ page }) => {
    await page.goto("/?mock=unlocked&attachAdd=attachmentTooLarge");
    await page.getByRole("button", { name: /Passport scan/ }).click();
    const files = page.getByRole("region", { name: "Files" });
    await files.getByRole("button", { name: "Add file…" }).click();
    await expect(page.getByText(/larger than 50 MiB/)).toBeVisible();
    await expect(files.getByRole("listitem")).toHaveCount(2);
  });
});
