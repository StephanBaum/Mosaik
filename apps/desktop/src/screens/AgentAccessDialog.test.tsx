import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App.tsx";
import { ALLOW_GUARD_MS } from "../hooks/useAllowGuard.ts";
import type { EventListener, MosaicIpc } from "../ipc/client.ts";
import type { AgentAccessRequest, AppEvent, CommandName, IpcResult } from "../ipc/contract/index.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";

afterEach(() => {
  vi.restoreAllMocks();
  vi.useRealTimers();
});

const TITLE = "An agent wants to use a key";

interface Sent {
  command: CommandName;
  request: unknown;
}

const REQUEST: AgentAccessRequest = {
  requestId: "req-1",
  pairingName: "Claude Code",
  itemTitle: "Example API",
  vaultName: "Personal",
  fieldLabel: "api_key.secret",
  origins: ["https://api.example.com", "https://eu.example.com:8443"],
  inject: "bearer",
  reason: "List the open invoices",
  waiting: 0,
  requestedAt: Date.now(),
  expiresAt: Date.now() + 120_000,
};

/** The mock, recording commands, able to push events, and able to fail one command. */
function harness(options: MockOptions, failRespond?: IpcResult<null>) {
  const base = createMockIpc(options);
  const sent: Sent[] = [];
  const listeners = new Set<EventListener>();
  let failures = failRespond === undefined ? 0 : 1;
  const ipc: MosaicIpc = {
    subscribe: async (listener) => {
      listeners.add(listener);
      const stop = await base.subscribe(listener);
      return () => {
        listeners.delete(listener);
        stop();
      };
    },
    call: (command, request) => {
      sent.push({ command, request });
      if (command === "agent_access_respond" && failures > 0 && failRespond !== undefined) {
        failures -= 1;
        return Promise.resolve(failRespond);
      }
      return base.call(command, request);
    },
  };
  const emit = (event: AppEvent) => {
    act(() => {
      for (const listener of listeners) {
        listener(event);
      }
    });
  };
  return { ipc, sent, emit };
}

async function openDialog(extra: Partial<AgentAccessRequest> = {}, failRespond?: IpcResult<null>) {
  const h = harness({ start: "unlocked" }, failRespond);
  render(<App ipc={h.ipc} languages={["en"]} />);
  await screen.findByText("GitHub");
  h.emit({ type: "agentAccessRequested", request: { ...REQUEST, ...extra } });
  const dialog = await screen.findByRole("dialog", { name: TITLE });
  return { ...h, dialog };
}

const responds = (sent: Sent[]) => sent.filter((each) => each.command === "agent_access_respond");

