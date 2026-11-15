import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { recordingIpc, sampleLogin, sampleState, SLACK, TOOL } from "../test/pickerSupport.tsx";
import { PickerList } from "./PickerList.tsx";
import { PickerShell } from "./PickerShell.tsx";

const WORK = sampleLogin(1, "Slack (work)", "alex@example.com");
const SIDE = sampleLogin(2, "Slack (side)", "sam@example.org");
const BANK = sampleLogin(3, "Bank", "alex");

const BANK_STEP = {
  login: BANK,
  program: SLACK,
  confirmations: 1,
  confirmed: 0,
  lookalikeOf: null,
  hashChange: null,
  submit: true,
};

function renderList(state = sampleState(), ipc = recordingIpc()) {
  const view = render(
    <I18nProvider locale="en">
      <PickerList state={state} ipc={ipc} />
    </I18nProvider>,
  );
  return { ipc, ...view };
}

describe("PickerList", () => {
  it("picks the preselected single match with Enter", async () => {
    const { ipc } = renderList(sampleState({ matches: [WORK], preselected: 0 }));
    // The list has the keys at once.
    expect(document.activeElement).toBe(screen.getByRole("listbox"));
    await userEvent.keyboard("{Enter}");
    expect(ipc.calls).toEqual([
      {
        command: "picker_choose",
        request: { generation: 7, vaultId: WORK.vaultId, itemId: WORK.itemId },
      },
    ]);
  });

  it("picks a login with a click", async () => {
    const { ipc } = renderList(sampleState({ matches: [WORK, SIDE] }));
    await userEvent.click(screen.getByRole("option", { name: /Slack \(side\)/ }));
    expect(ipc.calls[0]?.request).toEqual({
      generation: 7,
      vaultId: SIDE.vaultId,
      itemId: SIDE.itemId,
    });
  });

  it("moves with the arrow keys and closes with Escape", async () => {
    const { ipc } = renderList(sampleState({ matches: [WORK, SIDE] }));
    const list = screen.getByRole("listbox");
    expect(list.getAttribute("aria-activedescendant")).toBe(screen.getAllByRole("option")[0]?.id);
    await userEvent.keyboard("{ArrowDown}");
    expect(list.getAttribute("aria-activedescendant")).toBe(screen.getAllByRole("option")[1]?.id);
    await userEvent.keyboard("{ArrowDown}");
    expect(list.getAttribute("aria-activedescendant")).toBe(screen.getAllByRole("option")[1]?.id);
    await userEvent.keyboard("{ArrowUp}{ArrowUp}");
    expect(list.getAttribute("aria-activedescendant")).toBe(screen.getAllByRole("option")[0]?.id);
    await userEvent.keyboard("{ArrowDown}{Enter}");
    expect(ipc.calls.at(-1)?.request).toMatchObject({ itemId: SIDE.itemId });
    await userEvent.keyboard("{Escape}");
    expect(ipc.calls.at(-1)).toEqual({ command: "picker_dismiss", request: { generation: 7 } });
  });

  it("filters matches as the user types", async () => {
    renderList(sampleState({ matches: [WORK, SIDE, BANK] }));
    await userEvent.keyboard("sam");
    const options = screen.getAllByRole("option");
    expect(options).toHaveLength(1);
    expect(options[0]?.textContent).toContain("Slack (side)");
    const box = screen.getByRole("textbox", { name: "Search logins" });
    expect((box as HTMLInputElement).value).toBe("sam");
    // By title as well.
    await userEvent.clear(box);
    await userEvent.type(box, "bank");
    expect(screen.getAllByRole("option")).toHaveLength(1);
  });

  it("searches with the current generation when nothing matches", async () => {
    const ipc = recordingIpc({ picker_search: { ok: true, value: [BANK] } });
    renderList(sampleState({ matches: [], suggestions: [SIDE] }), ipc);
    expect(screen.getByText("No login for Slack yet")).toBeTruthy();
    expect(screen.getByText("Suggestions")).toBeTruthy();
    await userEvent.type(screen.getByRole("textbox", { name: "Search logins" }), "ba");
    expect(ipc.calls.filter((c) => c.command === "picker_search").map((c) => c.request)).toEqual([
      { generation: 7, query: "b" },
      { generation: 7, query: "ba" },
    ]);
    // A result starts the link step.
    await userEvent.click(await screen.findByRole("option", { name: /Bank/ }));
    expect(ipc.calls.at(-1)).toEqual({
      command: "picker_link_begin",
      request: { generation: 7, vaultId: BANK.vaultId, itemId: BANK.itemId },
    });
  });

  it("starts the link step from a suggestion", async () => {
    const { ipc } = renderList(sampleState({ suggestions: [SIDE] }));
    await userEvent.click(screen.getByRole("option", { name: /Slack \(side\)/ }));
    expect(ipc.calls[0]?.command).toBe("picker_link_begin");
  });

  it("shows link again with the reason", () => {
    renderList(sampleState({ linkAgain: "fileChanged", program: TOOL }));
    expect(screen.getByText("This program changed since you linked it. Link again?")).toBeTruthy();
  });

  it("shows the signer reason too", () => {
    renderList(sampleState({ linkAgain: "signerChanged" }));
    expect(screen.getByText("This program's signer changed. Link again?")).toBeTruthy();
  });

  it("links from a list that has matches and blocks the program", async () => {
    const { ipc } = renderList(sampleState({ matches: [WORK] }));
    await userEvent.click(screen.getByRole("button", { name: "Link a login…" }));
    expect(screen.queryByRole("listbox")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: "Don't offer for this program" }));
    expect(ipc.calls.at(-1)).toEqual({ command: "picker_block", request: { generation: 7 } });
  });

  it("keeps the tab order: list, search, link, block", async () => {
    renderList(sampleState({ matches: [WORK] }));
    expect(document.activeElement).toBe(screen.getByRole("listbox"));
    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("textbox", { name: "Search logins" }));
    await userEvent.tab();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Link a login…" }));
    await userEvent.tab();
    expect(document.activeElement).toBe(
      screen.getByRole("button", { name: "Don't offer for this program" }),
    );
  });

  it("shows the refusal of a stale pick", async () => {
    const ipc = recordingIpc({ picker_choose: { ok: false, error: { code: "pickerStale" } } });
    renderList(sampleState({ matches: [WORK], preselected: 0 }), ipc);
    await userEvent.keyboard("{Enter}");
    await waitFor(() => {
      expect(screen.getByRole("alert").textContent).toBe("This list is out of date. Click the field again.");
    });
  });

  it("renders no password input and no secret", () => {
    const { container } = renderList(
      sampleState({ matches: [WORK, SIDE], suggestions: [BANK] }),
    );
    expect(container.querySelector("input[type=password]")).toBeNull();
    expect(container.textContent).not.toMatch(/pw-|password/i);
  });

  it("renders in German", () => {
    render(
      <I18nProvider locale="de">
        <PickerList state={sampleState({ matches: [WORK] })} ipc={recordingIpc()} />
      </I18nProvider>,
    );
    expect(screen.getByText("Logins für Slack")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Login verknüpfen …" })).toBeTruthy();
  });

  it("shows the link step and the notice by the mode of the state", () => {
    const view = renderList(sampleState({ link: BANK_STEP }));
    expect(screen.getByText("Login: Bank")).toBeTruthy();
    view.unmount();
    renderList(sampleState({ mode: "notice", notice: "noField" }));
    expect(screen.getByRole("status").textContent).toContain("Mosaic can't see a login field here.");
  });
});

