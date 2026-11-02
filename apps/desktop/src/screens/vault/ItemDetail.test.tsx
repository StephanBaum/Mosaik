import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { App } from "../../App.tsx";
import { createMockIpc } from "../../ipc/mock.ts";
import { groupCode } from "./TotpCode.tsx";

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

  it("moves an item to Trash from the more-actions menu", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(screen.getByRole("menuitem", { name: "Move to Trash" }));
    await waitFor(() => {
      expect(screen.queryByRole("article", { name: "GitHub" })).toBeNull();
    });
  });

  it("asks before deleting forever and only purges on confirm", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    await user.click(screen.getByRole("button", { name: "More actions" }));
    await user.click(screen.getByRole("menuitem", { name: "Move to Trash" }));
    await waitFor(() => {
      expect(screen.queryByRole("article", { name: "GitHub" })).toBeNull();
    });

    await user.click(screen.getByRole("button", { name: /^Trash/ }));
    await user.click(await screen.findByRole("button", { name: /GitHub/ }));
    const article = await screen.findByRole("article", { name: "GitHub" });
    expect(screen.getByText(/days? left/)).toBeTruthy();

    await user.click(within(article).getByRole("button", { name: "Delete forever" }));
    const dialog = await screen.findByRole("dialog", { name: /Delete "GitHub" forever?/ });
    expect(within(dialog).getByText("You can't undo this.")).toBeTruthy();
    // Cancel keeps the item.
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(screen.getByRole("article", { name: "GitHub" })).toBeTruthy();

    await user.click(within(screen.getByRole("article", { name: "GitHub" })).getByRole("button", { name: "Delete forever" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Delete forever" }));
    await waitFor(() => {
      expect(screen.queryByRole("article", { name: "GitHub" })).toBeNull();
    });
    expect(await screen.findByText("Deleted for good.")).toBeTruthy();
  });

  it("shows the changed date in the footer", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openGitHub(user);
    const article = screen.getByRole("article", { name: "GitHub" });
    expect(within(article).getByText(/^Changed /)).toBeTruthy();
  });
});

describe("groupCode", () => {
  it("splits six digits 3 + 3 and leaves other lengths alone", () => {
    expect(groupCode("123456")).toBe("123 456");
    expect(groupCode("12345678")).toBe("12345678");
  });
});
