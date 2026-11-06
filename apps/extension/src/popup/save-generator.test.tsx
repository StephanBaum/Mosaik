import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { PendingSaveInfo } from "../shared/messages.ts";
import { baseState, fakeEnv, renderPopup, requestsOf } from "./test/harness.tsx";

const pending = (over: Partial<PendingSaveInfo> = {}): PendingSaveInfo => ({
  host: "fastmail.com",
  unicodeHost: null,
  username: "alex@fastmail.example",
  kind: "new",
  itemTitle: null,
  source: "capture",
  insecure: false,
  ...over,
});

describe("the save card (E04)", () => {
  it("shows the site, the user name and a hidden password, and no title or vault field", async () => {
    renderPopup(baseState({ pendingSave: pending() }));
    const card = (await screen.findByRole("heading", { name: "Save this login for fastmail.com?" })).closest("section") as HTMLElement;
    expect(within(card).getByText("alex@fastmail.example")).toBeTruthy();
    expect(within(card).getByText("••••••••")).toBeTruthy();
    expect(card.querySelector("input, select")).toBeNull();
    expect(within(card).getByRole("button", { name: "Save in Mosaic…" })).toBeTruthy();
  });

  it("asks the desktop to save and reports the outcome", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ pendingSave: pending() }), {
      replies: { savePending: () => ({ outcome: "saved" }) },
    });
    await user.click(await screen.findByRole("button", { name: "Save in Mosaic…" }));
    expect(requestsOf(port, "savePending")).toEqual([{ op: "savePending", action: "save" }]);
    await waitFor(() => {
      expect(document.querySelector(".status-bar")?.textContent).toBe("Saved in Mosaic.");
    });
  });

  it("says to confirm in the Mosaic app while it waits", async () => {
    const user = userEvent.setup();
    let release: (() => void) | undefined;
    const { port } = renderPopup(baseState({ pendingSave: pending() }));
    port.replies.savePending = () => ({ outcome: "updated" });
    const original = port.postMessage;
    port.postMessage = (message) => {
      if ((message as { request: { op: string } }).request.op === "savePending") {
        release = () => {
          original(message);
        };
        return;
      }
      original(message);
    };
    await user.click(await screen.findByRole("button", { name: "Save in Mosaic…" }));
    expect(await screen.findByText("Confirm in the Mosaic app. Nothing is saved until you do.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save in Mosaic…" })).toHaveProperty("disabled", true);
    release?.();
    await waitFor(() => {
      expect(screen.queryByText("Confirm in the Mosaic app. Nothing is saved until you do.")).toBeNull();
    });
  });

  it("Not now and Never for this site answer the background", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ pendingSave: pending() }), {
      replies: { savePending: (request) => ({ outcome: (request as { action: string }).action === "never" ? "never" : "dismissed" }) },
    });
    await user.click(await screen.findByRole("button", { name: "Not now" }));
    await user.click(screen.getByRole("button", { name: "Never for this site" }));
    expect(requestsOf(port, "savePending").map((request) => request.action)).toEqual(["dismiss", "never"]);
    expect(await screen.findByText("Mosaic won't offer to save on fastmail.com again.")).toBeTruthy();
  });

  it("names the saved login to update, and says a generated password was generated", async () => {
    renderPopup(baseState({ pendingSave: pending({ kind: "update", itemTitle: "Fastmail", source: "generated", insecure: true }) }));
    expect(await screen.findByRole("heading", { name: "Update the saved login for fastmail.com?" })).toBeTruthy();
    expect(screen.getByText("Updates “Fastmail”.")).toBeTruthy();
    expect(screen.getByText("A password Mosaic generated for fastmail.com.")).toBeTruthy();
    expect(screen.getByText("This page isn't encrypted (http).")).toBeTruthy();
  });

  it("shows an error from the desktop in words and stays", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ pendingSave: pending() }), {
      replies: {
        savePending: () => {
          throw new Error("busy");
        },
      },
    });
    await user.click(await screen.findByRole("button", { name: "Save in Mosaic…" }));
    expect(await screen.findByText("Mosaic is busy with another request. Try again in a moment.")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Save in Mosaic…" })).toBeTruthy();
  });
});

