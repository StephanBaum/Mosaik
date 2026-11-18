/** Capability fallbacks: reduced motion, Save-Data, no WebGL2, the motion toggle and a lost context. */
import { expect, test, type Page } from "@playwright/test";
import { chunkFile, forceField, fxDecided, noWebGl2, saveData } from "./fixtures.ts";

const FIELD = chunkFile("src/field/bootField.ts");
const HOLO = chunkFile("src/holo/index.ts");

function trackRequests(page: Page): string[] {
  const urls: string[] = [];
  page.on("request", (r) => urls.push(r.url()));
  return urls;
}

const has = (urls: string[], file: string): boolean => urls.some((u) => u.endsWith(file));

test("reduced motion: no field chunk, no canvas, demos at rest, the still shows", async ({ page }) => {
  await forceField(page);
  await page.emulateMedia({ reducedMotion: "reduce" });
  const urls = trackRequests(page);
  await page.goto("/security/");
  expect(await fxDecided(page)).toBe("none");
  await page.waitForTimeout(800);
  expect(has(urls, FIELD)).toBe(false);
  await expect(page.locator("canvas.field")).toHaveCount(0);
  await expect(page.locator(".field-still")).toBeVisible();
  // The sync demo's final frame: warning and "checked" mark are fully shown.
  await expect(page.locator(".js-warn")).toHaveCSS("opacity", "1");
  await expect(page.locator(".js-checked")).toHaveCSS("opacity", "1");
  // Every demo is at rest: its markup does not change any more.
  const demos = page.locator("[data-demo]");
  const before = await demos.evaluateAll((els) => els.map((e) => e.outerHTML));
  await page.waitForTimeout(1200);
  const after = await demos.evaluateAll((els) => els.map((e) => e.outerHTML));
  expect(after).toEqual(before);
});

test("reduced motion: the home demos stay at rest while scrolled into view", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/");
  // The islands start after the intro and the paint have settled; the field decision comes last.
  await fxDecided(page);
  await page.waitForTimeout(800);
  const demos = page.locator("[data-demo]");
  await demos.first().scrollIntoViewIfNeeded();
  await page.waitForTimeout(1500);
  const before = await demos.evaluateAll((els) => els.map((e) => e.outerHTML));
  await page.waitForTimeout(1500);
  expect(await demos.evaluateAll((els) => els.map((e) => e.outerHTML))).toEqual(before);
});

test("Save-Data: neither the field nor the holo chunk loads", async ({ page }) => {
  await forceField(page);
  await saveData(page);
  const urls = trackRequests(page);
  await page.goto("/download/");
  expect(await fxDecided(page)).toBe("none");
  await page.waitForTimeout(800);
  expect(has(urls, FIELD)).toBe(false);
  expect(has(urls, HOLO)).toBe(false);
});

test("no WebGL2: no canvas, data-fx none, the still shows", async ({ page }) => {
  await noWebGl2(page);
  await page.goto("/");
  expect(await fxDecided(page)).toBe("none");
  await expect(page.locator("canvas.field")).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("data-fx", "none");
  await expect(page.locator(".field-still")).toBeVisible();
});

test("a forced field runs and reports its state", async ({ page }) => {
  await forceField(page);
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-fx-state", /^(full|lite)$/);
  await expect(page.locator("canvas.field")).toHaveCount(1);
});

test("the motion toggle stops the field within 1 s and the choice survives a reload", async ({ page }) => {
  await forceField(page);
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-fx-state", /^(full|lite)$/);
  await page.locator("button[data-motion-toggle]").click();
  await expect(page.locator("html")).toHaveAttribute("data-motion", "off");
  await expect(page.locator("html")).toHaveAttribute("data-fx-state", "still", { timeout: 1000 });
  await expect(page.locator(".field-still")).toBeVisible();
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-motion", "off");
  expect(await fxDecided(page)).toBe("none");
  await expect(page.locator("canvas.field")).toHaveCount(0);
});

test("motion off: demos jump to the final frame and stay there", async ({ page }) => {
  await page.goto("/security/");
  await fxDecided(page);
  await page.locator('[data-demo="sync"]').scrollIntoViewIfNeeded();
  await page.waitForTimeout(800);
  await page.locator("button[data-motion-toggle]").click();
  await expect(page.locator(".js-warn")).toHaveCSS("opacity", "1");
  const demos = page.locator("[data-demo]");
  const before = await demos.evaluateAll((els) => els.map((e) => e.outerHTML));
  await page.waitForTimeout(1500);
  expect(await demos.evaluateAll((els) => els.map((e) => e.outerHTML))).toEqual(before);
});

test("a lost WebGL context drops to the still without a page error", async ({ page }) => {
  await forceField(page);
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));
  await page.goto("/");
  await expect(page.locator("html")).toHaveAttribute("data-fx-state", /^(full|lite)$/);
  await expect(page.locator("canvas.field.is-on")).toHaveCount(1);
  await page.evaluate(() => {
    const canvas = document.querySelector<HTMLCanvasElement>("canvas.field");
    canvas?.getContext("webgl2")?.getExtension("WEBGL_lose_context")?.loseContext();
  });
  await expect(page.locator("html")).toHaveAttribute("data-fx-state", "still");
  await expect(page.locator(".field-still")).toBeVisible();
  await expect(page.locator(".field-still")).not.toHaveCSS("opacity", "0");
  await page.waitForTimeout(500);
  expect(errors).toEqual([]);
});
