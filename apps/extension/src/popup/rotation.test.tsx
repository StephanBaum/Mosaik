import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { RotationMatch } from "../shared/protocol.ts";
import { baseState, match, renderPopup, requestsOf } from "./test/harness.tsx";

const attempt = (over: Partial<RotationMatch> = {}): RotationMatch => ({
  vaultId: "v1",
  itemId: "i1",
  attemptId: "a1.dev.0",
  title: "GitHub",
  username: "alex-dev",
  host: "github.com",
  stage: "prepared",
  startedAt: 1_767_225_600_000,
  changeUrl: "https://github.com/settings/security",
  ...over,
});

const ids = { vaultId: "v1", itemId: "i1", attemptId: "a1.dev.0" };

describe("starting a change", () => {
  it("offers Change password… for an exact-address login on https and asks the background", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ matches: [match()] }), { replies: { rotationBegin: { outcome: "started" } } });
    await user.click(await screen.findByRole("button", { name: "Change password… for alex-dev" }));
    expect(requestsOf(port, "rotationBegin")).toEqual([{ op: "rotationBegin", vaultId: "v1", itemId: "i1" }]);
    expect((await screen.findByRole("status")).textContent).toBe("Started. Choose Use this password in the Mosaic app.");
  });

  it("says nothing was started when the user declined in the Mosaic app", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match()] }), { replies: { rotationBegin: { outcome: "cancelled" } } });
    await user.click(await screen.findByRole("button", { name: "Change password… for alex-dev" }));
    expect((await screen.findByRole("status")).textContent).toBe("Nothing was started.");
  });

  it("offers it neither for a same-site match, nor on http, nor for a blocked login", async () => {
    renderPopup(baseState({ matches: [match({ matchedBy: "etld1" })] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(screen.queryByRole("button", { name: /Change password… for/ })).toBeNull();
  });

  it("is not offered on an http page", async () => {
    renderPopup(
      baseState({
        tab: { origin: "http://github.com", host: "github.com", unicodeHost: null, secure: false, supported: true },
        matches: [match()],
      }),
    );
    await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(screen.queryByRole("button", { name: /Change password… for/ })).toBeNull();
  });
});

describe("the password change card", () => {
  it("shows the login and never a password, and offers the five actions", async () => {
    renderPopup(baseState({ rotations: [attempt()] }));
    const card = await screen.findByRole("region", { name: "Password change" });
    expect(within(card).getByText("For GitHub")).toBeTruthy();
    expect(within(card).getByText("alex-dev")).toBeTruthy();
    expect(card.textContent).not.toMatch(/••/);
    for (const name of [
      "Open change page",
      "Fill change form",
      "Fill new password to sign in",
      "It worked",
      "It didn't work",
      "Start over with a new password",
    ]) {
      expect(within(card).getByRole("button", { name })).toBeTruthy();
    }
    expect(within(card).getByText(/never submits the form/)).toBeTruthy();
  });

  it("says when the new password was already filled once", async () => {
    renderPopup(baseState({ rotations: [attempt({ stage: "delivered" })] }));
    expect(await screen.findByText("The new password was already filled once.")).toBeTruthy();
  });

  it("opens the change page and fills the form on a click, naming the attempt", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ rotations: [attempt()] }), {
      replies: { rotationOpenPage: {}, fillChange: { username: 0, password: 1, otp: 0, newPassword: 2 } },
    });
    await user.click(await screen.findByRole("button", { name: "Open change page" }));
    expect(requestsOf(port, "rotationOpenPage")).toEqual([{ op: "rotationOpenPage", ...ids }]);
    await user.click(await screen.findByRole("button", { name: "Fill change form" }));
    expect(requestsOf(port, "fillChange")).toEqual([{ op: "fillChange", ...ids, confirmFrame: null }]);
    await waitFor(() => {
      expect(screen.getByRole("status").textContent).toBe("Change form filled. Check it, then submit it yourself.");
    });
  });

  it("explains an ambiguous form in words", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ rotations: [attempt()] }), {
      replies: {
        fillChange: () => {
          throw new Error("changeFormAmbiguous");
        },
      },
    });
    await user.click(await screen.findByRole("button", { name: "Fill change form" }));
    expect((await screen.findByRole("status")).textContent).toBe(
      "Mosaic couldn't tell which field is which. Copy the new password from Mosaic and paste it yourself.",
    );
  });

  it("sends the frame the popup showed for a same-origin frame, after the confirmation", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ rotations: [attempt()], frame: "sameOriginFrame", frameTarget: { frameId: 5, documentId: "doc-5" } }));
    await user.click(await screen.findByRole("button", { name: "Fill change form" }));
    expect(requestsOf(port, "fillChange")).toEqual([]);
    await user.click(await screen.findByRole("button", { name: "Fill in the frame" }));
    expect(requestsOf(port, "fillChange")).toEqual([
      { op: "fillChange", ...ids, confirmFrame: { frameId: 5, documentId: "doc-5" } },
    ]);
  });

  it("confirms the change form's frame, not the sign-in form's", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(
      baseState({
        rotations: [attempt()],
        frame: "top",
        frameTarget: { frameId: 0, documentId: "doc-0" },
        changeFrame: { frame: "sameOriginFrame", target: { frameId: 7, documentId: "doc-7" } },
      }),
    );
    await user.click(await screen.findByRole("button", { name: "Fill change form" }));
    expect(requestsOf(port, "fillChange")).toEqual([]);
    await user.click(await screen.findByRole("button", { name: "Fill in the frame" }));
    expect(requestsOf(port, "fillChange")).toEqual([
      { op: "fillChange", ...ids, confirmFrame: { frameId: 7, documentId: "doc-7" } },
    ]);
  });

  it("fills nothing while the change form is in a frame of another site", async () => {
    renderPopup(
      baseState({
        rotations: [attempt()],
        changeFrame: { frame: "crossOriginFrame", target: { frameId: 7, documentId: "doc-7" } },
      }),
    );
    expect(await screen.findByRole("button", { name: "Fill change form" })).toHaveProperty("disabled", true);
  });

  it("fills nothing while the field in focus is in a frame of another site", async () => {
    renderPopup(baseState({ rotations: [attempt()], frame: "crossOriginFrame" }));
    expect(await screen.findByRole("button", { name: "Fill change form" })).toHaveProperty("disabled", true);
    expect(screen.getByRole("button", { name: "Fill new password to sign in" })).toHaveProperty("disabled", true);
  });

  it("reports the result and says what the Mosaic app decided", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ rotations: [attempt()] }), {
      replies: { rotationResult: { outcome: "confirmed" } },
    });
    await user.click(await screen.findByRole("button", { name: "It worked" }));
    expect(requestsOf(port, "rotationResult")).toEqual([{ op: "rotationResult", ...ids, outcome: "succeeded" }]);
    expect((await screen.findByRole("status")).textContent).toBe("Done. The new password is now saved in Mosaic.");
  });

  it("reports a failure, and starts over with a new password", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ rotations: [attempt()] }), {
      replies: { rotationResult: { outcome: "keptOld" }, rotationBegin: { outcome: "started" } },
    });
    await user.click(await screen.findByRole("button", { name: "It didn't work" }));
    expect(requestsOf(port, "rotationResult")).toEqual([{ op: "rotationResult", ...ids, outcome: "failed" }]);
    await waitFor(() => {
      expect(screen.getByRole("status").textContent).toBe("Mosaic kept the old password.");
    });
    await user.click(screen.getByRole("button", { name: "Start over with a new password" }));
    expect(requestsOf(port, "rotationBegin")).toEqual([{ op: "rotationBegin", vaultId: "v1", itemId: "i1" }]);
  });

  it("speaks German", async () => {
    renderPopup(baseState({ rotations: [attempt()] }), { languages: ["de"] });
    expect(await screen.findByRole("region", { name: "Passwortänderung" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Änderungsformular ausfüllen" })).toBeTruthy();
  });
});

describe("a related address", () => {
  const offer = { host: "accounts.github.com", unicodeHost: null, title: "GitHub" };

  it("offers to add it and lets the desktop decide", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ rotationTabOffer: offer }), { replies: { rotationAddOrigin: { outcome: "added" } } });
    const card = await screen.findByRole("region", { name: "A related address" });
    expect(within(card).getByText(/accounts\.github\.com, which isn't saved for GitHub/)).toBeTruthy();
    await user.click(within(card).getByRole("button", { name: "Add accounts.github.com" }));
    expect(requestsOf(port, "rotationAddOrigin")).toEqual([{ op: "rotationAddOrigin" }]);
    expect((await screen.findByRole("status")).textContent).toBe("Added. The change is shown here now.");
  });

  it("shows an address with special characters in Unicode and the refusal in words", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ rotationTabOffer: { ...offer, host: "xn--bcher-kva.github.com", unicodeHost: "bücher.github.com" } }), {
      replies: {
        rotationAddOrigin: () => {
          throw new Error("originNotRelated");
        },
      },
    });
    await user.click(await screen.findByRole("button", { name: "Add bücher.github.com" }));
    expect((await screen.findByRole("status")).textContent).toBe(
      "That address isn't on the same site as this login, so Mosaic didn't add it.",
    );
  });
});
