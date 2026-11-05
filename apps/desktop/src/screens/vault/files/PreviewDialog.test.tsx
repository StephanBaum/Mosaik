import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { SecretsProvider, useSecrets } from "../../../app/SecretsContext.tsx";
import { I18nProvider } from "../../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../../ipc/client.ts";
import type {
  AttachmentEntry,
  CommandName,
  ErrorCode,
  IpcResult,
} from "../../../ipc/contract/index.ts";
import { IpcProvider } from "../../../ipc/IpcProvider.tsx";
import { PreviewDialog } from "./PreviewDialog.tsx";

const ITEM = {
  vaultId: "019a2b3c-4d5e-7010-8000-000000000001",
  itemId: "019a2b3c-4d5e-7020-8000-000000000008",
};
const ATTACHMENT: AttachmentEntry = {
  attachmentId: "019a2b3c-4d5e-7040-8000-000000000001",
  fileName: "passport.png",
  mime: "image/png",
  size: 100,
  addedAt: null,
  available: true,
  previewable: true,
};

function answer(width: number, height: number, fill = 9): ArrayBuffer {
  const buffer = new ArrayBuffer(8 + width * height * 4);
  const view = new DataView(buffer);
  view.setUint32(0, width, true);
  view.setUint32(4, height, true);
  new Uint8Array(buffer, 8).fill(fill);
  return buffer;
}

/** An IPC whose `attachment_preview` answers when the test says so. */
function controlledIpc() {
  const pending: ((result: IpcResult<ArrayBuffer>) => void)[] = [];
  const calls: CommandName[] = [];
  const ipc = {
    call: (command: CommandName) => {
      calls.push(command);
      return new Promise<IpcResult<ArrayBuffer>>((resolve) => {
        pending.push(resolve);
      });
    },
    subscribe: () => Promise.resolve(() => undefined),
  } as unknown as MosaicIpc;
  return {
    ipc,
    calls,
    answerWith: (result: IpcResult<ArrayBuffer>, index = 0) => {
      pending[index]?.(result);
    },
  };
}

const painted = vi.fn();
const cleared = vi.fn();

