import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { CommandName } from "../ipc/contract/index.ts";
import { createMockIpc, type MockOptions } from "../ipc/mock.ts";

interface Sent {
  command: CommandName;
  request: unknown;
}

/** The mock, recording every command that goes through it. */
function recording(options: MockOptions): { ipc: MosaicIpc; sent: Sent[] } {
  const base = createMockIpc(options);
  const sent: Sent[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => base.subscribe(listener),
    call: (command, request) => {
      sent.push({ command, request });
      return base.call(command, request);
    },
  };
  return { ipc, sent };
}

const callsOf = (sent: Sent[], command: CommandName) => sent.filter((each) => each.command === command);

async function openSection(options: MockOptions, locale: "en" | "de" = "en") {
  const { ipc, sent } = recording(options);
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={[locale]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: locale === "en" ? "Settings" : "Einstellungen" }));
  const heading = await screen.findByRole("heading", {
    name: locale === "en" ? "Fill in programs" : "In Programmen ausfüllen",
  });
  return { user, sent, ipc, section: heading.closest("section") as HTMLElement };
}

describe("program fill settings", () => {
  it("turns program fill on without a password and disables submit while off", async () => {
    const { user, sent, section } = await openSection({ start: "unlocked" });
    const enable = within(section).getByRole("switch", { name: "Offer logins in programs" });
    const submit = within(section).getByRole("switch", { name: "Sign in after filling" });
    expect((enable as HTMLInputElement).checked).toBe(false);
    expect((submit as HTMLInputElement).disabled).toBe(true);
    await user.click(enable);
    await waitFor(() => {
      expect(within(section).getByRole<HTMLInputElement>("switch", { name: "Sign in after filling" }).disabled).toBe(false);
    });
    const update = callsOf(sent, "update_settings").at(-1)?.request as {
      settings: { programFill: boolean; programFillSubmit: boolean };
      password: string | null;
    };
    expect(update.settings.programFill).toBe(true);
    expect(update.password).toBeNull();
    // The Enter switch is its own setting.
    await user.click(within(section).getByRole("switch", { name: "Sign in after filling" }));
    await waitFor(() => {
      const last = callsOf(sent, "update_settings").at(-1)?.request as { settings: { programFillSubmit: boolean } };
      expect(last.settings.programFillSubmit).toBe(false);
    });
  });

  it("shows only the reason when program fill is unavailable", async () => {
    for (const [variant, text] of [
      ["notWindows", "Only on Windows for now."],
      ["mosaicElevated", "Mosaic runs as administrator, so it doesn't fill programs. Start it normally."],
      ["hostFailed", "Mosaic couldn't start watching programs. Restart Mosaic to try again."],
    ] as const) {
      const { section } = await openSection({ start: "unlocked", programFill: variant });
      expect(within(section).getByText(text)).toBeTruthy();
      expect(within(section).queryByRole("switch")).toBeNull();
      cleanup();
    }
  });

  it("lists blocked programs and offers them again", async () => {
    const { user, sent, section } = await openSection({ start: "unlocked", programFill: "blocked" });
    expect(within(section).getByText("Notepad")).toBeTruthy();
    expect(within(section).getByText("legacy.exe")).toBeTruthy();
    await user.click(within(section).getByRole("button", { name: "Offer again: Notepad" }));
    await waitFor(() => {
      expect(within(section).queryByText("Notepad")).toBeNull();
    });
    expect(callsOf(sent, "program_fill_unblock")).toEqual([
      { command: "program_fill_unblock", request: { key: "signed:notepad.exe:AAAA" } },
    ]);
    expect(within(section).getByText("legacy.exe")).toBeTruthy();
  });

  it("says None when nothing is blocked, in German too", async () => {
    const { section } = await openSection({ start: "unlocked" }, "de");
    expect(within(section).getByText("Keine")).toBeTruthy();
    expect(within(section).getByRole("switch", { name: "Logins in Programmen anbieten" })).toBeTruthy();
  });

  it("records a hotkey and saves it, or removes it", async () => {
    const { user, sent, section } = await openSection({ start: "unlocked" });
    const field = within(section).getByLabelText("Shortcut to open the list");
    expect((field as HTMLInputElement).value).toBe("No shortcut");
    await user.click(field);
    await user.keyboard("{Control>}{Alt>}j{/Alt}{/Control}");
    await user.click(within(section).getByRole("button", { name: "Save list shortcut" }));
    await waitFor(() => {
      const last = callsOf(sent, "update_settings").at(-1)?.request as { settings: { programFillHotkey: string | null } };
      expect(last.settings.programFillHotkey).toBe("Ctrl+Alt+J");
    });
    await user.click(within(section).getByRole("button", { name: "No shortcut" }));
    await waitFor(() => {
      const last = callsOf(sent, "update_settings").at(-1)?.request as { settings: { programFillHotkey: string | null } };
      expect(last.settings.programFillHotkey).toBeNull();
    });
  });
});

describe("the program fill card", () => {
  it("shows the card once and remembers Not now", async () => {
    const { ipc, sent } = recording({ start: "unlocked" });
    const user = userEvent.setup();
    const first = render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    expect(await screen.findByText("Fill logins in programs too")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Not now" }));
    await waitFor(() => {
      expect(screen.queryByText("Fill logins in programs too")).toBeNull();
    });
    const sentSettings = callsOf(sent, "update_settings").at(-1)?.request as {
      settings: { programFill: boolean; programFillCardDismissed: boolean };
    };
    expect(sentSettings.settings.programFill).toBe(false);
    expect(sentSettings.settings.programFillCardDismissed).toBe(true);
    first.unmount();
    // Another start with the same settings: no card.
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    expect(screen.queryByText("Fill logins in programs too")).toBeNull();
  });

  it("turns program fill on from the card", async () => {
    const { ipc, sent } = recording({ start: "unlocked" });
    const user = userEvent.setup();
    render(<App ipc={ipc} languages={["en"]} />);
    await user.click(await screen.findByRole("button", { name: "Turn on" }));
    await waitFor(() => {
      const last = callsOf(sent, "update_settings").at(-1)?.request as {
        settings: { programFill: boolean; programFillCardDismissed: boolean };
        password: string | null;
      };
      expect(last.settings.programFill).toBe(true);
      expect(last.settings.programFillCardDismissed).toBe(true);
      expect(last.password).toBeNull();
    });
    expect(screen.queryByText("Fill logins in programs too")).toBeNull();
  });

  it("does not show the card where program fill is unavailable", async () => {
    const { ipc } = recording({ start: "unlocked", programFill: "notWindows" });
    render(<App ipc={ipc} languages={["en"]} />);
    await screen.findByText("GitHub");
    expect(screen.queryByText("Fill logins in programs too")).toBeNull();
  });
});
