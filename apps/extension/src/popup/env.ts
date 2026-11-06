/**
 * What the popup needs from the browser besides the port: the optional site access for save
 * capture (asked for here, in the click handler, because the browser wants a user gesture),
 * the fill shortcut's key, and closing itself. A parameter, so tests supply their own.
 */
import { CAPTURE_MATCHES, COMMAND_FILL } from "../shared/ids.ts";

export interface PopupEnv {
  requestSiteAccess(): Promise<boolean>;
  removeSiteAccess(): Promise<void>;
  /** The key bound to "fill login" (for example "Ctrl+Shift+L"), or null when unset. */
  fillShortcut(): Promise<string | null>;
  /** Closes the popup after a fill; does nothing where the page is not a popup. */
  closePopup(): Promise<void>;
  /** Shown to developers on the "host missing" screen; null in the store packages. */
  hostInstallCommand: string | null;
}

export const HOST_INSTALL_COMMAND = "pnpm --filter @mosaic/extension host:install";

export function chromeEnv(): PopupEnv {
  return {
    requestSiteAccess: async () => {
      try {
        return await chrome.permissions.request({ origins: CAPTURE_MATCHES });
      } catch {
        return false;
      }
    },
    removeSiteAccess: async () => {
      try {
        await chrome.permissions.remove({ origins: CAPTURE_MATCHES });
      } catch {
        // Required permissions (test builds) cannot be removed; the setting is off all the same.
      }
    },
    fillShortcut: async () => {
      const commands = await chrome.commands.getAll();
      const key = commands.find((command) => command.name === COMMAND_FILL)?.shortcut;
      return key === undefined || key === "" ? null : key;
    },
    closePopup: async () => {
      // `tabs.getCurrent` is undefined in a popup and a tab where the page was opened in one.
      if ((await chrome.tabs.getCurrent()) === undefined) {
        window.close();
      }
    },
    // Only dev and e2e builds name the developer command; the store packages show the user text alone.
    hostInstallCommand:
      import.meta.env.MODE === "dev" || import.meta.env.MODE === "e2e" ? HOST_INSTALL_COMMAND : null,
  };
}
