/**
 * The liquid-glass unlock and lock reveal (ADR-0032), driven on a paused clock. Glass (WebGL2)
 * and the plain lens both pass: headless Chromium may or may not have WebGL2.
 */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { watch } from "./helpers.ts";

test.use({ locale: "en-US", colorScheme: "light", viewport: { width: 1280, height: 800 } });

const T0 = new Date("2026-01-01T10:00:00Z");

async function box(locator: Locator): Promise<{ x: number; y: number; width: number; height: number }> {
  const b = await locator.boundingBox();
  if (b === null) {
    throw new Error("the element has no box");
  }
  return b;
}

const phase = (page: Page): Promise<string | null> =>
  page.locator(".reveal").getAttribute("data-phase");

/** Runs the fake clock in 50 ms steps until the phase is `wanted`. */
async function runUntilPhase(page: Page, wanted: string, steps = 100): Promise<void> {
  for (let i = 0; i < steps; i++) {
    if ((await phase(page)) === wanted) {
      return;
    }
    await page.clock.runFor(50);
  }
  expect(await phase(page)).toBe(wanted);
}

/** The mean of the points of an inline `clip-path: polygon(...)`. */
async function centroidOf(page: Page, selector: string): Promise<{ x: number; y: number }> {
  const clip = await page.locator(selector).evaluate((el) => (el as HTMLElement).style.clipPath);
  const points = [...clip.matchAll(/(-?[\d.]+)px (-?[\d.]+)px/g)].map((m) => [Number(m[1]), Number(m[2])] as const);
  expect(points.length).toBe(120);
  return {
    x: points.reduce((sum, p) => sum + p[0], 0) / points.length,
    y: points.reduce((sum, p) => sum + p[1], 0) / points.length,
  };
}

/** Whether the topmost element at a point is inside the layer. */
function hitIn(page: Page, layer: string, x: number, y: number): Promise<boolean> {
  return page.evaluate(
    ({ name, px, py }) => document.elementFromPoint(px, py)?.closest(`[data-reveal-layer="${name}"]`) != null,
    { name: layer, px: x, py: y },
  );
}

const unlockButton = (page: Page): Locator => page.getByRole("button", { name: "Unlock", exact: true });

