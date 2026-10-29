/** Shared checks for the Playwright specs: axe (WCAG 2.2 AA), CSP violations, off-origin requests. */
import AxeBuilder from "@axe-core/playwright";
import { expect, type Page } from "@playwright/test";

const WCAG_TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa", "wcag22aa"];
const ORIGIN = "http://localhost:1421/";

export async function expectAccessible(page: Page): Promise<void> {
  const results = await new AxeBuilder({ page }).withTags(WCAG_TAGS).analyze();
  expect(
    results.violations.map((v) => `${v.id}: ${v.help} ${v.nodes.map((n) => String(n.target)).join(" ")} ${v.nodes[0]?.any[0]?.message ?? ""}`),
  ).toEqual([]);
}

/** Collects CSP violations and requests to any other origin; call `expectClean` at the end. */
export function watch(page: Page): { expectClean: () => void } {
  const problems: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error" && /Content Security Policy/i.test(message.text())) {
      problems.push(message.text());
    }
  });
  page.on("request", (request) => {
    const url = request.url();
    if (!url.startsWith(ORIGIN) && !url.startsWith("data:") && !url.startsWith("blob:")) {
      problems.push(`off-origin request: ${url}`);
    }
  });
  return {
    expectClean: () => {
      expect(problems).toEqual([]);
    },
  };
}
