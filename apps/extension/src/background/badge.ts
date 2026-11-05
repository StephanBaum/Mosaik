/** The toolbar badge, a signal that needs no page DOM: "!" when a shortcut or menu fill could not finish. */
export interface Badge {
  set(tabId: number, text: string): Promise<void>;
  clear(tabId: number): Promise<void>;
  /** Tries to open the popup so the user sees why; allowed only right after a user gesture. */
  openPopup(): Promise<void>;
}

export function chromeBadge(): Badge {
  return {
    set: (tabId, text) => chrome.action.setBadgeText({ tabId, text }),
    clear: (tabId) => chrome.action.setBadgeText({ tabId, text: "" }),
    openPopup: async () => {
      try {
        await chrome.action.openPopup();
      } catch {
        // Not allowed here (no window focus, or a browser that lacks it): the badge stays.
      }
    },
  };
}
