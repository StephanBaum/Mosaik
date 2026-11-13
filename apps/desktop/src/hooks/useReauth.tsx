/**
 * The re-authentication pattern (UX_GUIDELINES §8) for commands that answer
 * `reauthRequired` (adding a key to a hardware vault, moving an item out of one): send the
 * command without a password; on `reauthRequired` ask for the master password, call
 * `reauthenticate`, then send the command again.
 */
import { useRef, useState, type ReactNode } from "react";
import type { MessageKey } from "@mosaic/i18n";
import { ReauthDialog } from "../components/ReauthDialog.tsx";
import { useIpc } from "../ipc/IpcProvider.tsx";
import type { IpcResult, ReauthPurpose } from "../ipc/contract/index.ts";

/** What `guard` resolves to when the user closes the dialog without a password. */
export const CANCELLED = { ok: false, error: { code: "cancelled" } } as const satisfies IpcResult<never>;

interface Request {
  run: () => Promise<IpcResult<unknown>>;
  resolve: (result: IpcResult<never>) => void;
  /** The password was right and the command is running again. */
  retrying: boolean;
}

export function useReauth(
  body: MessageKey,
  purpose: ReauthPurpose,
): {
  /** Runs `run`; the result is the command's, or `CANCELLED` if the user gave up. */
  guard: <T>(run: () => Promise<IpcResult<T>>) => Promise<IpcResult<T>>;
  /** Render this once, anywhere in the component. */
  dialog: ReactNode;
} {
  const ipc = useIpc();
  const [open, setOpen] = useState(false);
  const request = useRef<Request | null>(null);

  function guard<T>(run: () => Promise<IpcResult<T>>): Promise<IpcResult<T>> {
    return run().then(
      (result) =>
        new Promise<IpcResult<T>>((resolve) => {
          if (result.ok || result.error.code !== "reauthRequired") {
            resolve(result);
            return;
          }
          request.current = { run, resolve, retrying: false };
          setOpen(true);
        }),
    );
  }

  const dialog = open ? (
    <ReauthDialog
      body={body}
      onSubmit={async (password) => {
        const current = request.current;
        const result = await ipc.call("reauthenticate", { password, purpose });
        if (!result.ok) {
          return result.error;
        }
        if (current !== null) {
          current.retrying = true;
          // The dialog closes now; the caller keeps waiting (a key touch) with its own status.
          void current.run().then(current.resolve as (result: IpcResult<unknown>) => void);
        }
        return null;
      }}
      onClose={() => {
        const current = request.current;
        request.current = null;
        setOpen(false);
        if (current !== null && !current.retrying) {
          current.resolve(CANCELLED);
        }
      }}
    />
  ) : null;

  return { guard, dialog };
}
