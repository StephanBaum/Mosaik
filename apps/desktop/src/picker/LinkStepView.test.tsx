import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { recordingIpc, sampleState, sampleStep, SLACK, TOOL } from "../test/pickerSupport.tsx";
import { CONFIRM_GUARD_MS, LinkStepView } from "./LinkStepView.tsx";

function renderStep(step = sampleStep(), ipc = recordingIpc()) {
  const state = sampleState({ link: step });
  const view = render(
    <I18nProvider locale="en">
      <LinkStepView state={state} step={step} ipc={ipc} />
    </I18nProvider>,
  );
  return { ipc, state, ...view };
}

const advance = { advanceTimers: (ms: number) => vi.advanceTimersByTime(ms) };

describe("LinkStepView", () => {
  beforeEach(() => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("disables the link button for one second and asks twice for unsigned programs", async () => {
    const ipc = recordingIpc({
      picker_link_confirm: { ok: true, value: { kind: "needAnother" } },
    });
    const first = sampleStep({ program: TOOL, confirmations: 2 });
    const { rerender, state } = renderStep(first, ipc);
    const button = screen.getByRole("button", { name: "Link this unsigned program" });
    expect((button as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(CONFIRM_GUARD_MS);
    });
    expect((button as HTMLButtonElement).disabled).toBe(false);
    await userEvent.click(button, advance);
    expect(ipc.calls).toEqual([{ command: "picker_link_confirm", request: { generation: 7 } }]);
    // The state moves on: one confirmation given, the guard starts again.
    const second = { ...first, confirmed: 1 };
    rerender(
      <I18nProvider locale="en">
        <LinkStepView state={{ ...state, generation: 8, link: second }} step={second} ipc={ipc} />
      </I18nProvider>,
    );
    const again = screen.getByRole("button", { name: "Confirm again to link" });
    expect((again as HTMLButtonElement).disabled).toBe(true);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(CONFIRM_GUARD_MS);
    });
    expect((again as HTMLButtonElement).disabled).toBe(false);
  });

  it("names the button by what the step does", () => {
    renderStep(sampleStep({ submit: false }));
    expect(screen.getByRole("button", { name: "Link and fill" })).toBeTruthy();
  });

  it("keeps the step and the guard when the answer is too soon", async () => {
    const ipc = recordingIpc({ picker_link_confirm: { ok: true, value: { kind: "tooSoon" } } });
    renderStep(sampleStep({ confirmations: 2, confirmed: 1 }), ipc);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(CONFIRM_GUARD_MS);
    });
    const button = screen.getByRole("button", { name: "Confirm again to link" });
    await userEvent.click(button, advance);
    expect((button as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByRole("alert")?.textContent ?? "").toBe("");
  });

  it("says what a refused confirmation was", async () => {
    const ipc = recordingIpc({
      picker_link_confirm: { ok: false, error: { code: "pickerTooSoon" } },
    });
    renderStep(sampleStep(), ipc);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(CONFIRM_GUARD_MS);
    });
    await userEvent.click(screen.getByRole("button", { name: "Link and sign in" }), advance);
    await waitFor(() => {
      expect(screen.getByRole("alert").textContent).toBe("Wait a moment, then confirm again.");
    });
  });

  it("shows the look-alike warning and the hash change", () => {
    renderStep(
      sampleStep({
        program: TOOL,
        confirmations: 2,
        lookalikeOf: "toolz.exe",
        hashChange: { oldPrefix: "0011223344556677", newPrefix: "8899aabbccddeeff" },
      }),
    );
    expect(
      screen.getByText("Careful: this name looks like toolz.exe, which you linked before."),
    ).toBeTruthy();
    expect(
      screen.getByText("Linked file 0011223344556677…, this file 8899aabbccddeeff…"),
    ).toBeTruthy();
    expect(screen.getByText("tool.exe · Not signed · C:\\Tools\\tool.exe")).toBeTruthy();
    expect(
      screen.getByText("Mosaic will fill only this exact file. If it changes, Mosaic asks again."),
    ).toBeTruthy();
  });

  it("describes a signed program and a Store app", () => {
    const view = renderStep(sampleStep({ program: SLACK }));
    expect(
      screen.getByText("slack.exe · Signed by Slack Technologies, checked by Windows"),
    ).toBeTruthy();
    expect(screen.getByText("Login: Bank")).toBeTruthy();
    view.unmount();
    renderStep(
      sampleStep({
        program: { kind: "package", exe: "calc.exe", name: "Calculator", signer: "Microsoft", path: null },
      }),
    );
    expect(screen.getByText("Calculator · Store app · Publisher Microsoft")).toBeTruthy();
  });
});