describe("agent access dialog", () => {
  it("shows the facts first, then the reason, and sends no value anywhere", async () => {
    const { dialog } = await openDialog();
    const text = dialog.textContent;
    expect(within(dialog).getByText("Claude Code")).toBeTruthy();
    expect(within(dialog).getByText(/reports itself as .*Claude Code.*Mosaic cannot verify which program/)).toBeTruthy();
    expect(within(dialog).getByText("Example API")).toBeTruthy();
    expect(within(dialog).getByText("Personal")).toBeTruthy();
    expect(within(dialog).getByText("api_key.secret")).toBeTruthy();
    expect(within(dialog).getByText("https://api.example.com")).toBeTruthy();
    expect(within(dialog).getByText("https://eu.example.com:8443")).toBeTruthy();
    expect(within(dialog).getByText("As a bearer token in the Authorization header")).toBeTruthy();
    // The facts come before the agent's words.
    expect(text.indexOf("Example API")).toBeLessThan(text.indexOf("The agent says:"));
    expect(within(dialog).getByText("List the open invoices")).toBeTruthy();
  });

  it("deny has initial focus and Esc denies", async () => {
    const { dialog, sent } = await openDialog();
    const deny = within(dialog).getByRole("button", { name: "Deny" });
    await waitFor(() => {
      expect(document.activeElement).toBe(deny);
    });
    // Esc is the dialog's native cancel event (jsdom does not make one from the key).
    fireEvent(dialog, new Event("cancel", { cancelable: true }));
    await waitFor(() => {
      expect(responds(sent)).toEqual([
        { command: "agent_access_respond", request: { requestId: "req-1", decision: "deny" } },
      ]);
    });
  });

  it("allow buttons are disabled for one second and after refocus", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { dialog } = await openDialog();
    const allowOnce = within(dialog).getByRole("button", { name: "Allow once" });
    const allBut = ["Allow once", "Allow for 15 minutes", "Allow for 1 hour", "Allow until Mosaic locks"];
    const enabled = () =>
      allBut.map((name) => !within(dialog).getByRole<HTMLButtonElement>("button", { name }).disabled);
    expect(enabled()).toEqual([false, false, false, false]);
    expect(allowOnce).toBeTruthy();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(ALLOW_GUARD_MS + 500);
    });
    expect(enabled()).toEqual([true, true, true, true]);

    // The window loses focus and comes back: the second starts again.
    const hasFocus = vi.spyOn(document, "hasFocus");
    hasFocus.mockReturnValue(false);
    act(() => {
      window.dispatchEvent(new Event("blur"));
    });
    expect(enabled()).toEqual([false, false, false, false]);
    hasFocus.mockReturnValue(true);
    act(() => {
      window.dispatchEvent(new Event("focus"));
    });
    expect(enabled()).toEqual([false, false, false, false]);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ALLOW_GUARD_MS + 500);
    });
    expect(enabled()).toEqual([true, true, true, true]);
  });

  it("reason is labeled, isolated and not rendered as markup", async () => {
    const reason = `<b>trust me</b> <img src=x onerror=alert(1)> ‮evil ${"x".repeat(150)}`;
    const { dialog } = await openDialog({ reason });
    // The agent's words sit in their own labelled block, as text.
    const label = within(dialog).getByText("The agent says:");
    const block = label.closest("figure");
    expect(block).not.toBeNull();
    expect(block?.querySelector("b")).toBeNull();
    expect(block?.querySelector("img")).toBeNull();
    const quote = block?.querySelector("blockquote");
    expect(quote?.textContent).toBe(reason);
    expect(quote?.querySelector("bdi")).not.toBeNull();
    expect(within(dialog).getByText("This is the agent's own text, not Mosaic's.")).toBeTruthy();
    // Nothing outside the block repeats it.
    expect(dialog.textContent.split("trust me").length - 1).toBe(1);
  });

  it("no policy controls in the access dialog", async () => {
    const { dialog } = await openDialog();
    expect(within(dialog).queryAllByRole("textbox")).toEqual([]);
    expect(within(dialog).queryAllByRole("combobox")).toEqual([]);
    const names = within(dialog)
      .getAllByRole("button")
      .map((button) => button.textContent);
    expect(names).toEqual([
      "Deny",
      "Allow once",
      "Allow for 15 minutes",
      "Allow for 1 hour",
      "Allow until Mosaic locks",
    ]);
    expect(names.some((name) => /add|edit|destination|site/i.test(name))).toBe(false);
  });

  it("queue count is shown and a changed count does not restart the guard", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const { dialog, emit } = await openDialog({ waiting: 2 });
    expect(within(dialog).getByText("2 more requests waiting")).toBeTruthy();
    const allow = () => within(dialog).getByRole<HTMLButtonElement>("button", { name: "Allow once" });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(ALLOW_GUARD_MS + 500);
    });
    expect(allow().disabled).toBe(false);
    // The same request again with a shorter queue: the text changes, the guard stays.
    emit({ type: "agentAccessRequested", request: { ...REQUEST, waiting: 1 } });
    expect(within(dialog).getByText("1 more request waiting")).toBeTruthy();
    expect(allow().disabled).toBe(false);
    // Another request is another dialog with its own second.
    emit({ type: "agentAccessRequested", request: { ...REQUEST, requestId: "req-2", waiting: 0 } });
    const next = screen.getByRole("dialog", { name: TITLE });
    expect(within(next).getByRole<HTMLButtonElement>("button", { name: "Allow once" }).disabled).toBe(true);
    expect(within(next).queryByText(/more request/)).toBeNull();
  });

  it("allow too soon from Rust keeps the dialog open", async () => {
    const user = userEvent.setup();
    const { dialog, sent } = await openDialog({}, { ok: false, error: { code: "agentAllowTooSoon" } });
    const allow = within(dialog).getByRole("button", { name: "Allow once" });
    await waitFor(
      () => {
        expect((allow as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(allow);
    await waitFor(() => {
      expect(responds(sent)).toHaveLength(1);
    });
    expect(await within(dialog).findByText(/That was too quick to read the request/)).toBeTruthy();
    expect(screen.getByRole("dialog", { name: TITLE })).toBeTruthy();
    // A second try goes through to the mock, which has no such request: it is gone.
    await user.click(allow);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: TITLE })).toBeNull();
    });
  });

  it("closes when Rust says the request ended", async () => {
    const { emit } = await openDialog();
    emit({ type: "agentAccessClosed", requestId: "req-1", reason: "expired" });
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: TITLE })).toBeNull();
    });
  });

  it("the mock's own request goes through the 1 s guard to a grant", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", agent: "access" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: TITLE }, { timeout: 3000 });
    const allow = within(dialog).getByRole("button", { name: "Allow for 15 minutes" });
    expect((allow as HTMLButtonElement).disabled).toBe(true);
    await waitFor(
      () => {
        expect((allow as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(allow);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: TITLE })).toBeNull();
    });
  });
});
