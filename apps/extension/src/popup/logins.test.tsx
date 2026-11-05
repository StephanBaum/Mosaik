import { screen, within, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { baseState, fakeEnv, match, renderPopup, requestsOf } from "./test/harness.tsx";

const FRAME = { frameId: 5, documentId: "doc-5" };

describe("logins for the site", () => {
  it("lists the matches with a Fill button each and the hint for the shortcut", async () => {
    renderPopup(baseState({ matches: [match(), match({ itemId: "i2", title: "GitHub (work)", username: "alex-work" })] }));
    expect(await screen.findByRole("heading", { name: "For github.com" })).toBeTruthy();
    expect(screen.getByText("2 logins")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Fill alex-dev" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Fill alex-work" })).toBeTruthy();
    expect(await screen.findByText("Ctrl+Shift+L fills the first match.")).toBeTruthy();
  });

  it("hides the shortcut hint when no key is set", async () => {
    renderPopup(baseState({ matches: [match()] }), { env: fakeEnv({ fillShortcut: () => Promise.resolve(null) }) });
    await screen.findByRole("heading", { name: "For github.com" });
    expect(screen.queryByText(/fills the first match/)).toBeNull();
  });

  it("says plainly that lookalikes get no match when there is nothing", async () => {
    renderPopup(baseState());
    expect(await screen.findByText("No saved login for github.com.")).toBeTruthy();
    expect(screen.getByText(/lookalike sites never get a match/)).toBeTruthy();
  });

  it("fills the chosen item, never confirming a frame at the top level, and closes", async () => {
    const user = userEvent.setup();
    const { port, env } = renderPopup(baseState({ matches: [match()] }));
    await user.click(await screen.findByRole("button", { name: "Fill alex-dev" }));
    expect(requestsOf(port, "fill")).toEqual([{ op: "fill", vaultId: "v1", itemId: "i1", confirmFrame: null }]);
    await waitFor(() => {
      expect(env.closed).toHaveBeenCalled();
    });
    expect(screen.getByRole("status").textContent).toContain("Filled.");
  });

  it("explains a refusal in words, not a code", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match()] }), {
      replies: {
        fill: () => {
          throw new Error("originChanged");
        },
      },
    });
    await user.click(await screen.findByRole("button", { name: "Fill alex-dev" }));
    expect((await screen.findByRole("status")).textContent).toBe(
      "The page changed while Mosaic was filling. Nothing was filled. Try again.",
    );
  });

  it("will not fill a login that the unencrypted page may not use", async () => {
    renderPopup(baseState({ matches: [match({ blocked: "insecure" })], warnings: [{ kind: "insecure" }] }));
    const fill = await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(fill).toHaveProperty("disabled", true);
    expect(fill.getAttribute("aria-describedby")).toBeTruthy();
    expect(screen.getByText("This page isn't encrypted")).toBeTruthy();
  });
});

