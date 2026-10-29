/**
 * Static guards over the app sources (UX_GUIDELINES section 2, ADR-0019, ADR-0026): no raw
 * colors or pixel sizes in components, no console, no HTML injection, no browser storage,
 * no network calls, no remote URLs. The contract and the mock IPC (`ipc/`) are not UI code.
 */
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

const srcDir = import.meta.dirname;

function sources(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      return entry.name === "ipc" || entry.name === "test" ? [] : sources(path);
    }
    return /\.tsx?$/.test(entry.name) && !/\.test\.tsx?$/.test(entry.name) ? [path] : [];
  });
}

const app = sources(srcDir);
const tsx = app.filter((path) => path.endsWith(".tsx"));

describe("app sources", () => {
  it("has files to check", () => {
    expect(tsx.length).toBeGreaterThan(15);
  });

  it.each(tsx)("%s has no raw colors, pixel sizes or inline styles", (path) => {
    const text = readFileSync(path, "utf8");
    expect(text).not.toMatch(/#[0-9a-fA-F]{3,8}\b/);
    expect(text).not.toMatch(/\b\d+px\b/);
    expect(text).not.toMatch(/\bstyle=\{/);
  });

  it.each(app)("%s has no console, injection, storage or network access", (path) => {
    const text = readFileSync(path, "utf8");
    const code = text.replace(/\/\*[\s\S]*?\*\/|\/\/.*$/gm, "");
    expect(code).not.toMatch(/\bconsole\./);
    expect(code).not.toContain("dangerouslySetInnerHTML");
    expect(code).not.toMatch(/\b(localStorage|sessionStorage|indexedDB)\b/);
    expect(code).not.toMatch(/\bfetch\(/);
    expect(code).not.toMatch(/\bXMLHttpRequest\b/);
    expect(code).not.toMatch(/https?:\/\//);
  });
});
