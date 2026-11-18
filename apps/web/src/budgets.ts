/**
 * Hard size budgets of the built site (bytes). Gzip level 9 for text, raw for fonts and images.
 * `scripts/budget.ts` measures `dist/` against these and the build fails on a breach: raise a
 * limit only with a reason in the commit message, never to make a build pass.
 */
export interface Budgets {
  htmlGzip: number;
  cssGzip: number;
  entryGzip: number;
  lazyGzip: Readonly<Record<string, number>>;
  lazyDefaultGzip: number;
  pageJsGzip: number;
  fontPreload: number;
  fontOther: number;
  eagerImage: number;
  initialGzip: number;
}

export const BUDGETS = {
  htmlGzip: 40_000,
  cssGzip: 30_000,
  entryGzip: 8_000,
  lazyGzip: {
    "src/field/bootField.ts": 16_000,
    "src/holo/index.ts": 12_000,
    "src/demos/bootDemos.ts": 12_000,
    "src/micro/index.ts": 3_000,
  },
  lazyDefaultGzip: 4_000,
  pageJsGzip: 50_000,
  fontPreload: 40_000,
  fontOther: 16_000,
  eagerImage: 80_000,
  initialGzip: 200_000,
} as const satisfies Budgets;
