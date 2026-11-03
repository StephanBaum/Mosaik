import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { Dialog } from "./Dialog.tsx";

function Host() {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button
        type="button"
        onClick={() => {
          setOpen(true);
        }}
      >
        Open
      </button>
      {open && (
        <Dialog
          title="Sure?"
          onClose={() => {
            setOpen(false);
          }}
        >
          <button
            type="button"
            onClick={() => {
              setOpen(false);
            }}
          >
            Inside
          </button>
        </Dialog>
      )}
    </>
  );
}

describe("Dialog", () => {
  it("gives focus back to the control that opened it", async () => {
    const user = userEvent.setup();
    render(
      <I18nProvider locale="en">
        <Host />
      </I18nProvider>,
    );
    const opener = screen.getByRole("button", { name: "Open" });
    await user.click(opener);
    await screen.findByRole("dialog", { name: "Sure?" });
    await user.click(screen.getByRole("button", { name: "Inside" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(document.activeElement).toBe(opener);
  });
});
