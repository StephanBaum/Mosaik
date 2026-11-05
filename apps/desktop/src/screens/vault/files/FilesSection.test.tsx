import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { SecretsProvider } from "../../../app/SecretsContext.tsx";
import { ToastProvider } from "../../../components/ToastProvider.tsx";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import type { ItemDetail } from "../../../ipc/contract/index.ts";
import { IpcProvider } from "../../../ipc/IpcProvider.tsx";
import { createMockIpc, type MockOptions } from "../../../ipc/mock.ts";
import { FilesSection } from "./FilesSection.tsx";

const ITEM = {
  vaultId: "019a2b3c-4d5e-7010-8000-000000000001",
  itemId: "019a2b3c-4d5e-7020-8000-000000000008",
};

function Host({ options, editable = true }: { options?: MockOptions; editable?: boolean }) {
  const [ipc] = useState(() => createMockIpc({ start: "unlocked", ...options }));
  const [detail, setDetail] = useState<ItemDetail | null>(null);
  const [asked, setAsked] = useState(false);
  if (!asked) {
    setAsked(true);
    void ipc.call("get_item", ITEM).then((result) => {
      if (result.ok) {
        setDetail(result.value);
      }
    });
  }
  return (
    <I18nProvider locale="en">
      <IpcProvider ipc={ipc}>
        <SecretsProvider>
          <ToastProvider>
            {detail !== null && (
              <FilesSection
                detail={detail}
                item={ITEM}
                editable={editable}
                autoFocusAdd={false}
                onChanged={setDetail}
              />
            )}
          </ToastProvider>
        </SecretsProvider>
      </IpcProvider>
    </I18nProvider>
  );
}

async function section() {
  return within(await screen.findByRole("region", { name: "Files" }));
}

describe("FilesSection", () => {
  it("lists the files with size, date and the actions of each", async () => {
    render(<Host />);
    const files = await section();
    const rows = files.getAllByRole("listitem");
    expect(rows).toHaveLength(2);
    const [png, pdf] = rows;
    expect(within(png as HTMLElement).getByText("passport.png")).toBeTruthy();
    expect((png as HTMLElement).textContent).toContain("2.3 MB");
    expect((png as HTMLElement).textContent).toContain("Added");
    expect(within(png as HTMLElement).getByRole("button", { name: "Preview passport.png" })).toBeTruthy();
    expect(within(pdf as HTMLElement).queryByRole("button", { name: /Preview/ })).toBeNull();
    expect(within(pdf as HTMLElement).getByRole("button", { name: "Save contract.pdf as…" })).toBeTruthy();
    expect(within(pdf as HTMLElement).getByRole("button", { name: "Remove contract.pdf" })).toBeTruthy();
    expect(files.getByText(/Preview works for PNG and JPEG images/)).toBeTruthy();
  });

  it("hides Add and Remove when the item is not editable", async () => {
    render(<Host editable={false} />);
    const files = await section();
    expect(files.queryByRole("button", { name: "Add file…" })).toBeNull();
    expect(files.queryByRole("button", { name: /^Remove/ })).toBeNull();
    expect(files.getByRole("button", { name: "Preview passport.png" })).toBeTruthy();
  });

  it("asks before removing, with Keep file focused, and removes on confirm", async () => {
    const user = userEvent.setup();
    render(<Host />);
    const files = await section();
    await user.click(files.getByRole("button", { name: "Remove contract.pdf" }));
    const dialog = within(await screen.findByRole("dialog", { name: "Remove contract.pdf?" }));
    await waitFor(() => {
      expect(document.activeElement).toBe(dialog.getByRole("button", { name: "Keep file" }));
    });
    await user.click(dialog.getByRole("button", { name: "Keep file" }));
    expect(files.getAllByRole("listitem")).toHaveLength(2);

    await user.click(files.getByRole("button", { name: "Remove contract.pdf" }));
    await user.click(await screen.findByRole("button", { name: "Remove file" }));
    await waitFor(() => {
      expect(files.getAllByRole("listitem")).toHaveLength(1);
    });
    expect(await screen.findByText("File removed.")).toBeTruthy();
  });

  it("adds a file and says so; a cancelled dialog is silent", async () => {
    const user = userEvent.setup();
    render(<Host />);
    const files = await section();
    await user.click(files.getByRole("button", { name: "Add file…" }));
    expect(await screen.findByText("File added.")).toBeTruthy();
    expect(files.getAllByRole("listitem")).toHaveLength(3);
  });

  it("stays quiet when the file dialog is cancelled and shows an error otherwise", async () => {
    const user = userEvent.setup();
    const { unmount } = render(<Host options={{ attachAdd: "cancel" }} />);
    let files = await section();
    await user.click(files.getByRole("button", { name: "Add file…" }));
    await waitFor(() => {
      expect(files.getByRole("button", { name: "Add file…" }).getAttribute("aria-busy")).toBe("false");
    });
    expect(screen.queryByText("File added.")).toBeNull();
    expect(files.getAllByRole("listitem")).toHaveLength(2);
    unmount();

    render(<Host options={{ attachAdd: "attachmentTooLarge" }} />);
    files = await section();
    await user.click(files.getByRole("button", { name: "Add file…" }));
    expect(await screen.findByText(/larger than 50 MiB/)).toBeTruthy();
  });

  it("saves a file and says so", async () => {
    const user = userEvent.setup();
    render(<Host />);
    const files = await section();
    await user.click(files.getByRole("button", { name: "Save contract.pdf as…" }));
    expect(await screen.findByText("File saved.")).toBeTruthy();
  });

  it("opens the preview of an image", async () => {
    const user = userEvent.setup();
    render(<Host />);
    const files = await section();
    await user.click(files.getByRole("button", { name: "Preview passport.png" }));
    const dialog = await screen.findByRole("dialog", { name: "passport.png" });
    expect(dialog.querySelector("[data-secret] canvas")).not.toBeNull();
  });

  it("marks a file that is not on this device and disables Save as", async () => {
    const user = userEvent.setup();
    const ipc = createMockIpc({ start: "unlocked" });
    const result = await ipc.call("get_item", ITEM);
    if (!result.ok) {
      throw new Error("get_item failed");
    }
    const detail: ItemDetail = {
      ...result.value,
      attachments: result.value.attachments.map((entry) => ({
        ...entry,
        available: false,
        previewable: false,
      })),
    };
    render(
      <I18nProvider locale="en">
        <IpcProvider ipc={ipc}>
          <SecretsProvider>
            <ToastProvider>
              <FilesSection detail={detail} item={ITEM} editable autoFocusAdd={false} onChanged={() => undefined} />
            </ToastProvider>
          </SecretsProvider>
        </IpcProvider>
      </I18nProvider>,
    );
    const files = await section();
    expect(files.getAllByText(/isn't on this device yet/)).toHaveLength(2);
    expect(files.getByRole<HTMLButtonElement>("button", { name: "Save passport.png as…" }).disabled).toBe(true);
    expect(files.queryByRole("button", { name: /^Preview/ })).toBeNull();
    await user.click(files.getByRole("button", { name: "Remove passport.png" }));
    expect(await screen.findByRole("dialog")).toBeTruthy();
  });
});