beforeEach(() => {
  painted.mockReset();
  cleared.mockReset();
  vi.stubGlobal(
    "ImageData",
    class {
      constructor(
        readonly data: Uint8ClampedArray,
        readonly width: number,
        readonly height: number,
      ) {}
    },
  );
  // jsdom reports no focus; the provider would hide every secret at mount.
  vi.spyOn(document, "hasFocus").mockReturnValue(true);
  vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation(
    () => ({ putImageData: painted, clearRect: cleared }) as unknown as CanvasRenderingContext2D,
  );
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function HideAll() {
  const secrets = useSecrets();
  return (
    <button type="button" onClick={secrets.clearAll}>
      Hide secrets
    </button>
  );
}

function Host({ ipc, onSaveAs = () => undefined }: { ipc: MosaicIpc; onSaveAs?: () => void }) {
  const [open, setOpen] = useState(true);
  return (
    <I18nProvider locale="en">
      <IpcProvider ipc={ipc}>
        <SecretsProvider>
          <HideAll />
          {open && (
            <PreviewDialog
              item={ITEM}
              attachment={ATTACHMENT}
              onClose={() => {
                setOpen(false);
              }}
              onSaveAs={onSaveAs}
            />
          )}
        </SecretsProvider>
      </IpcProvider>
    </I18nProvider>
  );
}

describe("PreviewDialog", () => {
  it("paints the pixels through the canvas context and wipes the buffer", async () => {
    const { ipc, answerWith } = controlledIpc();
    render(<Host ipc={ipc} />);
    const buffer = answer(4, 3);
    await act(async () => {
      answerWith({ ok: true, value: buffer });
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(painted).toHaveBeenCalledOnce();
    });
    const canvas = screen.getByRole("img", { name: "Preview of passport.png", hidden: true });
    expect([(canvas as HTMLCanvasElement).width, (canvas as HTMLCanvasElement).height]).toEqual([4, 3]);
    expect(new Uint8Array(buffer).every((byte) => byte === 0)).toBe(true);
    expect(screen.getByRole("dialog", { name: "passport.png" })).toBeTruthy();
  });

  it("marks the frame as a secret without a mask", () => {
    const { ipc } = controlledIpc();
    render(<Host ipc={ipc} />);
    const canvas = screen.getByRole("img", { name: "Preview of passport.png", hidden: true });
    const frame = canvas.parentElement;
    expect(frame?.hasAttribute("data-secret")).toBe(true);
    expect(frame?.hasAttribute("data-mask")).toBe(false);
    expect(canvas.closest("dialog")).not.toBeNull();
  });

  it("wipes and drops an answer that arrives after the dialog closed", async () => {
    const user = userEvent.setup();
    const { ipc, answerWith } = controlledIpc();
    render(<Host ipc={ipc} />);
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(screen.queryByRole("dialog")).toBeNull();
    const late = answer(4, 3, 200);
    await act(async () => {
      answerWith({ ok: true, value: late });
      await Promise.resolve();
    });
    expect(painted).not.toHaveBeenCalled();
    expect(new Uint8Array(late).every((byte) => byte === 0)).toBe(true);
  });

  it("closes when the secrets are hidden (lock, blur)", async () => {
    const user = userEvent.setup();
    const { ipc, answerWith } = controlledIpc();
    render(<Host ipc={ipc} />);
    await act(async () => {
      answerWith({ ok: true, value: answer(2, 2) });
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(painted).toHaveBeenCalledOnce();
    });
    await user.click(screen.getByRole("button", { name: "Hide secrets", hidden: true }));
    expect(screen.queryByRole("dialog")).toBeNull();
    expect(cleared).toHaveBeenCalled();
  });

  it.each<ErrorCode>(["attachmentPreviewUnsupported", "attachmentPreviewTooLarge"])(
    "shows %s as a message",
    async (code) => {
      const { ipc, answerWith } = controlledIpc();
      render(<Host ipc={ipc} />);
      await act(async () => {
        answerWith({ ok: false, error: { code } });
        await Promise.resolve();
      });
      const alert = await screen.findByText(/Use Save as to open it/);
      expect(alert.getAttribute("role")).toBe("alert");
      expect(painted).not.toHaveBeenCalled();
    },
  );

  it("shows a damaged answer as an error and still wipes it", async () => {
    const { ipc, answerWith } = controlledIpc();
    render(<Host ipc={ipc} />);
    const broken = answer(4, 3).slice(0, -1);
    new Uint8Array(broken).fill(5);
    await act(async () => {
      answerWith({ ok: true, value: broken });
      await Promise.resolve();
    });
    await waitFor(() => {
      expect(screen.getByRole("alert").textContent).not.toBe("");
    });
    expect(new Uint8Array(broken).every((byte) => byte === 0)).toBe(true);
    expect(painted).not.toHaveBeenCalled();
  });

  it("asks again when another preview is still running", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    try {
      const { ipc, calls, answerWith } = controlledIpc();
      render(<Host ipc={ipc} />);
      await act(async () => {
        answerWith({ ok: false, error: { code: "busy" } }, 0);
        await vi.advanceTimersByTimeAsync(300);
      });
      expect(calls).toHaveLength(2);
      await act(async () => {
        answerWith({ ok: true, value: answer(1, 1) }, 1);
        await Promise.resolve();
      });
      await waitFor(() => {
        expect(painted).toHaveBeenCalledOnce();
      });
    } finally {
      vi.useRealTimers();
    }
  });

  it("offers Save as, and Esc closes", async () => {
    const user = userEvent.setup();
    const onSaveAs = vi.fn();
    render(<Host ipc={controlledIpc().ipc} onSaveAs={onSaveAs} />);
    await user.click(screen.getByRole("button", { name: "Save as…" }));
    expect(onSaveAs).toHaveBeenCalledOnce();
    // Esc is the dialog's own cancel event; jsdom does not send it for a key press.
    fireEvent(screen.getByRole("dialog"), new Event("cancel", { cancelable: true }));
    expect(screen.queryByRole("dialog")).toBeNull();
  });
});
