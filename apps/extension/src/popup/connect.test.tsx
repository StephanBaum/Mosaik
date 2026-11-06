import { screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { baseState, renderPopup, requestsOf } from "./test/harness.tsx";

describe("before the vault is reachable", () => {
  it("tells a developer how to install the helper when it is missing", async () => {
    renderPopup(baseState({ connection: "hostMissing" }));
    expect(await screen.findByRole("heading", { name: "Mosaic isn't set up for this browser" })).toBeTruthy();
    expect(screen.getByText(/Developers: run pnpm host:install/)).toBeTruthy();
  });

  it.each([
    ["hostForbidden", "This browser isn't allowed to connect"],
    ["desktopNotRunning", "Open Mosaic on this computer"],
    ["protocolMismatch", "Update Mosaic"],
    ["error", "Mosaic can't be reached"],
  ] as const)("shows %s", async (connection, title) => {
    renderPopup(baseState({ connection }));
    expect(await screen.findByRole("heading", { name: title })).toBeTruthy();
  });

  it("checks again when the desktop was not running", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ connection: "desktopNotRunning" }));
    await user.click(await screen.findByRole("button", { name: "Check again" }));
    expect(requestsOf(port, "getState")).toHaveLength(2);
  });

  it("connects, shows the pairing code beside the instruction, and reports a refusal", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ connection: "notPaired" }), {
      replies: {
        pair: () => {
          throw new Error("pairingDenied");
        },
      },
    });
    await user.click(await screen.findByRole("button", { name: "Connect" }));
    expect(requestsOf(port, "pair")).toHaveLength(1);
    expect((await screen.findByRole("alert")).textContent).toBe("The connection was denied in the Mosaic app. Choose Connect to try again, then choose Allow.");
  });

  it("shows the code as XXX-XXX while the user answers in the app, and takes a code pushed later", async () => {
    const { port } = renderPopup(baseState({ connection: "pairing", pairCode: null }));
    expect(await screen.findByRole("heading", { name: "Confirm in Mosaic" })).toBeTruthy();
    expect(screen.getByText("Check that Mosaic shows the same code, then click Allow there.")).toBeTruthy();
    port.push({ event: "pairCode", code: "4F7K2Q", expiresInSeconds: 120 });
    const status = await screen.findByText("4F7-K2Q");
    expect(status.getAttribute("aria-label")).toBe("4 F 7 - K 2 Q");
    // Pairing has no connect button: the answer is given in the desktop.
    expect(screen.queryByRole("button", { name: "Connect" })).toBeNull();
  });

  it("is locked without any master password field: unlocking happens in the app", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ connection: "locked" }));
    expect(await screen.findByRole("heading", { name: "Mosaic is locked" })).toBeTruthy();
    expect(screen.getByText("Unlock it in the Mosaic app to fill on github.com.")).toBeTruthy();
    expect(document.querySelector("input")).toBeNull();
    const main = screen.getByRole("main");
    await user.click(within(main).getByRole("button", { name: "Open Mosaic" }));
    expect(requestsOf(port, "focusDesktop")).toHaveLength(1);
  });

  it("speaks German", async () => {
    renderPopup(baseState({ connection: "locked" }), { languages: ["de-DE"] });
    expect(await screen.findByRole("heading", { name: "Mosaic ist gesperrt" })).toBeTruthy();
    expect(document.documentElement.lang).toBe("de");
    expect(screen.getByText("Entsperre es in der Mosaic-App, um auf github.com auszufüllen.")).toBeTruthy();
  });

  it("says so when the background does not answer", async () => {
    const { port } = renderPopup(baseState());
    port.replies.getState = () => {
      throw new Error("hostExited");
    };
    port.disconnect();
    expect((await screen.findByRole("alert")).textContent).toBe("The connection to Mosaic closed. Try again.");
  });
});