describe("copy", () => {
  it("copies from the menu and says how long the clipboard keeps it", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ matches: [match({ hasOtp: true })] }), {
      replies: { copy: () => ({ clearAfterSeconds: 30 }) },
    });
    await user.click(await screen.findByRole("button", { name: "Copy options for alex-dev" }));
    expect(screen.getByRole("menu")).toBeTruthy();
    await user.click(screen.getByRole("menuitem", { name: /Copy password/ }));
    expect(requestsOf(port, "copy")).toEqual([
      { op: "copy", source: { kind: "item", vaultId: "v1", itemId: "i1", field: "password" } },
    ]);
    expect((await screen.findByRole("status")).textContent).toBe("Copied. Clears in 30 seconds.");
    // The menu closed and gave focus back to its button.
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Copy options for alex-dev" }));
  });

  it("offers the code only for an item that has one", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match()] }));
    await user.click(await screen.findByRole("button", { name: "Copy options for alex-dev" }));
    expect(screen.queryByRole("menuitem", { name: /Copy code/ })).toBeNull();
  });

  it("moves with the arrow keys and closes on Escape", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match({ hasOtp: true })] }));
    const trigger = await screen.findByRole("button", { name: "Copy options for alex-dev" });
    await user.click(trigger);
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: /Copy user name/ }));
    await user.keyboard("{ArrowDown}");
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: /Copy password/ }));
    await user.keyboard("{End}");
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: /Copy code/ }));
    await user.keyboard("{Escape}");
    expect(screen.queryByRole("menu")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });

  it("copies with Ctrl+C, Ctrl+Shift+C and Ctrl+Alt+C for the first match, and leaves a text box alone", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ matches: [match({ hasOtp: true })] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    await user.keyboard("{Control>}c{/Control}");
    await user.keyboard("{Control>}{Shift>}c{/Shift}{/Control}");
    await user.keyboard("{Control>}{Alt>}c{/Alt}{/Control}");
    expect(requestsOf(port, "copy").map((request) => request.source)).toEqual([
      { kind: "item", vaultId: "v1", itemId: "i1", field: "username" },
      { kind: "item", vaultId: "v1", itemId: "i1", field: "password" },
      { kind: "item", vaultId: "v1", itemId: "i1", field: "totp" },
    ]);
    await user.click(screen.getByRole("searchbox"));
    await user.keyboard("{Control>}c{/Control}");
    expect(requestsOf(port, "copy")).toHaveLength(3);
  });

  it("fills with Enter on the page, not inside the search box", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ matches: [match()] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    await user.keyboard("{Enter}");
    expect(requestsOf(port, "fill")).toHaveLength(1);
    await user.click(screen.getByRole("searchbox"));
    await user.keyboard("{Enter}");
    expect(requestsOf(port, "fill")).toHaveLength(1);
  });
});

describe("frames", () => {
  const inFrame = baseState({ frame: "sameOriginFrame", frameTarget: FRAME, matches: [match()] });

  it("asks before filling a same-origin frame and sends the frame it showed", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(inFrame);
    await user.click(await screen.findByRole("button", { name: "Fill alex-dev" }));
    expect(requestsOf(port, "fill")).toEqual([]);
    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("Fill inside an embedded frame?")).toBeTruthy();
    // Cancel has the focus, so Enter does not confirm by accident.
    expect(document.activeElement).toBe(within(dialog).getByRole("button", { name: "Cancel" }));
    await user.click(within(dialog).getByRole("button", { name: "Fill in the frame" }));
    expect(requestsOf(port, "fill")).toEqual([{ op: "fill", vaultId: "v1", itemId: "i1", confirmFrame: FRAME }]);
  });

  it("fills nothing when the dialog is cancelled", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(inFrame);
    await user.click(await screen.findByRole("button", { name: "Fill alex-dev" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Cancel" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(requestsOf(port, "fill")).toEqual([]);
  });

  it("keeps the frame it showed when the dialog opened, even if the state refreshes behind it", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(inFrame);
    await user.click(await screen.findByRole("button", { name: "Fill alex-dev" }));
    const swapped = { frameId: 9, documentId: "doc-evil" };
    port.replies.getState = () => ({ ...inFrame, frameTarget: swapped });
    port.push({ event: "connection", connection: "unlocked" });
    await waitFor(() => {
      expect(requestsOf(port, "getState").length).toBeGreaterThan(1);
    });
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Fill in the frame" }));
    expect(requestsOf(port, "fill")[0]?.confirmFrame).toEqual(FRAME);
  });

  it("never offers a frame of another site", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ frame: "crossOriginFrame", frameTarget: FRAME, matches: [match()] }));
    const fill = await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(fill).toHaveProperty("disabled", true);
    await user.click(fill);
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(requestsOf(port, "fill")).toEqual([]);
    expect(screen.getByText(/frame from another site/)).toBeTruthy();
  });
});

