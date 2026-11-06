import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../../App.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { BrowserStatus } from "../../ipc/contract/index.ts";
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
    const enabled = screen.getByRole("button", { name: "Continue" });
    expect((enabled as HTMLButtonElement).disabled).toBe(false);
    expect(document.querySelectorAll(".holo")).toHaveLength(1);
    await user.click(enabled);

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

    // D06: the optional browser extension step, then straight into the vault.
    await screen.findByRole("heading", { name: "Install the browser extension" });
    expect(screen.getByRole("progressbar").getAttribute("aria-valuetext")).toBe("Step 5 of 5");
    expect(screen.getByText("Chrome connected")).toBeTruthy();
    // No store listing yet, so no store links.
    expect(screen.queryByRole("link", { name: /^Get for/ })).toBeNull();
    expect(screen.getByText(/isn't in the browser stores yet/)).toBeTruthy();
    // A browser is connected: "Done" is the one primary button, with the done note, and no skip.
    expect(screen.getByText("You're all set")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Skip for now" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "Done" }));

    const nav = await screen.findByRole("navigation", { name: "Vaults" });
    expect(await within(nav).findByRole("button", { name: /^Personal/ })).toBeTruthy();
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

  it("shows the pairing dialog over the browser step and lists the new browser", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc({ browserPair: true })} languages={["en-US"]} />);
    await createAccount(user, "Create your account");
    await user.click(screen.getByRole("button", { name: "Create account" }));
    await screen.findByRole("heading", { name: "Save your Emergency Kit" });
    const secretKey = document.querySelector(".secret-key")?.textContent ?? "";
    await user.click(screen.getByRole("button", { name: "Save Emergency Kit (PDF)" }));
    await user.click(await screen.findByRole("button", { name: "Continue" }));
    await user.type(await screen.findByLabelText("Last group of your Secret Key"), secretKey.split("-").at(-1) ?? "");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    await user.click(await screen.findByRole("button", { name: "Skip for now" }));
    await screen.findByRole("heading", { name: "Install the browser extension" });

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("4F7-K2Q")).toBeTruthy();
    const allow = within(dialog).getByRole("button", { name: "Allow" });
    await waitFor(() => {
      expect((allow as HTMLButtonElement).disabled).toBe(false);
    });
    await user.click(allow);
    expect(await screen.findByText("Edge connected")).toBeTruthy();
    expect(screen.getByText("Chrome connected")).toBeTruthy();
  });

  it("offers only \"Skip for now\" while no browser is connected, with an honest note", async () => {
    const user = userEvent.setup();
    const base = createMockIpc();
    const ipc: MosaicIpc = {
      subscribe: (listener) => base.subscribe(listener),
      call: (command, request) =>
        command === "browser_status"
          ? Promise.resolve({ ok: true, value: { endpoint: "running", registry: "ok", pairings: [] } satisfies BrowserStatus })
          : base.call(command, request),
    };
    render(<App ipc={ipc} languages={["en-US"]} />);
    await createAccount(user, "Create your account");
    await user.click(screen.getByRole("button", { name: "Create account" }));
    await screen.findByRole("heading", { name: "Save your Emergency Kit" });
    const secretKey = document.querySelector(".secret-key")?.textContent ?? "";
    await user.click(screen.getByRole("button", { name: "Save Emergency Kit (PDF)" }));
    await user.click(await screen.findByRole("button", { name: "Continue" }));
    await user.type(await screen.findByLabelText("Last group of your Secret Key"), secretKey.split("-").at(-1) ?? "");
    await user.click(screen.getByRole("button", { name: "Confirm" }));
    await user.click(await screen.findByRole("button", { name: "Skip for now" }));
    await screen.findByRole("heading", { name: "Install the browser extension" });
    expect(screen.getByText("Waiting for the extension. No browser is connected yet.")).toBeTruthy();
    expect(screen.getByText("The extension isn't in the browser stores yet. You can connect it later in Settings → Browsers.")).toBeTruthy();
    expect(screen.queryByText(/developer build/)).toBeNull();
    expect(screen.queryByRole("button", { name: "Done" })).toBeNull();
    expect(screen.queryByText("You're all set")).toBeNull();
    await user.click(screen.getByRole("button", { name: "Skip for now" }));
    expect(await screen.findByRole("navigation", { name: "Vaults" })).toBeTruthy();
  });

  it("speaks German on the browser step", async () => {
    const user = userEvent.setup();
    render(<App ipc={createMockIpc()} languages={["de-DE"]} />);
    await createAccount(user, "Erstelle dein Konto");
    await user.click(screen.getByRole("button", { name: "Konto erstellen" }));
    await screen.findByRole("heading", { name: "Sichere dein Emergency Kit" });
    const secretKey = document.querySelector(".secret-key")?.textContent ?? "";
    await user.click(screen.getByRole("button", { name: "Emergency Kit speichern (PDF)" }));
    await user.click(await screen.findByRole("button", { name: "Weiter" }));
    await user.type(await screen.findByLabelText("Letzte Gruppe deines Secret Keys"), secretKey.split("-").at(-1) ?? "");
    await user.click(screen.getByRole("button", { name: "Bestätigen" }));
    await user.click(await screen.findByRole("button", { name: "Jetzt überspringen" }));
    expect(await screen.findByRole("heading", { name: "Installiere die Browser-Erweiterung" })).toBeTruthy();
    expect(await screen.findByRole("button", { name: "Fertig" })).toBeTruthy();
  });
});
