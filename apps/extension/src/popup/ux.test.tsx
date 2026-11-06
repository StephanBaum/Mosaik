/**
 * The UX review follow-ups: a generated password is in no live region, the status bar is in the
 * layout flow, one primary button per screen, the no-code announcement, headings, the pairing
 * code's name, the shortcut hints per platform, copy-only rows and the developer-only text.
 */
import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { baseState, fakeEnv, match, renderPopup, requestsOf } from "./test/harness.tsx";

const PASSWORD = "t7#Vq9!mR2xL$wp4Kz";
const generated = { value: PASSWORD, entropyBits: 118, strength: "strong", rulesSource: null };
const pendingSave = {
  host: "fastmail.com",
  unicodeHost: null,
  username: "alex@fastmail.example",
  kind: "new" as const,
  itemTitle: null,
  source: "capture" as const,
  insecure: false,
};
const nextOtp = { vaultId: "v1", itemId: "i1", title: "GitHub", username: "alex-dev" };

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the generated password (M2)", () => {
  it("is in no live region, so a screen reader never announces it", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ generatorFill: true }), { replies: { generate: () => generated } });
    await user.click(await screen.findByRole("button", { name: "Password generator" }));
    const value = await screen.findByRole("group", { name: "Generated password" });
    await waitFor(() => {
      expect(value.textContent).toBe(PASSWORD);
    });
    expect(value.hasAttribute("data-secret")).toBe(true);
    expect(value.tagName).not.toBe("OUTPUT");
    const live = document.querySelectorAll("output, [role='status'], [role='alert'], [role='log'], [aria-live]");
    for (const region of live) {
      expect(region.contains(value), region.outerHTML).toBe(false);
      expect(region.textContent).not.toContain(PASSWORD);
    }
    expect(value.closest("[aria-live], [role='status'], [role='alert'], output")).toBeNull();
  });
});

describe("the status bar (M3)", () => {
  it("sits in the layout flow between the page and the footer, as the polite live region", async () => {
    renderPopup(baseState({ matches: [match()] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    const bar = document.querySelector(".status-bar");
    expect(bar?.getAttribute("role")).toBe("status");
    expect(bar?.getAttribute("aria-live")).toBe("polite");
    expect(bar?.previousElementSibling?.tagName).toBe("MAIN");
    expect(bar?.nextElementSibling?.tagName).toBe("FOOTER");
    expect(bar?.parentElement?.className).toBe("popup");
  });

  it("shows an announcement in that bar", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match()] }), { replies: { copy: () => ({ clearAfterSeconds: 30 }) } });
    await user.click(await screen.findByRole("button", { name: "Copy options for alex-dev" }));
    await user.click(screen.getByRole("menuitem", { name: /Copy password/ }));
    await waitFor(() => {
      expect(document.querySelector(".status-bar")?.textContent).toBe("Copied. Clears in 30 seconds.");
    });
  });
});

