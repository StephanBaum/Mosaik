/** Mock clipboard commands: no real clipboard, but the receipt and the cleared event. */
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";

export const clipboardHandlers: MockHandlers = {
  copy_to_clipboard: (state, request) => {
    if (request.source.kind === "item") {
      requireUnlocked(state);
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
