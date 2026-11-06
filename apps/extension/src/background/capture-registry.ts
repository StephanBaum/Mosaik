/**
 * The opt-in save capture script (ADR-0038). It is never in the manifest: turning the setting
 * on asks for the optional host permission (in the popup, on a click) and registers
 * `capture.js` for all pages; turning it off, or the browser taking the permission back,
 * unregisters it.
 */
import { CAPTURE_MATCHES } from "../shared/ids.ts";

export const CAPTURE_SCRIPT_ID = "capture";
export const CAPTURE_SCRIPT_FILE = "capture.js";
export { CAPTURE_MATCHES };

export interface CaptureRegistry {
  /** Whether the site access (`optional_host_permissions`) is granted. */
  hasAccess(): Promise<boolean>;
  isRegistered(): Promise<boolean>;
  register(): Promise<void>;
  unregister(): Promise<void>;
}

export function chromeCaptureRegistry(): CaptureRegistry {
  return {
    hasAccess: () => chrome.permissions.contains({ origins: CAPTURE_MATCHES }),
    isRegistered: async () =>
      (await chrome.scripting.getRegisteredContentScripts({ ids: [CAPTURE_SCRIPT_ID] })).length > 0,
    register: async () => {
      await chrome.scripting.registerContentScripts([
        {
          id: CAPTURE_SCRIPT_ID,
          matches: CAPTURE_MATCHES,
          js: [CAPTURE_SCRIPT_FILE],
          allFrames: true,
          runAt: "document_idle",
          persistAcrossSessions: true,
        },
      ]);
    },
    unregister: async () => {
      try {
        await chrome.scripting.unregisterContentScripts({ ids: [CAPTURE_SCRIPT_ID] });
      } catch {
        // Not registered: nothing to undo.
      }
    },
  };
}
