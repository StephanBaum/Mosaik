/** Mock clipboard commands: no real clipboard, but the receipt and the cleared event. */
import { findItem } from "./items.ts";
import { mockPendingCode } from "./otp.ts";
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";

export const clipboardHandlers: MockHandlers = {
  copy_to_clipboard: (state, request) => {
    if (request.source.kind === "item") {
      requireUnlocked(state);
      if (request.source.field.kind === "sshPublicKey") {
        const { vaultId, itemId } = request.source;
        if (findItem(state, { vaultId, itemId }).detail.sshKey === null) {
          fail("fieldEmpty");
        }
      }
    } else if (request.source.kind === "pendingOtp") {
      mockPendingCode(state, request.source.pendingId);
    } else if (state.lastGenerated === null) {
      fail("fieldEmpty");
    }
    const seconds = state.settings.clipboardClearSeconds;
    if (seconds === null) {
      return { clearsAt: null, clearAfterSeconds: null };
    }
    setTimeout(() => {
      state.emit({ type: "clipboardCleared" });
    }, seconds * 1000);
    return { clearsAt: state.now() + seconds * 1000, clearAfterSeconds: seconds };
  },

  clear_clipboard: () => null,
};
