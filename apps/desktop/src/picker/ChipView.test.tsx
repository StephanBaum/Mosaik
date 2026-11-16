import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { recordingIpc, sampleState } from "../test/pickerSupport.tsx";
import { ChipView } from "./ChipView.tsx";

describe("ChipView", () => {
  it("is one button that clicks with the current generation", async () => {
    const ipc = recordingIpc();
    render(
      <I18nProvider locale="en">
        <ChipView state={sampleState({ mode: "chip" })} ipc={ipc} />
      </I18nProvider>,
    );
    const button = screen.getByRole("button", { name: "Fill with Mosaic" });
    await userEvent.click(button);
    expect(ipc.calls).toEqual([{ command: "picker_chip_click", request: { generation: 7 } }]);
  });

  it("says unlock while the vault is locked, in German too", () => {
    render(
      <I18nProvider locale="de">
        <ChipView state={sampleState({ mode: "chip", locked: true })} ipc={recordingIpc()} />
      </I18nProvider>,
    );
    expect(screen.getByRole("button", { name: "Entsperre Mosaic zum Ausfüllen" })).toBeTruthy();
  });
});
