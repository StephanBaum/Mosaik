/** Every page, both languages: status, CSP, same-origin only, axe, 320 px overflow, no-JS reveals, contracts. */
import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { allPaths, collectViolations, forceField, fxDecided, ORIGIN } from "./fixtures.ts";

/** Comparison pages quote what other apps say about themselves; only claims about Mosaic are banned there. */
const COMPARISON = /^\/(de\/vergleich|compare)\//;
const BANNED = /\baudited\b|bank-grade|unhackable|geprüft von|unknackbar|pre-audit/i;

/** Scrolls through the page so reveals and demos run. */
async function scrollThrough(page: Page): Promise<void> {
  const height = await page.evaluate(() => document.documentElement.scrollHeight);
  for (let y = 0; y < height; y += 700) {
    await page.evaluate((top) => {
      window.scrollTo(0, top);
    }, y);
    await page.waitForTimeout(40);
  }
  await page.evaluate(() => {
    window.scrollTo(0, 0);
  });
}

for (const path of allPaths()) {
  test.describe(path, () => {
    test("loads clean: status, CSP, console, same-origin requests, axe, no password input", async ({ page }) => {
      await forceField(page);
      const violations = await collectViolations(page);
      const requests: string[] = [];
      page.on("request", (r) => requests.push(r.url()));
      const res = await page.goto(path);
      // `vite preview` serves the file 404.html as a plain file (200); the host answers unknown paths with it as 404.
      expect(res?.status()).toBe(200);
      await fxDecided(page);
      await scrollThrough(page);
      await page.waitForTimeout(1500);
      expect(await violations.list()).toEqual([]);
      expect(requests.filter((u) => new URL(u).origin !== ORIGIN)).toEqual([]);
      expect(await page.locator('input[type="password"]').count()).toBe(0);
      expect(await page.locator('a[href*="generator"], a[href*="generate"]').count()).toBe(0);
      const words = COMPARISON.test(path) ? /bank-grade|unhackable|geprüft von|unknackbar|pre-audit/i : BANNED;
      expect(await page.locator("body").innerText()).not.toMatch(words);
      const axe = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"]).analyze();
      expect(axe.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(" | ")}`)).toEqual([]);
    });

    test("fits 320 px without horizontal scroll", async ({ page }) => {
      await page.setViewportSize({ width: 320, height: 640 });
      await page.goto(path);
      await page.waitForLoadState("load");
      const m = await page.evaluate(() => ({ scroll: document.documentElement.scrollWidth, client: document.documentElement.clientWidth }));
      expect(m.scroll).toBeLessThanOrEqual(m.client);
    });
  });
}

test.describe("without JavaScript", () => {
  test.use({ javaScriptEnabled: false });
  for (const path of allPaths()) {
    test(`${path}: every reveal is visible`, async ({ page }) => {
      await page.goto(path);
      const opacities = await page.locator("[data-reveal]").evaluateAll((els) => els.map((e) => getComputedStyle(e).opacity));
      expect(opacities.filter((o) => o !== "1")).toEqual([]);
    });
  }
});

test.describe("page contracts", () => {
  const priority = ["/", "/features/", "/security/", "/download/", "/de/", "/de/funktionen/", "/de/sicherheit/", "/de/download/"];
  for (const path of priority) {
    test(`${path} has exactly one fetchpriority=high`, async ({ page }) => {
      await page.goto(path);
      const n = await page.evaluate(() => [...document.querySelectorAll("[fetchpriority]")].filter((e) => e.getAttribute("fetchpriority")?.toLowerCase() === "high").length);
      expect(n).toBe(1);
    });
  }

  test("every page has at most one fetchpriority=high", async ({ page }) => {
    for (const path of allPaths()) {
      await page.goto(path);
      const n = await page.evaluate(() => [...document.querySelectorAll("[fetchpriority]")].filter((e) => e.getAttribute("fetchpriority")?.toLowerCase() === "high").length);
      expect(n, path).toBeLessThanOrEqual(1);
    }
  });

  // The downloads island later points the hero button at the detected system's file, so the
  // page's own markup is read without JavaScript.
  test.describe("download hero jump (markup)", () => {
    test.use({ javaScriptEnabled: false });
    for (const path of ["/download/", "/de/download/"]) {
      test(`${path}: the hero jump targets #platforms (tabindex -1)`, async ({ page }) => {
        await page.goto(path);
        await expect(page.locator("a[data-jump]").first()).toHaveAttribute("href", "#platforms");
        await expect(page.locator("#platforms")).toHaveAttribute("tabindex", "-1");
      });
    }
  });

  for (const path of ["/security/", "/de/sicherheit/"]) {
    test(`${path}: three steps, WarnMock in the hero, SyncDemo in step three`, async ({ page }) => {
      await page.goto(path);
      const steps = page.locator("li.how-step");
      await expect(steps).toHaveCount(3);
      await expect(page.locator(".hero .mock--warn")).toHaveCount(1);
      await expect(steps.nth(2).locator('[data-island="demos"][data-demo="sync"]')).toHaveCount(1);
    });
  }

  test("an unknown path answers 404", async ({ page }) => {
    // The preview server has its own 404 page; the host serves 404.html with this status.
    const res = await page.goto("/no-such-page/");
    expect(res?.status()).toBe(404);
  });

  test("the 404 page is bilingual and plain", async ({ page }) => {
    await page.goto("/404.html");
    await expect(page.locator("section.hero.hero--page.hero--plain.hero--lost")).toHaveCount(1);
    const text = await page.locator("section.hero--lost").innerText();
    // Both languages on one page: the English heading and its German counterpart.
    expect(text).toMatch(/not found|lost|404/i);
    expect(text).toMatch(/nicht gefunden|nicht|Seite/i);
  });

  const DEMOS: Record<string, string[]> = {
    "/": ["fill", "agent", "import", "phone"],
    "/features/": ["cli"],
    "/security/": ["sync", "cipher"],
    "/download/next/": ["nextSteps"],
  };
  for (const [path, names] of Object.entries(DEMOS)) {
    for (const name of names) {
      test(`${path} has the ${name} demo as an img with a label`, async ({ page }) => {
        await page.goto(path);
        const demo = page.locator(`[data-island="demos"][data-demo="${name}"]`);
        await expect(demo).toHaveCount(1);
        if (name === "nextSteps") {
          // A real ordered list: it keeps its list semantics instead of becoming an image.
          expect(await demo.evaluate((e) => e.tagName)).toBe("OL");
          return;
        }
        await expect(demo).toHaveAttribute("aria-label", /.+/);
        // The terminal is a group, every other demo an image.
        await expect(demo).toHaveAttribute("role", /^(img|group)$/);
      });
    }
  }
});