describe("PickerShell", () => {
  it("re-reads the state on every changed event", async () => {
    const answers = {
      picker_state: { ok: true, value: sampleState({ generation: 7, matches: [WORK] }) },
    } as const;
    const live: Parameters<typeof recordingIpc>[0] = { ...answers };
    const ipc = recordingIpc(live);
    render(<PickerShell ipc={ipc}>{(state) => <PickerList state={state} ipc={ipc} />}</PickerShell>);
    expect(await screen.findAllByRole("option")).toHaveLength(1);
    live.picker_state = { ok: true, value: sampleState({ generation: 8, matches: [WORK, SIDE] }) };
    act(() => {
      ipc.emit(8);
    });
    await waitFor(() => {
      expect(screen.getAllByRole("option")).toHaveLength(2);
    });
  });

  it("follows the language and theme of the state", async () => {
    const ipc = recordingIpc({
      picker_state: {
        ok: true,
        value: sampleState({ language: "de", theme: "dark", matches: [WORK] }),
      },
    });
    render(<PickerShell ipc={ipc}>{(state) => <PickerList state={state} ipc={ipc} />}</PickerShell>);
    expect(await screen.findByText("Logins für Slack")).toBeTruthy();
    expect(document.documentElement.dataset["theme"]).toBe("dark");
  });
});
