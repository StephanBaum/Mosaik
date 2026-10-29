import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../../App.tsx";
import { createMockIpc } from "../../ipc/mock.ts";

afterEach(() => {
  vi.useRealTimers();
});

async function openGitHub(user: ReturnType<typeof userEvent.setup>) {
  await user.click(await screen.findByRole("button", { name: /GitHub/ }));
  await screen.findByRole("article", { name: "GitHub" });
}

describe("item detail", () => {
  it("reveals a secret on request and hides it again on lock", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);

    expect(screen.queryByText("correct-horse-battery-staple")).toBeNull();
    expect(screen.getByRole("img", { name: /Hidden/ })).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Reveal Password" }));
    expect(await screen.findByText("correct-horse-battery-staple")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Lock" }));
    await screen.findByRole("heading", { name: "Unlock Mosaic" });
    expect(document.body.textContent).not.toContain("correct-horse-battery-staple");
  });

  it("labels a single URL as Website, not Websites", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    const article = screen.getByRole("article", { name: "GitHub" });
    expect(within(article).getByText("Website")).toBeTruthy();
    expect(within(article).queryByText("Websites")).toBeNull();
  });

  it("hides a revealed secret after its timeout", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    vi.useFakeTimers({ shouldAdvanceTime: true });
    await user.click(screen.getByRole("button", { name: "Reveal Password" }));
    await screen.findByText("correct-horse-battery-staple");
    await vi.advanceTimersByTimeAsync(61_000);
    await waitFor(() => {
      expect(screen.queryByText("correct-horse-battery-staple")).toBeNull();
    });
  });

  it("copies and says when the clipboard clears", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    await user.click(screen.getByRole("button", { name: "Copy Password" }));
    expect(await screen.findByText("Copied. Clears in 30 seconds.")).toBeTruthy();
  });
});
