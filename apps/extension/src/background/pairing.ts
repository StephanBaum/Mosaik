import type { Pairing } from "../shared/protocol.ts";
import { isPairing } from "../shared/validate.ts";
import type { KeyValueStore } from "./db.ts";

const KEY = "pairing";

/** The pairing (ID and `K_pair`) in the extension's IndexedDB. Only the background reads it. */
export class PairingStore {
  readonly #kv: KeyValueStore;

  constructor(kv: KeyValueStore) {
    this.#kv = kv;
  }

  async load(): Promise<Pairing | null> {
    const stored = await this.#kv.get(KEY);
    if (typeof stored !== "object" || stored === null) {
      return null;
    }
    const { pairingId, key } = stored as Record<string, unknown>;
    const pairing = { pairingId, key };
    return isPairing(pairing) ? pairing : null;
  }

  async save(pairing: Pairing): Promise<void> {
    await this.#kv.set(KEY, { pairingId: pairing.pairingId, key: pairing.key, createdAt: Date.now() });
  }

  async clear(): Promise<void> {
    await this.#kv.delete(KEY);
  }
}
