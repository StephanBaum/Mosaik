/**
 * The extension's small persistent store (docs/EXTENSION_SECURITY.md, "storage"): IndexedDB in
 * the extension's own origin, used by the background only. Never `storage.local` or
 * `storage.sync`: those are readable by every extension page and may sync to the cloud.
 */
export interface KeyValueStore {
  get(key: string): Promise<unknown>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  /** Removes everything (the e2e reset; nothing else calls it). */
  clear(): Promise<void>;
}

const DB_NAME = "mosaic";
const STORE = "kv";

function request<T>(req: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => {
      resolve(req.result);
    };
    req.onerror = () => {
      reject(req.error ?? new Error("indexeddb"));
    };
  });
}

function open(): Promise<IDBDatabase> {
  const opening = indexedDB.open(DB_NAME, 1);
  opening.onupgradeneeded = () => {
    opening.result.createObjectStore(STORE);
  };
  return request(opening);
}

export function createIndexedDbStore(): KeyValueStore {
  let db: Promise<IDBDatabase> | null = null;
  const store = async (mode: IDBTransactionMode): Promise<IDBObjectStore> => {
    db ??= open();
    return (await db).transaction(STORE, mode).objectStore(STORE);
  };
  return {
    get: async (key) => request<unknown>((await store("readonly")).get(key)),
    set: async (key, value) => {
      await request((await store("readwrite")).put(value, key));
    },
    delete: async (key) => {
      await request((await store("readwrite")).delete(key));
    },
    clear: async () => {
      await request((await store("readwrite")).clear());
    },
  };
}

export function createMemoryStore(): KeyValueStore {
  const map = new Map<string, unknown>();
  return {
    get: (key) => Promise.resolve(map.get(key)),
    set: (key, value) => {
      map.set(key, value);
      return Promise.resolve();
    },
    delete: (key) => {
      map.delete(key);
      return Promise.resolve();
    },
    clear: () => {
      map.clear();
      return Promise.resolve();
    },
  };
}
