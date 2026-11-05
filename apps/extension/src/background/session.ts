/**
 * `chrome.storage.session`: memory only, readable by the extension's own contexts, gone when
 * the browser closes. Holds the small per-tab facts (last fill, last error), never a secret.
 */
export interface SessionStore {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  remove(...keys: string[]): Promise<void>;
}

export function chromeSession(): SessionStore {
  return {
    get: async (key) => (await chrome.storage.session.get(key))[key],
    set: (key, value) => chrome.storage.session.set({ [key]: value }),
    remove: (...keys) => chrome.storage.session.remove(keys),
  };
}

export function memorySession(): SessionStore {
  const map = new Map<string, unknown>();
  return {
    get: (key) => Promise.resolve(map.get(key)),
    set: (key, value) => {
      map.set(key, value);
      return Promise.resolve();
    },
    remove: (...keys) => {
      for (const key of keys) {
        map.delete(key);
      }
      return Promise.resolve();
    },
  };
}

export const lastFilledKey = (tabId: number): string => `lastFilled:${String(tabId)}`;
export const lastErrorKey = (tabId: number): string => `lastError:${String(tabId)}`;
