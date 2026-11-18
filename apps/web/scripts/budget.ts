/**
 * Measures the built site against `src/budgets.ts`. Text is gzip level 9, fonts and images are
 * raw bytes. Reads `.vite/manifest.json` for the chunk graph and the written pages for what each
 * page loads (islands by `data-island`, preloaded fonts, eager images).
 */
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { gzipSync } from "node:zlib";
import { BUDGETS, type Budgets } from "../src/budgets.ts";
import { ISLAND_SOURCES } from "../src/islandSources.ts";

const ENTRY = "src/main.ts";
const FIELD = "src/field/bootField.ts";
const MICRO = "src/micro/index.ts";

export interface PageReport {
  file: string;
  /** gzip bytes of the HTML. */
  html: number;
  /** gzip HTML + CSS + entry JS, plus raw preloaded fonts and the largest eager image. */
  initial: number;
  /** gzip JS of everything the page can load. */
  jsMax: number;
  eager: readonly { src: string; bytes: number }[];
  /** Images with `fetchpriority="high"`. */
  highPriority: number;
}

export interface BuildReport {
  pages: readonly PageReport[];
  css: number;
  entry: number;
  lazy: Readonly<Record<string, number>>;
  fonts: Readonly<Record<string, { bytes: number; preload: boolean }>>;
}

interface Chunk {
  file: string;
  imports?: string[];
  css?: string[];
  isDynamicEntry?: boolean;
}

const gz = (buf: Buffer): number => gzipSync(buf, { level: 9 }).length;

function walk(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const path = join(dir, e.name);
    return e.isDirectory() ? walk(path) : [path];
  });
}

/** The chunk's file and its static imports, transitively. */
function closure(manifest: Readonly<Record<string, Chunk>>, key: string, into = new Set<string>()): Set<string> {
  const chunk = manifest[key];
  if (chunk === undefined || into.has(chunk.file)) {
    return into;
  }
  into.add(chunk.file);
  for (const dep of chunk.imports ?? []) {
    closure(manifest, dep, into);
  }
  return into;
}

/** The 1x candidate of every source of an eager picture, and the img src: what a browser at DPR 1 fetches. */
function eagerImages(outDir: string, html: string): { src: string; bytes: number }[] {
  const found = new Set<string>();
  for (const [block] of html.matchAll(/<picture>[\s\S]*?<\/picture>|<img\b[^>]*>/g)) {
    if (!block.includes('loading="eager"')) {
      continue;
    }
    for (const m of block.matchAll(/\ssrc[sS]et="([^"]+)"/g)) {
      const first = (m[1] ?? "").split(",")[0]?.trim().split(/\s+/)[0];
      if (first !== undefined && first !== "") {
        found.add(first);
      }
    }
    for (const m of block.matchAll(/\ssrc="([^"]+)"/g)) {
      found.add(m[1] ?? "");
    }
  }
  return [...found]
    .filter((src) => src.startsWith("/") && existsSync(join(outDir, src)))
    .map((src) => ({ src, bytes: statSync(join(outDir, src)).size }));
}

export function measureBuild(outDir: string): BuildReport {
  const manifest = JSON.parse(readFileSync(join(outDir, ".vite/manifest.json"), "utf8")) as Record<string, Chunk>;
  const size = (file: string): number => gz(readFileSync(join(outDir, file)));
  const sum = (files: Iterable<string>): number => [...files].reduce((n, f) => n + size(f), 0);

  const entryFiles = closure(manifest, ENTRY);
  const entry = sum(entryFiles);
  const css = sum(manifest[ENTRY]?.css ?? []);

  const lazyFiles: Record<string, Set<string>> = {};
  for (const [key, chunk] of Object.entries(manifest)) {
    if (chunk.isDynamicEntry === true && key.startsWith("src/")) {
      lazyFiles[key] = new Set([...closure(manifest, key)].filter((f) => !entryFiles.has(f)));
    }
  }
  const lazy = Object.fromEntries(Object.entries(lazyFiles).map(([key, files]) => [key, sum(files)]));

  const preloaded = new Set<string>();
  const pages = walk(outDir)
    .filter((f) => f.endsWith(".html"))
    .map((path): PageReport => {
      const buf = readFileSync(path);
      const html = buf.toString("utf8");
      const fonts = [...html.matchAll(/<link[^>]*rel="preload"[^>]*as="font"[^>]*>/g)]
        .map((m) => /href="([^"]+)"/.exec(m[0])?.[1] ?? "")
        .filter((href) => href !== "");
      for (const f of fonts) {
        preloaded.add(f.replace(/^\//, ""));
      }
      const js = new Set(entryFiles);
      const islands = [...html.matchAll(/data-island="([^"]+)"/g)].map((m) => m[1] ?? "");
      const sources = ISLAND_SOURCES as Readonly<Record<string, string>>;
      for (const key of [FIELD, MICRO, ...islands.map((id) => sources[id] ?? "")]) {
        for (const f of lazyFiles[key] ?? []) {
          js.add(f);
        }
      }
      const eager = eagerImages(outDir, html);
      const htmlGz = gz(buf);
      const fontBytes = fonts.reduce((n, f) => n + (existsSync(join(outDir, f)) ? statSync(join(outDir, f)).size : 0), 0);
      const imageBytes = eager.reduce((n, e) => Math.max(n, e.bytes), 0);
      return {
        file: relative(outDir, path).replace(/\\/g, "/"),
        html: htmlGz,
        initial: htmlGz + css + entry + fontBytes + imageBytes,
        jsMax: sum(js),
        eager,
        highPriority: (html.match(/fetchpriority="high"/gi) ?? []).length,
      };
    });

  const assets = join(outDir, "assets");
  const fonts: Record<string, { bytes: number; preload: boolean }> = {};
  if (existsSync(assets)) {
    for (const f of readdirSync(assets).filter((x) => x.endsWith(".woff2"))) {
      fonts[f] = { bytes: statSync(join(assets, f)).size, preload: preloaded.has(`assets/${f}`) };
    }
  }
  return { pages, css, entry, lazy, fonts };
}

/** One readable line per breach; empty when the build is inside every budget. */
export function budgetViolations(report: BuildReport, budgets: Budgets = BUDGETS): string[] {
  const out: string[] = [];
  const over = (label: string, bytes: number, limit: number): void => {
    if (bytes > limit) {
      out.push(`${label} ${String(bytes)} B > ${String(limit)} B`);
    }
  };
  over("css", report.css, budgets.cssGzip);
  over("entry", report.entry, budgets.entryGzip);
  for (const [key, bytes] of Object.entries(report.lazy)) {
    over(key, bytes, budgets.lazyGzip[key] ?? budgets.lazyDefaultGzip);
  }
  for (const [file, f] of Object.entries(report.fonts)) {
    over(file, f.bytes, f.preload ? budgets.fontPreload : budgets.fontOther);
  }
  for (const p of report.pages) {
    over(`${p.file}: html`, p.html, budgets.htmlGzip);
    over(`${p.file}: initial transfer`, p.initial, budgets.initialGzip);
    over(`${p.file}: page js`, p.jsMax, budgets.pageJsGzip);
    for (const e of p.eager) {
      over(`${p.file}: eager image ${e.src}`, e.bytes, budgets.eagerImage);
    }
    if (p.highPriority > 1) {
      out.push(`${p.file}: ${String(p.highPriority)} fetchpriority=high images (at most 1)`);
    }
  }
  return out;
}
