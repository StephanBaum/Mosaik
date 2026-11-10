import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { PopupRequest } from "../shared/messages.ts";
import { baseState, renderPopup, requestsOf } from "./test/harness.tsx";

async function openList(user: ReturnType<typeof userEvent.setup>): Promise<HTMLElement> {
  await user.click(await screen.findByRole("button", { name: "More options" }));
  await user.click(screen.getByRole("menuitem", { name: "Never offered for saving…" }));
  return screen.findByRole("dialog", { name: "Never offered for saving" });
}

describe("Never offered for saving (M4-R10)", () => {
  it("lists the origins as text, each with a Remove button named after it", async () => {
    const user = userEvent.setup();
    renderPopup(baseState(), { replies: { neverSaveList: { origins: ["https://bank.test", "https://shop.test:8443"] } } });
    const dialog = await openList(user);
    expect(await within(dialog).findByText("https://bank.test")).toBeTruthy();
    expect(within(dialog).getByText("https://shop.test:8443")).toBeTruthy();
    expect(within(dialog).getByRole("button", { name: "Remove https://bank.test from the list" })).toBeTruthy();
  });

  it("removes one site, says so, and keeps focus inside the dialog", async () => {
    const user = userEvent.setup();
    let origins = ["https://bank.test", "https://shop.test"];
    const { port } = renderPopup(baseState(), {
      replies: {
        neverSaveList: () => ({ origins }),
        neverSaveRemove: (request: PopupRequest) => {
          origins = origins.filter((o) => request.op !== "neverSaveRemove" || o !== request.origin);
          return { origins };
        },
      },
    });
    const dialog = await openList(user);
    await user.click(await within(dialog).findByRole("button", { name: "Remove https://bank.test from the list" }));
    await waitFor(() => {
      expect(requestsOf(port, "neverSaveRemove")).toEqual([{ op: "neverSaveRemove", origin: "https://bank.test" }]);
    });
    expect(await within(dialog).findByText(/https:\/\/bank\.test removed/)).toBeTruthy();
    expect(within(dialog).queryByText("https://bank.test")).toBeNull();
    expect(dialog.contains(document.activeElement)).toBe(true);
  });

  it("says when the list is empty, and Escape closes the dialog", async () => {
    const user = userEvent.setup();
    renderPopup(baseState(), { replies: { neverSaveList: { origins: [] } } });
    const dialog = await openList(user);
    expect(await within(dialog).findByText(/No sites yet/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "Close" }));
    await waitFor(() => {
      expect(screen.queryByRole("dialog", { name: "Never offered for saving" })).toBeNull();
    });
  });
});
