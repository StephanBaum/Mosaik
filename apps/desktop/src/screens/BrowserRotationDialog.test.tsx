import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import { ALLOW_GUARD_MS } from "../hooks/useAllowGuard.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";
import { ROTATION_MASK } from "./BrowserRotationDialog.tsx";

const BEGIN = "Start a password change?";
const RESULT = "Did github.com accept the new password?";
const ADD = "Add gist.github.com to GitHub?";
const FIRST = "q7#Vt2!mZp9@Lx4$Nw6&";

function mock(variant: "browserBegin" | "browserResult" | "addOrigin", extra: MockOptions = {}) {
  return createMockIpc({ start: "unlocked", credRotation: { variant, stepMs: 0 }, ...extra });
}

async function enabled(button: HTMLElement) {
  await waitFor(
    () => {
      expect((button as HTMLButtonElement).disabled).toBe(false);
    },
    { timeout: ALLOW_GUARD_MS + 1500 },
  );
}

describe("browser password-change dialog", () => {
  it("starts with a mask, Cancel focused, and shows the password only on request", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("browserBegin")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: BEGIN }, { timeout: 3000 });
    expect(within(dialog).getByText(ROTATION_MASK)).toBeTruthy();
    expect(within(dialog).queryByText(FIRST)).toBeNull();
    // The dialog holds no input that could carry the password.
    expect(dialog.querySelectorAll('input[type="password"]')).toHaveLength(0);
    await waitFor(() => {
      expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Cancel" }));
    });
    await user.click(within(dialog).getByRole("button", { name: "Show" }));
    expect(await within(dialog).findByText(FIRST)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Hide" }));
    expect(within(dialog).queryByText(FIRST)).toBeNull();
    expect(within(dialog).getByText("to cancel")).toBeTruthy();
  });

  it("keeps Use this password disabled for one second, and again after a new password", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("browserBegin")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: BEGIN }, { timeout: 3000 });
    const allow = within(dialog).getByRole("button", { name: "Use this password" });
    expect((allow as HTMLButtonElement).disabled).toBe(true);
    await enabled(allow);
    await user.click(within(dialog).getByRole("button", { name: "Generate another" }));
    expect(await within(dialog).findByRole("button", { name: "Use this password" })).toHaveProperty("disabled", true);
    expect(within(dialog).getAllByRole("status").map((node) => node.textContent)).toContain("A new password was generated.");
    await user.click(within(dialog).getByRole("button", { name: "Show" }));
    expect(within(dialog).queryByText(FIRST)).toBeNull();
    expect(await within(dialog).findByText("Hd3!kP8$wQ5#zR2&mT7@")).toBeTruthy();
    const again = within(dialog).getByRole("button", { name: "Use this password" });
    await enabled(again);
    await user.click(again);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: BEGIN })).toBeNull();
    });
  });

  it("asks whether the site accepted the password; keeping the old one needs no wait", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("browserResult")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: RESULT }, { timeout: 3000 });
    const confirm = within(dialog).getByRole("button", { name: "It worked" });
    expect((confirm as HTMLButtonElement).disabled).toBe(true);
    const keep = within(dialog).getByRole("button", { name: "Keep old password" });
    expect((keep as HTMLButtonElement).disabled).toBe(false);
    await enabled(confirm);
    await user.click(confirm);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: RESULT })).toBeNull();
    });
  });

  it("calls the way out of a result \"Decide later\": the change stays open", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("browserResult")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: RESULT }, { timeout: 3000 });
    expect(within(dialog).queryByRole("button", { name: "Cancel" })).toBeNull();
    expect(within(dialog).getByText(/Started .* · the new password was filled into a page · 1 other change open/)).toBeTruthy();
    await waitFor(() => {
      expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Decide later" }));
    });
    await user.click(within(dialog).getByRole("button", { name: "Decide later" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: RESULT })).toBeNull();
    });
  });

  it("keeps focus on Generate another and announces the new password", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("browserBegin")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: BEGIN }, { timeout: 3000 });
    const again = within(dialog).getByRole("button", { name: "Generate another" });
    await user.click(again);
    await waitFor(() => {
      expect(within(dialog).getAllByRole("status").map((node) => node.textContent)).toContain("A new password was generated.");
    });
    // The body was not replaced: the same button still has focus.
    expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Generate another" }));
    expect(within(dialog).getByRole("button", { name: "Generate another" })).toBe(again);
  });

  it("keeps the old password without a delay", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("browserResult")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: RESULT }, { timeout: 3000 });
    await user.click(within(dialog).getByRole("button", { name: "Keep old password" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: RESULT })).toBeNull();
    });
  });

  it("names the address to add and waits one second before adding it", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("addOrigin")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: ADD }, { timeout: 3000 });
    expect(within(dialog).getByText(/only add it if gist\.github\.com is really part of the same site/i)).toBeTruthy();
    const add = within(dialog).getByRole("button", { name: "Add address" });
    expect((add as HTMLButtonElement).disabled).toBe(true);
    await enabled(add);
    await user.click(add);
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: ADD })).toBeNull();
    });
  });

  it("cancels with Not now", async () => {
    const user = userEvent.setup();
    render(<App ipc={mock("addOrigin")} languages={["en"]} />);
    const dialog = await screen.findByRole("dialog", { name: ADD }, { timeout: 3000 });
    await user.click(within(dialog).getByRole("button", { name: "Not now" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: ADD })).toBeNull();
    });
  });

  it("speaks German", async () => {
    render(<App ipc={mock("browserBegin")} languages={["de"]} />);
    expect(await screen.findByRole("dialog", { name: "Passwortänderung starten?" }, { timeout: 3000 })).toBeTruthy();
  });
});