describe("one primary button per screen (m4)", () => {
  const primaries = (): string[] =>
    Array.from(document.querySelectorAll<HTMLElement>(".button--primary")).map((button) => button.textContent);

  it("is the first row's Fill when nothing else asks for an answer", async () => {
    renderPopup(baseState({ matches: [match(), match({ itemId: "i2", username: "alex-work" })] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(primaries()).toEqual(["Fill"]);
  });

  it("is the save card's button, not the row's, while a save waits", async () => {
    renderPopup(baseState({ pendingSave, matches: [match()] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(primaries()).toEqual(["Save in Mosaic…"]);
  });

  it("is Fill code, not the row's, while a code is next", async () => {
    renderPopup(baseState({ nextOtp, matches: [match()] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    expect(primaries()).toEqual(["Fill code"]);
  });
});

describe("the no-code shortcut (m5)", () => {
  it("says there is no one-time code instead of doing nothing", async () => {
    const user = userEvent.setup();
    const { port } = renderPopup(baseState({ matches: [match({ hasOtp: false })] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    await user.keyboard("{Control>}{Alt>}c{/Alt}{/Control}");
    await waitFor(() => {
      expect(document.querySelector(".status-bar")?.textContent).toBe("This item has no one-time code.");
    });
    expect(requestsOf(port, "copy")).toEqual([]);
  });
});

describe("headings (m6)", () => {
  it("has an h1 while checking the connection", async () => {
    renderPopup(baseState({ connection: "checking" }));
    expect(await screen.findByRole("heading", { level: 1, name: "Mosaic" })).toBeTruthy();
  });

  it("has an h1 while loading", () => {
    renderPopup(baseState());
    expect(screen.getByRole("heading", { level: 1, name: "Loading…" })).toBeTruthy();
  });

  it("has an h1 on a page Mosaic cannot fill", async () => {
    renderPopup(baseState({ tab: null, frame: "none", frameTarget: null }));
    expect(await screen.findByRole("heading", { level: 1, name: /can't fill on this page/ })).toBeTruthy();
  });

  it("puts the h1 before the save and code cards, which are h2", async () => {
    renderPopup(baseState({ pendingSave, nextOtp, matches: [match()] }));
    await screen.findByRole("button", { name: "Fill alex-dev" });
    const headings = screen.getAllByRole("heading");
    expect(headings[0]?.tagName).toBe("H1");
    expect(headings.filter((heading) => heading.tagName === "H1")).toHaveLength(1);
    for (const name of ["Save this login for fastmail.com?", "Fill your one-time code", "For github.com"]) {
      expect(screen.getByRole("heading", { level: 2, name })).toBeTruthy();
    }
  });
});

describe("the pairing code (m7)", () => {
  it("is an image named with the spaced characters", async () => {
    renderPopup(baseState({ connection: "pairing", pairCode: { code: "4F7K2Q", expiresInSeconds: 120 } }));
    const code = await screen.findByRole("img", { name: "4 F 7 - K 2 Q" });
    expect(code.textContent).toBe("4F7-K2Q");
  });
});

describe("the developer text (m8)", () => {
  it("shows the helper command only when the build gives one", async () => {
    renderPopup(baseState({ connection: "hostMissing" }), { env: fakeEnv({ hostInstallCommand: null }) });
    await screen.findByRole("heading", { name: "Mosaic isn't set up for this browser" });
    expect(screen.queryByText(/Developers/)).toBeNull();
    expect(document.body.textContent).not.toContain("host:install");
  });
});

describe("the copy menu (m1) and rows (n3, n8)", () => {
  it("names the shortcuts with Ctrl on a PC", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match({ hasOtp: true })] }));
    await user.click(await screen.findByRole("button", { name: "Copy options for alex-dev" }));
    const menu = screen.getByRole("menu");
    expect(within(menu).getAllByText(/Ctrl/).map((hint) => hint.textContent)).toEqual([
      "Ctrl+C",
      "Ctrl+Shift+C",
      "Ctrl+Alt+C",
    ]);
  });

  it("names them with the Command symbols on a Mac", async () => {
    vi.spyOn(window.navigator, "platform", "get").mockReturnValue("MacIntel");
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match({ hasOtp: true })] }));
    await user.click(await screen.findByRole("button", { name: "Copy options for alex-dev" }));
    const hints = Array.from(screen.getByRole("menu").querySelectorAll("kbd")).map((hint) => hint.textContent);
    expect(hints).toEqual(["⌘C", "⇧⌘C", "⌥⌘C"]);
  });

  it("names them in German with Strg", async () => {
    const user = userEvent.setup();
    renderPopup(baseState({ matches: [match()] }), { languages: ["de"] });
    await user.click(await screen.findByRole("button", { name: "Kopier-Optionen für alex-dev" }));
    const hints = Array.from(screen.getByRole("menu").querySelectorAll("kbd")).map((hint) => hint.textContent);
    expect(hints).toEqual(["Strg+C", "Strg+Umschalt+C"]);
  });

  it("marks a search hit that cannot be filled here as copy-only, and gives titles a tooltip", async () => {
    const user = userEvent.setup();
    const hit = { vaultId: "v2", itemId: "s1", title: "Netlify", username: "alex", host: "netlify.com", hasOtp: false, favorite: false };
    renderPopup(baseState(), { replies: { search: () => ({ items: [{ ...hit, fillable: false }] }) } });
    await user.type(await screen.findByRole("searchbox"), "net");
    expect(await screen.findByText("Copy only: not saved for this site")).toBeTruthy();
    expect(screen.getByText("Netlify").getAttribute("title")).toBe("Netlify");
  });

  it("lets a click anywhere in the search box focus it", async () => {
    renderPopup(baseState());
    const input = await screen.findByRole("searchbox");
    expect(input.closest("label")?.className).toBe("search__box");
  });
});
