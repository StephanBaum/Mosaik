import { describe, expect, it } from "vitest";
import type { ImportResult } from "../../ipc/contract/index.ts";
import { importComplete } from "./CleanupStep.tsx";

const done: ImportResult = {
  imported: 3,
  skipped: 0,
  failed: 0,
  attachments: 1,
  attachmentsFailed: 0,
  clean: true,
};

describe("importComplete", () => {
  it("allows deleting the file only for a clean import", () => {
    expect(importComplete(done)).toBe(true);
  });

  it("keeps the file when the shell says something did not come across completely", () => {
    // Entries skipped while reading, or an attachment left out: no failure count, but not clean.
    expect(importComplete({ ...done, clean: false })).toBe(false);
  });

  it("keeps the file after a failed entry or attachment", () => {
    expect(importComplete({ ...done, failed: 1 })).toBe(false);
    expect(importComplete({ ...done, attachmentsFailed: 1 })).toBe(false);
  });
});
