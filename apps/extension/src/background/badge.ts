/** The toolbar badge, a signal that needs no page DOM: "!" when a shortcut or menu fill could not finish, "1" when a login is waiting to be saved. */
export interface Badge {
  set(tabId: number, text: string): Promise<void>;
  /** Clears a failure mark ("!"); a pending-save mark ("1") stays. */
  clear(tabId: number): Promise<void>;
  /** Tries to open the popup so the user sees why; allowed only right after a user gesture. */
  openPopup(): Promise<void>;
}

/** The mark for a login waiting to be saved. */
export interface SaveBadge {
  show(tabId: number): Promise<void>;
  hide(tabId: number): Promise<void>;
}

const FAILURE = "!";
const SAVE = "1";

export function chromeBadge(): Badge {
  return {
    set: (tabId, text) => chrome.action.setBadgeText({ tabId, text }),
    clear: async (tabId) => {
      if ((await chrome.action.getBadgeText({ tabId })) === FAILURE) {
        await chrome.action.setBadgeText({ tabId, text: "" });
      }
    },
    openPopup: async () => {
      try {
        await chrome.action.openPopup();
      } catch {
        // Not allowed here (no window focus, or a browser that lacks it): the badge stays.
      }
    },
  };
}

export function chromeSaveBadge(): SaveBadge {
  return {
    show: async (tabId) => {
      await chrome.action.setBadgeText({ tabId, text: SAVE });
      await chrome.action.setTitle({ tabId, title: chrome.i18n.getMessage("saveBadgeTitle") });
    },
    hide: async (tabId) => {
      if ((await chrome.action.getBadgeText({ tabId })) === SAVE) {
        await chrome.action.setBadgeText({ tabId, text: "" });
      }
      await chrome.action.setTitle({ tabId, title: chrome.i18n.getMessage("extName") });
    },
  };
}
