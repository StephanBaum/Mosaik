/**
 * `chrome.storage.session`: memory only, readable by the extension's own contexts, gone when
 * the browser closes. Holds the small per-tab facts (last fill, last error), never a secret.
 */
export interface SessionStore {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  remove(...keys: string[]): Promise<void>;
  /** The keys that start with `prefix`. */
  keys(prefix: string): Promise<string[]>;
}

export function chromeSession(): SessionStore {
  return {
    get: async (key) => (await chrome.storage.session.get(key))[key],
    set: (key, value) => chrome.storage.session.set({ [key]: value }),
    remove: (...keys) => chrome.storage.session.remove(keys),
    keys: async (prefix) => Object.keys(await chrome.storage.session.get(null)).filter((key) => key.startsWith(prefix)),
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
    keys: (prefix) => Promise.resolve([...map.keys()].filter((key) => key.startsWith(prefix))),
  };
}

export const pendingSaveKey = (tabId: number): string => `pendingSave:${String(tabId)}`;
export const PENDING_SAVE_PREFIX = "pendingSave:";
export const lastFilledKey = (tabId: number): string => `lastFilled:${String(tabId)}`;
/** A tab the extension sent to a change page (`background/rotation.ts`). */
export const rotationTabKey = (tabId: number): string => `rotationTab:${String(tabId)}`;
export const lastErrorKey = (tabId: number): string => `lastError:${String(tabId)}`;
