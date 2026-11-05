/**
 * What `e2e.ts` is replaced with in every build except `--mode e2e` (plugin `excludeE2e`). The
 * type-only import keeps the stub's signatures equal to the real ones; at run time it is empty.
 */
import type * as Real from "./e2e.ts";

export const e2eEnabled = false;
export const e2ePortFactory: typeof Real.e2ePortFactory = () => null;
export const testTabOf: typeof Real.testTabOf = () => null;
export const installE2E: typeof Real.installE2E = () => undefined;
