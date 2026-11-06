import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import { ALLOW_GUARD_MS } from "../hooks/useAllowGuard.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";

const GET = "Sign in to example.com with a passkey";
const CREATE = "Save a new passkey";
const EXCLUDED = "Passkey already saved";

async function enabled(button: HTMLElement) {
  await waitFor(
    () => {
      expect((button as HTMLButtonElement).disabled).toBe(false);
    },
    { timeout: ALLOW_GUARD_MS + 1500 },
  );
}

async function open(options: MockOptions, name: string) {
  const ipc = createMockIpc({ start: "unlocked", ...options });
  render(<App ipc={ipc} languages={["en"]} />);
  const dialog = await screen.findByRole("dialog", { name }, { timeout: 3000 });
  return { ipc, dialog };
}

describe("browser passkey dialog: sign in", () => {
  it("names the browser, the site and the secure connection, and lists the accounts", async () => {
    const { dialog } = await open({ browserPasskey: "get" }, GET);
    expect(within(dialog).getByText(/Chrome/)).toBeTruthy();
    expect(within(dialog).getByText(/login\.example\.com/)).toBeTruthy();
    expect(within(dialog).getByText("Secure connection (https)")).toBeTruthy();
    const accounts = within(dialog).getAllByRole("radio");
    expect(accounts).toHaveLength(2);
    expect((accounts[0] as HTMLInputElement).checked).toBe(true);
    expect((accounts[1] as HTMLInputElement).checked).toBe(false);
    expect(within(dialog).getByText("alex@example.com")).toBeTruthy();
    expect(within(dialog).getByText("alex.work")).toBeTruthy();
    expect(within(dialog).getByText("Work")).toBeTruthy();
    // Nothing to type, and the dialog holds no field that could carry a secret.
    expect(dialog.querySelectorAll("input[type=password]")).toHaveLength(0);
    // Cancel has the initial focus.
    await waitFor(() => {
      expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Cancel" }));
    });
  });

  it("keeps the sign-in button disabled for one second, then signs in with the chosen account", async () => {
    const user = userEvent.setup();
    const { dialog } = await open({ browserPasskey: "get" }, GET);
    const signIn = within(dialog).getByRole("button", { name: "Sign in with passkey" });
    expect((signIn as HTMLButtonElement).disabled).toBe(true);
    await user.click(within(dialog).getAllByRole("radio")[1] as HTMLElement);
    await enabled(signIn);
    await user.click(signIn);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: GET })).toBeNull();
    });
  });

  it("asks for the master password when the site wants user verification", async () => {
    const user = userEvent.setup();
    const { dialog } = await open({ browserPasskey: "uv" }, GET);
    expect(
      within(dialog).getByText(
        "The site asked for user verification. Enter your master password to confirm it's you.",
      ),
    ).toBeTruthy();
    const signIn = within(dialog).getByRole("button", { name: "Sign in with passkey" });
    await new Promise((resolve) => setTimeout(resolve, ALLOW_GUARD_MS + 300));
    // Not enough without the password.
    expect((signIn as HTMLButtonElement).disabled).toBe(true);
    const password = within(dialog).getByLabelText("Master password");
    await user.type(password, "not the password");
    await user.click(signIn);
    expect(await within(dialog).findByRole("alert")).toHaveProperty("textContent", expect.stringMatching(/didn't work/));
    // The dialog stays open and the password is gone from it.
    expect((password as HTMLInputElement).value).toBe("");
    await user.type(password, "mosaic");
    await user.click(signIn);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: GET })).toBeNull();
    });
  });

  it("switches to asking for the password when the verification ran out while the dialog was open", async () => {
    const user = userEvent.setup();
    const { dialog } = await open({ browserPasskey: "uvExpires" }, GET);
    // Nothing to type when the dialog opened.
    expect(dialog.querySelectorAll("input[type=password]")).toHaveLength(0);
    const signIn = within(dialog).getByRole("button", { name: "Sign in with passkey" });
    await enabled(signIn);
    await user.click(signIn);
    // Rust answered `reauthRequired`: the dialog stays open and shows the password field.
    expect(await within(dialog).findByRole("alert")).toHaveProperty(
      "textContent",
      expect.stringMatching(/master password/),
    );
    const password = within(dialog).getByLabelText("Master password");
    expect((signIn as HTMLButtonElement).disabled).toBe(true);
    await user.type(password, "mosaic");
    await user.click(signIn);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: GET })).toBeNull();
    });
  });

  it("lets the browser use another device", async () => {
    const user = userEvent.setup();
    const first = await open({ browserPasskey: "get" }, GET);
    await user.click(within(first.dialog).getByRole("button", { name: "Use another device or security key" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: GET })).toBeNull();
    });
    expect(first.dialog.isConnected).toBe(false);
  });

  it("closes on Esc and says so", async () => {
    const { dialog } = await open({ browserPasskey: "get" }, GET);
    expect(within(dialog).getByText("to cancel")).toBeTruthy();
    fireEvent(dialog, new Event("cancel", { cancelable: true }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: GET })).toBeNull();
    });
  });
});

