import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import { createMockIpc } from "../../ipc/mock.ts";

async function openImport(user: ReturnType<typeof userEvent.setup>) {
  await screen.findByText("GitHub");
  await user.keyboard("{Control>}k{/Control}");
  await user.click(await screen.findByRole("option", { name: "Import from another app" }));
  await screen.findByRole("heading", { name: "Import" });
}

describe("import flow", () => {
  it("previews with duplicates unticked, commits the rest and reminds about the file", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openImport(user);

    await user.click(screen.getByRole("button", { name: "Choose the export file" }));
    await user.click(await screen.findByRole("button", { name: "Preview" }));

    const github = await screen.findByRole("checkbox", { name: /GitHub/ });
    expect((github as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText(/You already have an item like this/)).toBeTruthy();
    const mail = screen.getByRole("checkbox", { name: /Example Mail/ });
    expect((mail as HTMLInputElement).checked).toBe(true);
    const note = screen.getByRole("checkbox", { name: /Router notes/ });
    expect((note as HTMLInputElement).checked).toBe(false);
    expect(screen.getByText(/This entry is empty/)).toBeTruthy();

    await user.click(screen.getByRole("button", { name: "Import 1 item" }));
    expect(await screen.findByText(/1 item imported/)).toBeTruthy();
    expect(screen.getByText(/Delete the export file now/)).toBeTruthy();
  });

  it("cancels and drops the preview", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await openImport(user);
    await user.click(screen.getByRole("button", { name: "Choose the export file" }));
    await user.click(await screen.findByRole("button", { name: "Preview" }));
    await screen.findByRole("checkbox", { name: /GitHub/ });
    await user.click(screen.getByRole("button", { name: "Cancel import" }));
    expect(await screen.findByText("GitHub")).toBeTruthy();
  });
});
