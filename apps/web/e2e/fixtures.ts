/** Shared helpers of the website's Playwright specs. */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import type { Page } from "@playwright/test";
import { ROUTES, NOT_FOUND_PATH } from "../src/pages/routes.ts";

export const ORIGIN = "http://localhost:5181";

/**
 * CI renders WebGL in software (SwiftShader). Its first context costs 1 to 2 s of main-thread
 * time and every frame is slow, which real GPUs do not pay. Budgets that this distorts are
 * asserted at full strength with `WEB_PERF_GPU=1` (a machine with a GPU); otherwise they use the
 * software ceilings named next to them.
 */
export const REAL_GPU = process.env["WEB_PERF_GPU"] === "1";

/** Every route in both languages, plus the 404 page. */
export function allPaths(): string[] {
  return [...ROUTES.flatMap((r) => [r.en, r.de]), NOT_FOUND_PATH];
}

/** Lets SwiftShader count as a capable GPU, and pins the core and memory hints so the tier is `full`. */
export async function forceField(page: Page): Promise<void> {
  await page.addInitScript(() => {
    // eslint-disable-next-line @typescript-eslint/unbound-method
    const original = HTMLCanvasElement.prototype.getContext as (this: HTMLCanvasElement, id: string, attrs?: unknown) => unknown;
    HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, id: string, attrs?: unknown): unknown {
      if (id === "webgl2" && typeof attrs === "object" && attrs !== null) {
        const rest = { ...(attrs as Record<string, unknown>) };
        delete rest["failIfMajorPerformanceCaveat"];
        return original.call(this, id, rest);
      }
      return original.call(this, id, attrs);
    } as typeof HTMLCanvasElement.prototype.getContext;
    Object.defineProperty(navigator, "hardwareConcurrency", { get: () => 8 });
    Object.defineProperty(navigator, "deviceMemory", { get: () => 8 });
  });
}

/** No WebGL2 at all. */
export async function noWebGl2(page: Page): Promise<void> {
  await page.addInitScript(() => {
    // eslint-disable-next-line @typescript-eslint/unbound-method
    const original = HTMLCanvasElement.prototype.getContext as (this: HTMLCanvasElement, id: string, attrs?: unknown) => unknown;
    HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement, id: string, attrs?: unknown): unknown {
      return id === "webgl2" ? null : original.call(this, id, attrs);
    } as typeof HTMLCanvasElement.prototype.getContext;
  });
}

/** `navigator.connection.saveData`. */
export async function saveData(page: Page): Promise<void> {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "connection", { get: () => ({ saveData: true, effectiveType: "4g" }) });
  });
}

export interface Violations {
  /** CSP and Trusted Types violations, console errors and uncaught page errors seen so far. */
  list(): Promise<string[]>;
}

/** Listens for `securitypolicyviolation` (in every document of the page), console errors and page errors. */
export async function collectViolations(page: Page): Promise<Violations> {
  const seen: string[] = [];
  await page.addInitScript(() => {
    document.addEventListener("securitypolicyviolation", (e) => {
      const w = window as unknown as { __csp?: string[] };
      (w.__csp ??= []).push(`${e.violatedDirective} ${e.blockedURI} ${e.sample}`);
    });
  });
  page.on("console", (m) => {
    if (m.type() === "error") {
      seen.push(`console: ${m.text()}`);
    }
  });
  page.on("pageerror", (e) => {
    seen.push(`pageerror: ${e.message}`);
  });
  return {
    async list() {
      const csp = await page.evaluate(() => (window as unknown as { __csp?: string[] }).__csp ?? []).catch(() => []);
      return [...seen, ...csp.map((c) => `csp: ${c}`)];
    },
  };
}

interface ManifestEntry {
  file: string;
}

/** The built file name of a chunk, from the Vite manifest (key: its source path, e.g. `src/field/bootField.ts`). */
export function chunkFile(src: string): string {
  const path = fileURLToPath(new URL("../dist/.vite/manifest.json", import.meta.url));
  const manifest = JSON.parse(readFileSync(path, "utf8")) as Record<string, ManifestEntry>;
  const entry = manifest[src];
  if (entry === undefined) {
    throw new Error(`no manifest entry for ${src}`);
  }
  return entry.file;
}

/** Waits until the capability tier is decided (`html[data-fx]`, set after load plus idle). */
export async function fxDecided(page: Page): Promise<string> {
  await page.waitForFunction(() => document.documentElement.dataset["fx"] !== undefined, undefined, { timeout: 10_000 });
  return page.evaluate(() => document.documentElement.dataset["fx"] ?? "");
}