describe("address warnings", () => {
  it("shows the Unicode and the ASCII form of an international address, with no way to fill anyway", async () => {
    renderPopup(
      baseState({
        tab: { origin: "https://xn--bcher-kva.example", host: "xn--bcher-kva.example", unicodeHost: "bücher.example", secure: true, supported: true },
        warnings: [{ kind: "idn", unicodeHost: "bücher.example" }],
      }),
    );
    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText("Check the address")).toBeTruthy();
    expect(within(alert).getByText("bücher.example")).toBeTruthy();
    expect(within(alert).getByText("xn--bcher-kva.example")).toBeTruthy();
    expect(screen.queryByText(/anyway/i)).toBeNull();
  });

  it("names the saved address when the page looks like it", async () => {
    renderPopup(
      baseState({
        tab: { origin: "https://githuh.com", host: "githuh.com", unicodeHost: null, secure: true, supported: true },
        warnings: [{ kind: "lookalike", savedHost: "github.com" }],
      }),
    );
    const alert = await screen.findByRole("alert");
    expect(within(alert).getByText("This site isn't github.com. It looks similar, which is a common phishing trick.")).toBeTruthy();
    expect(within(alert).getByText("githuh.com")).toBeTruthy();
    expect(screen.queryByRole("button", { name: /fill anyway/i })).toBeNull();
  });
});

describe("one-time codes", () => {
  const next = { vaultId: "v1", itemId: "i1", title: "GitHub", username: "alex-dev" };

  it("offers to fill or copy the code, and never shows it", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ nextOtp: next }));
    expect(await screen.findByRole("heading", { name: "Fill your one-time code" })).toBeTruthy();
    expect(screen.getByText("Password filled for alex-dev. Next: the two-factor code.")).toBeTruthy();
    expect(document.body.textContent).not.toMatch(/\d{3}\s?\d{3}/);
    await user.click(screen.getByRole("button", { name: "Fill code" }));
    expect(requestsOf(port, "fillOtp")).toEqual([{ op: "fillOtp", vaultId: "v1", itemId: "i1", confirmFrame: null }]);
    await user.click(screen.getByRole("button", { name: "Copy code" }));
    expect(requestsOf(port, "copy")[0]?.source).toEqual({ kind: "item", vaultId: "v1", itemId: "i1", field: "totp" });
  });

  it("asks to confirm a frame for the code too", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ nextOtp: next, frame: "sameOriginFrame", frameTarget: FRAME }));
    await user.click(await screen.findByRole("button", { name: "Fill code" }));
    await user.click(within(screen.getByRole("dialog")).getByRole("button", { name: "Fill in the frame" }));
    expect(requestsOf(port, "fillOtp")[0]?.confirmFrame).toEqual(FRAME);
  });
});

describe("search", () => {
  const hit = { vaultId: "v2", itemId: "s1", title: "Netlify", username: "alex", host: "netlify.com", hasOtp: false, favorite: false };

  it("lists hits, filling only those that fit the page", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState(), {
      replies: { search: () => ({ items: [{ ...hit, fillable: false }, { ...hit, itemId: "s2", title: "Local", username: "bob", fillable: true }] }) },
    });
    await user.type(await screen.findByRole("searchbox"), "net");
    expect(await screen.findByText("Netlify")).toBeTruthy();
    expect(requestsOf(port, "search")).toEqual([{ op: "search", query: "net" }]);
    expect(screen.queryByRole("button", { name: "Fill alex" })).toBeNull();
    expect(screen.getByRole("button", { name: "Fill bob" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Copy options for alex" })).toBeTruthy();
  });

  it("says when nothing was found", async () => {
    const user = userEvent.setup();
    renderPopup(baseState(), { replies: { search: () => ({ items: [] }) } });
    await user.type(await screen.findByRole("searchbox"), "zzz");
    expect(await screen.findByText("Nothing found.")).toBeTruthy();
  });
});

describe("a failed shortcut fill", () => {
  it("is shown once, in words", async () => {
    renderPopup(baseState({ lastError: "frameNeedsConfirmation" }));
    expect((await screen.findByRole("alert")).textContent).toBe(
      "The field is inside a frame. Open the Mosaic popup and confirm the frame to fill it.",
    );
  });
});

describe("pages Mosaic cannot fill", () => {
  it("still lets the user search and copy", async () => {
    renderPopup(baseState({ tab: null, frame: "none", frameTarget: null }));
    expect(await screen.findByText(/can't fill on this page/)).toBeTruthy();
    expect(screen.getByRole("searchbox")).toBeTruthy();
  });
});
