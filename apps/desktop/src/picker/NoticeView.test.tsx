import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { pickerNotices } from "../ipc/contract/index.ts";
import { recordingIpc, sampleState } from "../test/pickerSupport.tsx";
import { NoticeView } from "./NoticeView.tsx";

describe("NoticeView", () => {
  it.each(pickerNotices)("shows the %s notice and closes with the generation", async (notice) => {
    const ipc = recordingIpc();
    render(
      <I18nProvider locale="en">
        <NoticeView state={sampleState({ mode: "notice", notice })} ipc={ipc} />
      </I18nProvider>,
    );
    expect(screen.getByRole("status").textContent).toMatch(/\w/);
    await userEvent.click(screen.getByRole("button", { name: "Close" }));
    expect(ipc.calls).toEqual([{ command: "picker_dismiss", request: { generation: 7 } }]);
  });
});
