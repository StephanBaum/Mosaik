import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { IpcProvider } from "../ipc/IpcProvider.tsx";
import { createMockIpc } from "../ipc/mock.ts";
import { PlatformContext, type Platform } from "../platform/platform.ts";
import { CommandPalette } from "./CommandPalette.tsx";

function renderPalette(platform: Platform = "windows") {
  const onClose = vi.fn();
  render(
    <PlatformContext value={platform}>
      <I18nProvider locale="en">
        <IpcProvider ipc={createMockIpc({ start: "unlocked" })}>
          <CommandPalette
            onClose={onClose}
            onOpenItem={vi.fn()}
            onNavigate={vi.fn()}
            onNewItem={vi.fn()}
            onLock={vi.fn()}
          />
        </IpcProvider>
      </I18nProvider>
    </PlatformContext>,
  );
  return { onClose };
}

describe("command palette", () => {
  it("does not point aria-controls at a list that is not rendered", async () => {
    const user = userEvent.setup();
    renderPalette();
    const box = screen.getByRole("combobox");
    await user.type(box, "zzzz");
    await screen.findByText("Nothing matches. Try another word.");
    expect(box.getAttribute("aria-expanded")).toBe("false");
    expect(box.hasAttribute("aria-controls")).toBe(false);
  });

  it("shows the shortcut of a command", async () => {
    renderPalette("windows");
    const row = await screen.findByRole("option", { name: /New item/ });
    expect(row.textContent).toContain("Ctrl+N");
    expect(row.getAttribute("aria-keyshortcuts")).toBe("Control+N");
  });

  it("closes with one Esc, whatever the query", async () => {
    const user = userEvent.setup();
    const { onClose } = renderPalette();
    await user.type(screen.getByRole("combobox"), "git");
    await user.keyboard("{Escape}");
    expect(onClose).toHaveBeenCalledTimes(1);
  });
});
