import { describe, expect, it } from "vitest";
import type { ImportPreview } from "../../ipc/contract/index.ts";
import { notComing } from "./NotComing.tsx";
import { sourceById } from "./sources.ts";

const preview = (over: Partial<ImportPreview>): ImportPreview => ({
  previewId: "p",
  format: "keeper",
  entries: [],
  skipped: [],
  counts: [],
  attachments: 0,
  attachmentBytes: 0,
  warnings: [],
  ...over,
});

describe("notComing", () => {
  it("lists the source's gaps first, then what the preview adds, without duplicates", () => {
    const lines = notComing(
      preview({
        skipped: [
          { index: 1, reason: "empty" },
          { index: 2, reason: "malformed" },
        ],
        warnings: ["attachmentsSkipped", "entriesSkipped"],
      }),
      sourceById("keeper"),
    );
    expect(lines.map((line) => line.key)).toEqual([
      "import.lost.attachments",
      "import.lost.passkeys",
      "import.lost.entriesSkipped",
      "import.lost.attachmentsSkipped",
    ]);
    expect(lines[2]?.values).toEqual({ count: 2 });
  });

  it("is empty for a source without gaps and a clean preview", () => {
    expect(notComing(preview({}), sourceById("keepass"))).toEqual([]);
  });

  it("adds the passkey warnings of the preview", () => {
    const keys = notComing(preview({ warnings: ["passkeysNotImported"] }), sourceById("enpass")).map(
      (line) => line.key,
    );
    expect(keys).toEqual(["import.warning.passkeysNotImported"]);
  });
});