describe("the More menu", () => {
  it("asks the browser for site access in the click, then tells the background", async () => {
    const user = userEvent.setup();
    const requestSiteAccess = vi.fn(() => Promise.resolve(true));
    const { port } = renderPopup(baseState(), { env: fakeEnv({ requestSiteAccess }), replies: { saveOffersChanged: () => ({ saveOffers: true }) } });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    const item = screen.getByRole("menuitemcheckbox", { name: /Offer to save logins/ });
    expect(item.getAttribute("aria-checked")).toBe("false");
    await user.click(item);
    expect(requestSiteAccess).toHaveBeenCalledOnce();
    await waitFor(() => {
      expect(requestsOf(port, "saveOffersChanged")).toEqual([{ op: "saveOffersChanged", enabled: true }]);
    });
  });

  it("does nothing when the browser access is declined", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState(), { env: fakeEnv({ requestSiteAccess: () => Promise.resolve(false) }) });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    await user.click(screen.getByRole("menuitemcheckbox", { name: /Offer to save logins/ }));
    expect(await screen.findByText("Mosaic needs access to websites to notice logins, and you didn't allow it.")).toBeTruthy();
    expect(requestsOf(port, "saveOffersChanged")).toEqual([]);
  });

  it("turns it off and gives the access back", async () => {
    const user = userEvent.setup();
    const removeSiteAccess = vi.fn(() => Promise.resolve());
    const { port } = renderPopup(baseState({ saveOffers: true }), {
      env: fakeEnv({ removeSiteAccess }),
      replies: { saveOffersChanged: () => ({ saveOffers: false }) },
    });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    const item = screen.getByRole("menuitemcheckbox", { name: /Offer to save logins/ });
    expect(item.getAttribute("aria-checked")).toBe("true");
    await user.click(item);
    await waitFor(() => {
      expect(requestsOf(port, "saveOffersChanged")).toEqual([{ op: "saveOffersChanged", enabled: false }]);
    });
    await waitFor(() => {
      expect(removeSiteAccess).toHaveBeenCalled();
    });
  });

  it("asks for the site access in the click before turning passkeys on", async () => {
    const user = userEvent.setup();
    const requestSiteAccess = vi.fn(() => Promise.resolve(true));
    const { port } = renderPopup(baseState(), { env: fakeEnv({ requestSiteAccess }), replies: { setPasskeys: () => ({ passkeys: true }) } });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    const item = screen.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ });
    expect(item.getAttribute("aria-checked")).toBe("false");
    await user.click(item);
    expect(requestSiteAccess).toHaveBeenCalledOnce();
    await waitFor(() => {
      expect(requestsOf(port, "setPasskeys")).toEqual([{ op: "setPasskeys", enabled: true }]);
    });
  });

  it("leaves passkeys off, with its own explanation, when the site access is declined", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState(), { env: fakeEnv({ requestSiteAccess: () => Promise.resolve(false) }) });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    await user.click(screen.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ }));
    expect(await screen.findByText("Mosaic needs access to websites to answer passkey requests, and you didn't allow it.")).toBeTruthy();
    expect(requestsOf(port, "setPasskeys")).toEqual([]);
  });

  it("gives the site access back only when neither passkeys nor save capture needs it", async () => {
    const user = userEvent.setup();
    const removeSiteAccess = vi.fn(() => Promise.resolve());
    const both = renderPopup(baseState({ passkeys: true, saveOffers: true }), {
      env: fakeEnv({ removeSiteAccess }),
      replies: { setPasskeys: () => ({ passkeys: false }) },
    });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    await user.click(screen.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ }));
    await waitFor(() => {
      expect(requestsOf(both.port, "setPasskeys")).toEqual([{ op: "setPasskeys", enabled: false }]);
    });
    expect(removeSiteAccess).not.toHaveBeenCalled();
  });

  it("gives the site access back when passkeys were the only user of it", async () => {
    const user = userEvent.setup();
    const removeSiteAccess = vi.fn(() => Promise.resolve());
    renderPopup(baseState({ passkeys: true }), { env: fakeEnv({ removeSiteAccess }), replies: { setPasskeys: () => ({ passkeys: false }) } });
    await user.click(await screen.findByRole("button", { name: "More options" }));
    await user.click(screen.getByRole("menuitemcheckbox", { name: /Use Mosaic for passkeys/ }));
    await waitFor(() => {
      expect(removeSiteAccess).toHaveBeenCalled();
    });
  });

  it("disconnects only after a confirmation", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState());
    await user.click(await screen.findByRole("button", { name: "More options" }));
    await user.click(screen.getByRole("menuitem", { name: "Disconnect from Mosaic" }));
    const dialog = screen.getByRole("dialog");
    expect(requestsOf(port, "unpair")).toEqual([]);
    await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
    expect(requestsOf(port, "unpair")).toEqual([]);
    await user.click(screen.getByRole("button", { name: "More options" }));
    await user.click(screen.getByRole("menuitem", { name: "Disconnect from Mosaic" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Disconnect from Mosaic" }));
    expect(requestsOf(port, "unpair")).toHaveLength(1);
  });

  it("has no Disconnect before the browser is connected", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ connection: "notPaired" }));
    expect(screen.queryByRole("button", { name: "More options" })).toBeNull();
    await screen.findByRole("heading", { name: "Connect to Mosaic" });
    expect(user).toBeDefined();
  });
});

