/** Mock app status: the shortcut registered, screen-lock detection present. */
import type { MockHandlers } from "./state.ts";

/** The mock system can detect a locked screen. */
export const SCREEN_LOCK_DETECTION: boolean = true;

export const appHandlers: MockHandlers = {
  app_status: (state) => ({
    shortcut: "registered",
    screenLockDetection: SCREEN_LOCK_DETECTION,
    otpScreenScan: state.options.screenScan,
  }),
};
