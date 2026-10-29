import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../App.tsx";
import { createMockIpc } from "../ipc/mock.ts";
import { ALLOW_GUARD_MS } from "./SshApprovalDialog.tsx";

afterEach(() => {
  vi.useRealTimers();
});

describe("SSH approval dialog", () => {
  it("announces the countdown to screen readers once, at ten seconds", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    render(<App ipc={createMockIpc({ start: "unlocked", sshRequest: true })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: "An app wants to use your SSH key" }, { timeout: 3000 });
    const live = within(dialog).getByRole("status");
    expect(live.textContent).toBe("");

    // One tick per act, as in the app: every render sees the countdown change.
    for (let elapsed = 0; elapsed < 16_000 && live.textContent === ""; elapsed += 250) {
      await act(async () => {
        await vi.advanceTimersByTimeAsync(250);
      });
    }
    expect(live.textContent).toBe("Refused automatically in 10 seconds");
    expect(within(dialog).getAllByText(/Refused automatically in/)).toHaveLength(2);

    await act(async () => {
      await vi.advanceTimersByTimeAsync(3_000);
    });
    expect(live.textContent).toBe("Refused automatically in 10 seconds");
  });

  it("focuses Deny, keeps Allow disabled for one second, then allows once", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", sshRequest: true })} languages={["en"]} />);

    const deny = await screen.findByRole("button", { name: "Deny" }, { timeout: 3000 });
    await waitFor(() => {
      expect(document.activeElement).toBe(deny);
    });
    expect(screen.getByText("git (4242)")).toBeTruthy();
    expect(screen.getByText(/SHA256:/)).toBeTruthy();
    const once = screen.getByRole("button", { name: "Allow once" });
    expect(screen.getByRole("button", { name: "Allow for 15 minutes" })).toBeTruthy();
    expect((once as HTMLButtonElement).disabled).toBe(true);

    await waitFor(
      () => {
        expect((once as HTMLButtonElement).disabled).toBe(false);
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(once);
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Deny" })).toBeNull();
    });
  });

  it("denies", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", sshRequest: true })} languages={["en"]} />);
    await user.click(await screen.findByRole("button", { name: "Deny" }, { timeout: 3000 }));
    await waitFor(() => {
      expect(screen.queryByRole("button", { name: "Deny" })).toBeNull();
    });
  });
});
