import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import type { ImportPreview, ImportWarning } from "../../ipc/contract/index.ts";
import { PreviewList } from "./PreviewList.tsx";

function previewWith(warnings: ImportWarning[]): ImportPreview {
  return {
    previewId: "p1",
    format: "cxf",
    entries: [],
    skipped: [],
    counts: [],
    attachments: 0,
    attachmentBytes: 0,
    warnings,
  };
}

function show(warnings: ImportWarning[], locale: "en" | "de" = "en") {
  return render(
    <I18nProvider locale={locale}>
      <PreviewList preview={previewWith(warnings)} selected={new Set()} onChange={() => undefined} />
    </I18nProvider>,
  );
}

const KEEP = "Keep your old app until you've moved the passkeys.";

describe("import preview passkey warnings", () => {
  it("says the old app is still needed once, however many passkey warnings there are", () => {
    show(["passkeysNotImported", "passkeyUnreadable", "passkeyUnsupported", "passkeysSkipped"]);
    expect(screen.getAllByText(KEEP)).toHaveLength(1);
    expect(document.body.textContent.match(/Keep your old app/g)).toHaveLength(1);
    expect(screen.getByText(/Some passkeys couldn't be read and were left out\./)).toBeTruthy();
  });

  it("leaves the reminder out when no passkey was dropped", () => {
    show(["unmappedFields", "entriesSkipped"]);
    expect(screen.queryByText(KEEP)).toBeNull();
  });

  it("is translated", () => {
    show(["passkeyUnsupported"], "de");
    expect(screen.getByText("Behalte deine alte App, bis du die Passkeys umgezogen hast.")).toBeTruthy();
  });
});