describe("the generator (E03)", () => {
  const generated = { value: "t7#Vq9!mR2xL$wp4Kz", entropyBits: 118, strength: "strong", rulesSource: null as null | "page" | "siteQuirks" };

  async function openGenerator(state = baseState({ generatorFill: true }), replies = { generate: () => generated }) {
    const user = userEvent.setup();
    const view = renderPopup(state, { replies });
    await user.click(await screen.findByRole("button", { name: "Password generator" }));
    return { user, ...view };
  }

  it("shows the value as a secret, with its strength, and asks for this site first", async () => {
    const { port } = await openGenerator();
    const output = await screen.findByLabelText("Generated password");
    await waitFor(() => {
      expect(output.textContent).toBe("t7#Vq9!mR2xL$wp4Kz");
    });
    expect(output.hasAttribute("data-secret")).toBe(true);
    expect(screen.getByText("Strong · 118 bits")).toBeTruthy();
    expect(screen.getByRole("heading", { name: "Generate a password for github.com" })).toBeTruthy();
    expect(requestsOf(port, "generate")[0]?.request).toEqual({ kind: "forSite", host: "github.com", rules: null });
  });

  it("says which rules the password follows", async () => {
    await openGenerator(baseState({ generatorFill: true }), { generate: () => ({ ...generated, rulesSource: "page" }) });
    expect(await screen.findByText("Follows this page's password rules")).toBeTruthy();
  });

  it("asks for a characters request with the desktop's limits once an option is touched", async () => {
    const { user, port } = await openGenerator();
    await waitFor(() => {
      expect(requestsOf(port, "generate")).toHaveLength(1);
    });
    await user.click(screen.getByRole("checkbox", { name: "Symbols" }));
    await waitFor(() => {
      expect(requestsOf(port, "generate")).toHaveLength(2);
    });
    expect(requestsOf(port, "generate")[1]?.request).toEqual({
      kind: "password",
      length: 20,
      lowercase: true,
      uppercase: true,
      digits: true,
      symbols: false,
      excludeAmbiguous: true,
    });
    const slider = screen.getByRole<HTMLInputElement>("slider", { name: "Length" });
    expect([slider.min, slider.max]).toEqual(["8", "64"]);
  });

  it("switches to a passphrase with 3 to 10 words", async () => {
    const { user, port } = await openGenerator();
    await user.click(screen.getByRole("radio", { name: "Passphrase" }));
    const slider = await screen.findByRole<HTMLInputElement>("slider", { name: "Words" });
    expect([slider.min, slider.max]).toEqual(["3", "10"]);
    await waitFor(() => {
      expect(requestsOf(port, "generate").at(-1)?.request).toMatchObject({ kind: "passphrase", words: 5 });
    });
  });

  it("fills the shown password and copies it through the desktop", async () => {
    const { user, port } = await openGenerator();
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Fill" })).toHaveProperty("disabled", false);
    });
    await user.click(screen.getByRole("button", { name: "Fill" }));
    expect(requestsOf(port, "fillGenerated")).toEqual([{ op: "fillGenerated", confirmFrame: null }]);
    await user.click(screen.getByRole("button", { name: "Copy" }));
    expect(requestsOf(port, "copy")[0]?.source).toEqual({ kind: "generated" });
  });

  it("confirms a same-origin frame before filling into it", async () => {
    const frame = { frameId: 5, documentId: "doc-5" };
    const { user, port } = await openGenerator(baseState({ generatorFill: true, frame: "sameOriginFrame", frameTarget: frame }));
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Fill" })).toHaveProperty("disabled", false);
    });
    await user.click(screen.getByRole("button", { name: "Fill" }));
    expect(requestsOf(port, "fillGenerated")).toEqual([]);
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Fill in the frame" }));
    expect(requestsOf(port, "fillGenerated")).toEqual([{ op: "fillGenerated", confirmFrame: frame }]);
  });

  it("cannot fill without a new-password field, and says what to do", async () => {
    await openGenerator(baseState({ generatorFill: false }));
    await waitFor(() => {
      expect(screen.getByLabelText("Generated password").textContent).not.toBe("");
    });
    expect(screen.getByRole("button", { name: "Fill" })).toHaveProperty("disabled", true);
    expect(screen.getByText(/click into a new-password field/)).toBeTruthy();
  });

  it("replaces a password the background no longer holds instead of filling another one", async () => {
    let calls = 0;
    const { user, port } = await openGenerator(baseState({ generatorFill: true }), {
      generate: () => ({ ...generated, value: `pw-${String(++calls)}` }),
    });
    port.replies.fillGenerated = () => {
      throw new Error("generatedExpired");
    };
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Fill" })).toHaveProperty("disabled", false);
    });
    await user.click(screen.getByRole("button", { name: "Fill" }));
    expect(await screen.findByText("That password was no longer held, so a new one was generated.")).toBeTruthy();
    await waitFor(() => {
      expect(screen.getByLabelText("Generated password").textContent).toBe("pw-2");
    });
  });
});
