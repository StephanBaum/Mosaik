import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import { createMockIpc } from "../../ipc/mock.ts";

async function createAccount(user: ReturnType<typeof userEvent.setup>, heading: string) {
  await screen.findByRole("heading", { name: heading });
  await user.type(screen.getByLabelText(/^(Email|E-Mail)$/), "alex@example.com");
  await user.type(screen.getByLabelText(/^(Master password|Master-Passwort)$/), "correct horse");
  await user.type(
    screen.getByLabelText(/^(Confirm master password|Master-Passwort bestätigen)$/),
    "correct horse",
  );
}

describe("onboarding", () => {
  it("creates the account, saves and verifies the kit, skips the import and opens the vault", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc()} languages={["en-US"]} />);

    await createAccount(user, "Create your account");
    await user.click(screen.getByRole("button", { name: "Create account" }));

    await screen.findByRole("heading", { name: "Save your Emergency Kit" });
    const secretKey = document.querySelector(".secret-key")?.textContent ?? "";
    expect(secretKey).toMatch(/^M1(-[0-9A-Z]{4}){7}$/);
    const next = screen.getByRole("button", { name: "Continue" });
    expect((next as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Save Emergency Kit (PDF)" }));
    await screen.findByText("Saved. Keep the file somewhere safe.");
    await user.click(next);

    await screen.findByRole("heading", { name: "Your Secret Key" });
    const lastGroup = screen.getByLabelText("Last group of your Secret Key");
    await user.type(lastGroup, "0000");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    expect((await screen.findAllByRole("alert")).some((a) => a.textContent.includes("doesn't match"))).toBe(true);
    await user.clear(lastGroup);
    await user.type(lastGroup, secretKey.split("-").at(-1) ?? "");
    await user.click(screen.getByRole("button", { name: "Confirm" }));

    // The Secret Key is no longer on the screen.
    await screen.findByRole("heading", { name: "Bring your passwords along" });
    expect(document.body.textContent).not.toContain(secretKey);
    await user.click(screen.getByRole("button", { name: "Skip for now" }));
    await user.click(await screen.findByRole("button", { name: "Open my vault" }));

    const nav = await screen.findByRole("navigation", { name: "Vaults" });
    expect(within(nav).getByRole("button", { name: "Personal" })).toBeTruthy();
    expect(screen.getByText(/Your vault is empty/)).toBeTruthy();
  });

  it("speaks German", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc()} languages={["de-DE"]} />);
    await createAccount(user, "Erstelle dein Konto");
    await user.click(screen.getByRole("button", { name: "Konto erstellen" }));
    expect(await screen.findByRole("heading", { name: "Sichere dein Emergency Kit" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Emergency Kit speichern (PDF)" })).toBeTruthy();
  });
});
