/** WCAG 2.2 AA check of a page with axe, as the desktop's e2e does (UX_GUIDELINES §7). */
import AxeBuilder from "@axe-core/playwright";
import { expect, type Page } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];

export async function expectAccessible(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
  expect(
    results.violations.map(
      (v) => `${v.id}: ${v.help} ${v.nodes.map((n) => String(n.target)).join(" ")} ${v.nodes[0]?.any[0]?.message ?? ""}`,
    ),
  ).toEqual([]);
}
