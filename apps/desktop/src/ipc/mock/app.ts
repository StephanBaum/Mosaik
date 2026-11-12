/** Mock app status: the shortcut registered, screen-lock detection present. */
import type { MockHandlers, MockState } from "./state.ts";

/** The mock system can detect a locked screen. */
export const SCREEN_LOCK_DETECTION: boolean = true;

export const appHandlers: MockHandlers = {
  app_status: (state) => ({
    shortcut: state.options.shortcut,
    screenLockDetection: SCREEN_LOCK_DETECTION,
    otpScreenScan: state.options.screenScan,
  }),
};

/** `&metaDropped=`: a sync run found vault metadata too long to keep whole (M9). */
export function announceMetaDropped(state: MockState): void {
  const fields = state.options.metaDropped;
  const vault = state.vaults[0];
  if (!state.unlocked || fields.length === 0 || vault === undefined) {
    return;
  }
  setTimeout(() => {
    state.emit({ type: "vaultMetaDropped", vaultId: vault.vaultId, fields });
  }, 500);
}
