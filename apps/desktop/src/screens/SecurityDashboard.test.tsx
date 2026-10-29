import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import { createMockIpc } from "../ipc/mock.ts";

describe("security dashboard", () => {
  it("shows counts and the findings, with the 2FA attribution", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Security" }));

    await screen.findByRole("heading", { name: "Security" });
    expect(await screen.findByText("Missing 2FA")).toBeTruthy();
    expect(screen.getByText("Unsecured HTTP")).toBeTruthy();
    expect(screen.getByText(/2FA Directory/)).toBeTruthy();

    await user.click(await screen.findByRole("button", { name: /Show all/ }));
    expect((await screen.findAllByText(/Figma offers two-factor codes/)).length).toBeGreaterThan(0);
  });

  it("opens the item of a finding", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ start: "unlocked" })} languages={["en"]} />);
    await screen.findByText("GitHub");
    await user.click(screen.getByRole("button", { name: "Security" }));
    await user.click(await screen.findByRole("button", { name: /Show all/ }));
    await user.click(await screen.findByRole("button", { name: "Open Figma" }));
    expect(await screen.findByRole("article", { name: "Figma" })).toBeTruthy();
  });
});
