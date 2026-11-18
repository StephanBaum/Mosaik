import { describe, expect, it } from "vitest";
import { budgetViolations, type BuildReport, type PageReport } from "./budget.ts";

const page = (o: Partial<PageReport> = {}): PageReport => ({
  file: "index.html",
  html: 1,
  initial: 1,
  jsMax: 1,
  eager: [],
  highPriority: 0,
  ...o,
});
const ok: BuildReport = {
  pages: [page()],
  css: 1,
  entry: 1,
  lazy: { "src/field/bootField.ts": 1 },
  fonts: { "dm.woff2": { bytes: 1, preload: true } },
};

describe("budgetViolations", () => {
  it("passes a report inside every budget", () => {
    expect(budgetViolations(ok)).toEqual([]);
  });

  it.each([
    [{ ...ok, css: 30_001 }, /css 30001 B > 30000 B/],
    [{ ...ok, entry: 8_001 }, /entry 8001 B > 8000 B/],
    [{ ...ok, lazy: { "src/field/bootField.ts": 16_001 } }, /bootField\.ts 16001 B > 16000 B/],
    [{ ...ok, lazy: { "src/islands/print.ts": 4_001 } }, /print\.ts 4001 B > 4000 B/],
    [{ ...ok, pages: [page({ html: 40_001 })] }, /index\.html: html 40001 B > 40000 B/],
    [{ ...ok, pages: [page({ highPriority: 2 })] }, /index\.html: 2 fetchpriority=high images/],
    [{ ...ok, pages: [page({ jsMax: 50_001 })] }, /page js 50001 B > 50000 B/],
    [{ ...ok, pages: [page({ initial: 200_001 })] }, /initial transfer 200001 B > 200000 B/],
    [{ ...ok, pages: [page({ eager: [{ src: "/img/a.webp", bytes: 80_001 }] })] }, /eager image \/img\/a\.webp 80001 B > 80000 B/],
    [{ ...ok, fonts: { "dm.woff2": { bytes: 40_001, preload: true } } }, /dm\.woff2 40001 B > 40000 B/],
    [{ ...ok, fonts: { "mono.woff2": { bytes: 16_001, preload: false } } }, /mono\.woff2 16001 B > 16000 B/],
  ] as const)("reports %#", (report, line) => {
    expect(budgetViolations(report).join("\n")).toMatch(line);
  });
});