describe("browser passkey dialog: create", () => {
  it("asks to create a passkey for the site, for the account, in a vault", async () => {
    const user = userEvent.setup();
    const { ipc, dialog } = await open({ browserPasskey: "create" }, CREATE);
    expect(within(dialog).getByText("Create a passkey for example.com in Mosaic?")).toBeTruthy();
    expect(within(dialog).getByText("alex@example.com")).toBeTruthy();
    expect(within(dialog).queryByLabelText("Master password")).toBeNull();
    const vault = within(dialog).getByRole("combobox", { name: "Vault" });
    await waitFor(() => {
      expect(within(vault).getAllByRole("option").length).toBeGreaterThan(0);
    });
    expect((vault as HTMLSelectElement).value).not.toBe("");
    const save = within(dialog).getByRole("button", { name: "Save passkey" });
    expect((save as HTMLButtonElement).disabled).toBe(true);
    await enabled(save);
    await user.click(save);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: CREATE })).toBeNull();
    });
    const listed = await ipc.call("list_items", {
      vaultId: null,
      trash: "exclude",
      favoritesOnly: false,
      query: null,
    });
    expect(listed.ok && listed.value.some((item) => item.kind === "passkey" && item.host === "example.com")).toBe(
      true,
    );
  });

  it("shows an exclusion as information with one button", async () => {
    const user = userEvent.setup();
    const { dialog } = await open({ browserPasskey: "excluded" }, EXCLUDED);
    expect(within(dialog).getByText("You already have a passkey for example.com in Mosaic.")).toBeTruthy();
    expect(within(dialog).queryByRole("button", { name: "Save passkey" })).toBeNull();
    expect(within(dialog).queryByRole("radio")).toBeNull();
    // The acknowledge waits out the Allow guard like an Allow.
    const ok = within(dialog).getByRole("button", { name: "OK" });
    expect(ok.getAttribute("aria-disabled")).toBe("true");
    await user.click(ok);
    expect(screen.queryByRole("dialog", { name: EXCLUDED })).not.toBeNull();
    await waitFor(
      () => {
        expect(ok.getAttribute("aria-disabled")).toBe("false");
      },
      { timeout: ALLOW_GUARD_MS + 1500 },
    );
    await user.click(ok);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: EXCLUDED })).toBeNull();
    });
  });
});

describe("browser passkey dialog: German", () => {
  it("is translated", async () => {
    const ipc = createMockIpc({ start: "unlocked", browserPasskey: "get" });
    render(<App ipc={ipc} languages={["de"]} />);
    const dialog = await screen.findByRole(
      "dialog",
      { name: "Mit einem Passkey bei example.com anmelden" },
      { timeout: 3000 },
    );
    expect(within(dialog).getByText("Sichere Verbindung (https)")).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Mit Passkey anmelden" })).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Anderes Gerät oder Sicherheitsschlüssel verwenden" })).toBeTruthy();
  });
});
