import type { Hint } from "./protocol.ts";

/** The native messaging host name (docs/NATIVE_MESSAGING.md §7). */
export const HOST_NAME = "io.github.stephanbaum.mosaic";
/** The Firefox add-on ID the host accepts. */
export const FIREFOX_EXTENSION_ID = "mosaic@stephanbaum.github.io";
/** The ID that `keys/dev-public-key.b64` gives dev and e2e builds (checked in `manifest.test.ts`). */
export const DEV_CHROMIUM_EXTENSION_ID = "lccdgfpfnmegbdgooanfejdbahhokikm";

export const POPUP_PAGE = "popup.html";
export const CONTENT_SCRIPT = "content.js";

/** A random ID for one fill, so a reply can be matched to its request. */
export function newFillId(): string {
  return crypto.randomUUID();
}

/** The browser as far as the user agent says; display only (Brave cannot be told from Chrome). */
export function detectHint(userAgent: string): Hint {
  if (/Firefox\//.test(userAgent)) {
    return "firefox";
  }
  if (/Edg\//.test(userAgent)) {
    return "edge";
  }
  if (/OPR\//.test(userAgent)) {
    return "opera";
  }
  if (/Vivaldi\//.test(userAgent)) {
    return "vivaldi";
  }
  if (/Chrome\//.test(userAgent)) {
    return "chrome";
  }
  return "other";
}
