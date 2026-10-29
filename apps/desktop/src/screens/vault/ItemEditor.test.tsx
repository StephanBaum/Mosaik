import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ToastProvider } from "../../components/ToastProvider.tsx";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import { IpcProvider } from "../../ipc/IpcProvider.tsx";
import { createMockIpc } from "../../ipc/mock.ts";
import { ItemEditor } from "./ItemEditor.tsx";

function renderEditor() {
  const onCancel = vi.fn();
  render(
    <I18nProvider locale="en">
      <IpcProvider ipc={createMockIpc({ start: "unlocked" })}>
        <ToastProvider>
          <ItemEditor
            vaults={[{ vaultId: "v1", name: "Personal", itemCount: 0, available: true }]}
            vaultId="v1"
            editing={null}
            onSaved={vi.fn()}
            onCancel={onCancel}
          />
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>,
  );
  return { onCancel };
}

describe("item editor", () => {
  it("cancels on Esc when nothing changed", async () => {
    const user = userEvent.setup();
    const { onCancel } = renderEditor();
    await user.click(screen.getByLabelText("Title"));
    await user.keyboard("{Escape}");
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("asks before it drops edits", async () => {
    const user = userEvent.setup();
    const { onCancel } = renderEditor();
    await user.type(screen.getByLabelText("Title"), "Draft");
    await user.keyboard("{Escape}");
    expect(onCancel).not.toHaveBeenCalled();
    const dialog = await screen.findByRole("dialog", { name: "Discard your changes?" });
    expect(dialog).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Discard" }));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it("keeps the notes in a multi-line field", () => {
    renderEditor();
    const notes = screen.getByLabelText("Notes");
    expect(notes.tagName).toBe("TEXTAREA");
  });
});
