import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import { ALLOW_GUARD_MS } from "../hooks/useAllowGuard.ts";
import { createMockIpc } from "../ipc/mock.ts";
import { PASSWORD_MASK } from "./BrowserSaveDialog.tsx";

const NEW = "Save this login in Mosaic?";
const UPDATE = "Update this login in Mosaic?";

async function enabled(button: HTMLElement) {
  await waitFor(
    () => {
      expect((button as HTMLButtonElement).disabled).toBe(false);
    },
    { timeout: ALLOW_GUARD_MS + 1500 },
  );
}

describe("browser save dialog", () => {
  it("shows the site and the user name, only a mask for the password, and the HTTP switch", async () => {
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "new" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NEW }, { timeout: 3000 });
    expect(within(dialog).getByText("example.com")).toBeTruthy();
    expect(within(dialog).getByText("alex@example.com")).toBeTruthy();
    expect(within(dialog).getByText(PASSWORD_MASK)).toBeTruthy();
    expect(
      within(dialog).getByRole("switch", { name: "Allow filling on this unencrypted page" }),
    ).toBeTruthy();
    // The dialog holds no input that could carry the password.
    expect(dialog.querySelectorAll('input[type="password"]')).toHaveLength(0);
    // Cancel has the initial focus.
    await waitFor(() => {
      expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Cancel" }));
    });
  });

  it("keeps Save disabled for one second, then saves a new login", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "new" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NEW }, { timeout: 3000 });
    const save = within(dialog).getByRole("button", { name: "Save" });
    expect((save as HTMLButtonElement).disabled).toBe(true);
    await enabled(save);
    await user.click(save);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: NEW })).toBeNull();
    });
  });

  it("offers the logins an update could change, or a new one", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "update" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: UPDATE }, { timeout: 3000 });
    const target = within(dialog).getByRole("combobox", { name: "Save as" });
    const labels = within(target).getAllByRole("option").map((option) => option.textContent);
    expect(labels).toHaveLength(2);
    expect(labels[0]).toMatch(/^Update .+$/);
    expect(labels[1]).toBe("A new login");
    expect(within(dialog).getByRole("button", { name: "Update password" })).toBeTruthy();
    expect(within(dialog).queryByRole("textbox", { name: "Title" })).toBeNull();
    await user.selectOptions(target, "A new login");
    expect(within(dialog).getByRole("textbox", { name: "Title" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Save" })).toBeTruthy();
  });

  it("says Esc cancels, and describes an update as a new password for the login", async () => {
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "update" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: UPDATE }, { timeout: 3000 });
    expect(within(dialog).getByText("to cancel")).toBeTruthy();
    expect(within(dialog).queryByText("to deny")).toBeNull();
    expect(within(dialog).getByText(/offers a new password for this login\. Mosaic changes it only if you choose Update password\./)).toBeTruthy();
    expect(within(dialog).queryByText(/offers to save a login/)).toBeNull();
  });

  it("describes a new login as an offer to save one", async () => {
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "new" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NEW }, { timeout: 3000 });
    expect(within(dialog).getByText(/offers to save a login\. Mosaic writes it only if you choose Save\./)).toBeTruthy();
    expect(within(dialog).getByText("to cancel")).toBeTruthy();
  });

  it("cancels at once", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "update" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: UPDATE }, { timeout: 3000 });
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: UPDATE })).toBeNull();
    });
  });

  it("does not save without a title", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked", browserSave: "new" })} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: NEW }, { timeout: 3000 });
    await user.clear(within(dialog).getByRole("textbox", { name: "Title" }));
    await new Promise((resolve) => setTimeout(resolve, ALLOW_GUARD_MS + 300));
    const save = within(dialog).getByRole("button", { name: "Save" });
    expect((save as HTMLButtonElement).disabled).toBe(true);
  });
});
