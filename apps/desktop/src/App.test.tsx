import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App.tsx";
import { createMockIpc } from "./ipc/mock.ts";

describe("App with the mock IPC", () => {
  it("keeps one holo button when the editor opens on an empty vault", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", emptyVault: true })} languages={["en"]} />);

    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: "New item" })).toHaveLength(2);
    });
    const buttons = screen.getAllByRole("button", { name: "New item" });
    expect(document.querySelectorAll(".holo")).toHaveLength(1);

    const cta = buttons.find((b) => b.classList.contains("holo"));
    expect(cta).toBeDefined();
    if (cta !== undefined) {
      await user.click(cta);
    }

    expect(await screen.findByRole("heading", { name: /^New / })).toBeTruthy();
    expect(document.querySelectorAll(".holo")).toHaveLength(1);
    expect(screen.getByRole("button", { name: "Save" }).classList.contains("holo")).toBe(true);

    await user.click(screen.getByRole("button", { name: "Cancel" }));
    await waitFor(() => {
      expect(screen.getAllByRole("button", { name: "New item" })).toHaveLength(2);
    });
    expect(document.querySelectorAll(".holo")).toHaveLength(1);
  });

  it("unlocks, shows a wrong password as a sentence, searches and locks again", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "locked" })} languages={["en"]} />);

    const password = await screen.findByLabelText("Master password");
    await user.type(password, "wrong");
    await user.click(screen.getByRole("button", { name: "Unlock" }));
    expect((await screen.findByRole("alert")).textContent).toMatch(/didn't work/);

    await user.type(screen.getByLabelText("Master password"), "mosaic");
    await user.click(screen.getByRole("button", { name: "Unlock" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();

    await user.type(screen.getByRole("searchbox", { name: "Search your vault" }), "jira");
    expect(await screen.findByText("Jira")).toBeTruthy();
    await waitFor(() => {
      expect(screen.queryByText("GitHub")).toBeNull();
    });

    await user.click(screen.getByRole("button", { name: "Lock" }));
    expect(await screen.findByRole("heading", { name: "Unlock Mosaic" })).toBeTruthy();
  });

  it("speaks German with informal address", async () => {
    render(<App ipc={createMockIpc({ start: "locked" })} languages={["de-DE"]} />);
    expect(await screen.findByRole("heading", { name: "Mosaic entsperren" })).toBeTruthy();
    expect(document.documentElement.lang).toBe("de");
  });

  it("shows why the vault locked", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Lock" }));
    expect(await screen.findByText("You locked Mosaic.")).toBeTruthy();
  });

  it("asks for the Emergency Kit again when it was never confirmed", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", kitUnverified: true })} languages={["en"]} />);
    await screen.findByRole("heading", { name: "Save your Emergency Kit again" });
    await user.click(screen.getByRole("button", { name: "Save Emergency Kit (PDF)" }));
    await user.type(await screen.findByLabelText("Master password"), "mosaic");
    const dialog = screen.getByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Confirm" }));
    await screen.findByRole("heading", { name: "Your Secret Key" });
    await user.type(screen.getByLabelText("Last group of your Secret Key"), "8VJ2");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect(await screen.findByRole("navigation", { name: "Vaults" })).toBeTruthy();
  });
});
