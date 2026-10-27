import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "./App.tsx";
import { createMockIpc } from "./ipc/mock.ts";

describe("App with the mock IPC", () => {
  it("creates an account, verifies the Emergency Kit and lands in the vault", async () => {
    const user = userEvent.setup();
    const ipc = createMockIpc();
    render(<App ipc={ipc} languages={["en-US"]} />);

    await screen.findByRole("heading", { name: "Create your account" });
    await user.type(screen.getByLabelText("Email"), "alex@example.com");
    await user.type(screen.getByLabelText("Master password"), "correct horse");
    await user.type(screen.getByLabelText("Confirm master password"), "correct horse");
    await user.click(screen.getByRole("button", { name: "Create account" }));

    await screen.findByRole("heading", { name: "Your Secret Key" });
    const secretKey = document.querySelector(".secret-key")?.textContent ?? "";
    expect(secretKey).toMatch(/^M1(-[0-9A-Z]{4}){7}$/);

    const lastGroup = screen.getByLabelText("Last group of your Secret Key");
    await user.type(lastGroup, "0000");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect((await screen.findAllByRole("alert")).some((a) => a.textContent.includes("doesn't match"))).toBe(
      true,
    );

    await user.clear(lastGroup);
    await user.type(lastGroup, secretKey.split("-").at(-1) ?? "");
    await user.click(screen.getByRole("button", { name: "Confirm" }));

    const nav = await screen.findByRole("navigation", { name: "Vaults" });
    expect(within(nav).getByRole("button", { name: "Personal" })).toBeTruthy();
    expect(screen.getByText(/Your vault is empty/)).toBeTruthy();
  });

  it("unlocks, shows a wrong password as a sentence, and locks again", async () => {
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
    expect(screen.queryByText("GitHub")).toBeNull();

    await user.click(screen.getByRole("button", { name: "Lock" }));
    expect(await screen.findByRole("heading", { name: "Unlock Mosaic" })).toBeTruthy();
  });

  it("speaks German with informal address", async () => {
    render(<App ipc={createMockIpc({ start: "locked" })} languages={["de-DE"]} />);
    expect(await screen.findByRole("heading", { name: "Mosaic entsperren" })).toBeTruthy();
    expect(document.documentElement.lang).toBe("de");
  });
});
