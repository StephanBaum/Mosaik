/** Mock one-time codes: a fixed code with a real countdown. */
import { fail, requireUnlocked, type MockHandlers } from "./state.ts";

export const totpHandlers: MockHandlers = {
  totp_code: (state, request) => {
    requireUnlocked(state);
    const item =
      state.items.find(
        (each) =>
          each.detail.summary.vaultId === request.vaultId &&
          each.detail.summary.itemId === request.itemId,
      ) ?? fail("itemNotFound");
    const otp = item.detail.otp ?? fail("noOtp");
    if (otp.scheme === "hotp" || otp.period === null) {
      return { code: "755224", secondsRemaining: null, period: null };
    }
    const elapsed = Math.floor(state.now() / 1000) % otp.period;
    return { code: "492039", secondsRemaining: otp.period - elapsed, period: otp.period };
  },
};