test("unlock grows from an off-centre click, stays usable mid-animation, and lock closes into the button", async ({
  page,
}) => {
  const guard = watch(page);
  await page.clock.install({ time: T0 });
  await page.goto("/?mock=locked");
  await expect(page.locator(".reveal")).toHaveAttribute("data-phase", "idle-locked");
  await page.getByLabel("Master password", { exact: true }).fill("mosaic");
  await page.clock.pauseAt(new Date(T0.getTime() + 5000));

  // Unlock: click well left of the button centre.
  const button = await box(unlockButton(page));
  const click = { x: button.x + 24, y: button.y + button.height / 2 };
  expect(Math.hypot(click.x - (button.x + button.width / 2), click.y - (button.y + button.height / 2))).toBeGreaterThan(100);
  await page.mouse.click(click.x, click.y);
  await runUntilPhase(page, "unlocking");
  await page.clock.runFor(50);

  const mode = await page.locator(".reveal").getAttribute("data-mode");
  expect(["glass", "lens"]).toContain(mode);
  const focused = await page.evaluate(() => {
    const el = document.activeElement;
    return el?.tagName === "MAIN" && el.id === "main" && el.closest('[data-reveal-layer="vault"]') !== null;
  });
  expect(focused).toBe(true);
  const centre = await centroidOf(page, '[data-reveal-layer="vault"]');
  expect(Math.hypot(centre.x - click.x, centre.y - click.y)).toBeLessThan(30);
  expect(await hitIn(page, "vault", click.x, click.y)).toBe(true);
  // Outside the lens the vault does not take clicks, and the unlock screen below is inert.
  expect(await hitIn(page, "vault", 1275, 5)).toBe(false);
  await expect(page.locator('[data-reveal-layer="unlock"]')).toHaveAttribute("inert", "");
  await expect(page.locator('[data-reveal-layer="unlock"]')).toHaveAttribute("aria-hidden", "true");

  // The vault is usable as soon as the lens has reached a row.
  const row = page.getByRole("button", { name: /GitHub/ });
  let reached = false;
  for (let i = 0; i < 40 && !reached; i++) {
    const b = await box(row);
    reached = await hitIn(page, "vault", b.x + b.width / 2, b.y + b.height / 2);
    if (!reached) {
      await page.clock.runFor(50);
    }
  }
  expect(reached).toBe(true);
  const rowBox = await box(row);
  await page.mouse.click(rowBox.x + rowBox.width / 2, rowBox.y + rowBox.height / 2);
  await page.clock.runFor(32);
  await expect(row).toHaveAttribute("aria-current", "true");
  expect(await phase(page)).toBe("unlocking");

  // The end state is exact: nothing of the run is left behind.
  await page.clock.runFor(3000);
  expect(await phase(page)).toBe("idle-unlocked");
  await expect(page.getByLabel("Master password", { exact: true })).toHaveCount(0);
  await expect(page.locator('[data-reveal-layer="unlock"]')).toHaveCount(0);
  expect(await page.locator('[data-reveal-layer="vault"]').evaluate((el) => (el as HTMLElement).style.clipPath)).toBe("");
  await expect(page.locator(".reveal__band")).toBeHidden();
  const leftovers = await page
    .locator('[data-reveal-layer="vault"]')
    .evaluate((layer) =>
      [...layer.querySelectorAll<HTMLElement>("*")].filter((el) => el.style.opacity || el.style.transform || el.style.filter).length,
    );
  expect(leftovers).toBe(0);

  // Lock: the vault leaves the DOM at once, the unlock screen is live and focused under the closing lens.
  const lock = await box(page.getByRole("button", { name: "Lock", exact: true }));
  const lockClick = { x: lock.x + 4, y: lock.y + 4 };
  await page.mouse.click(lockClick.x, lockClick.y);
  await runUntilPhase(page, "locking");
  await page.clock.runFor(50);
  await expect(page.getByText("GitHub")).toHaveCount(0);
  await expect(page.locator('[data-reveal-layer="vault"]')).toHaveCount(0);
  await expect(page.getByLabel("Master password", { exact: true })).toBeFocused();
  const closing = await centroidOf(page, ".reveal__sealed");
  expect(Math.hypot(closing.x - lockClick.x, closing.y - lockClick.y)).toBeLessThan(30);
  expect(await hitIn(page, "unlock", 640, 400)).toBe(true);
  await page.keyboard.type("mos");
  await expect(page.getByLabel("Master password", { exact: true })).toHaveValue("mos");

  await page.clock.runFor(2500);
  expect(await phase(page)).toBe("idle-locked");
  await expect(page.locator(".reveal__sealed")).toHaveCount(0);
  await expect(page.locator(".reveal__band")).toBeHidden();
  await expect(page.getByLabel("Master password", { exact: true })).toHaveValue("mos");
  guard.expectClean();
});

test.describe("reduced motion", () => {
  test.use({ reducedMotion: "reduce" });

  test("unlock and lock are a short crossfade without glass", async ({ page }) => {
    const guard = watch(page);
    await page.clock.install({ time: T0 });
    await page.goto("/?mock=locked");
    await expect(page.locator(".reveal")).toHaveAttribute("data-phase", "idle-locked");
    await page.getByLabel("Master password", { exact: true }).fill("mosaic");
    await page.clock.pauseAt(new Date(T0.getTime() + 5000));

    await page.keyboard.press("Enter");
    await runUntilPhase(page, "unlocking");
    await expect(page.locator(".reveal")).toHaveAttribute("data-mode", "fade");
    await expect(page.locator(".reveal__band")).toBeHidden();
    await page.clock.runFor(400);
    expect(await phase(page)).toBe("idle-unlocked");

    await page.mouse.click(...(await centreOf(page.getByRole("button", { name: "Lock", exact: true }))));
    await runUntilPhase(page, "locking");
    await page.clock.runFor(400);
    expect(await phase(page)).toBe("idle-locked");
    await expect(page.locator(".reveal__sealed")).toHaveCount(0);
    guard.expectClean();
  });
});

async function centreOf(locator: Locator): Promise<[number, number]> {
  const b = await box(locator);
  return [b.x + b.width / 2, b.y + b.height / 2];
}
