/**
 * State and helpers shared by the mock IPC handlers (one file per contract area, so the
 * tracks extend their own file). The mock follows the contract, not the real crypto: it
 * stores sample data in memory and never touches a real vault.
 */
import type {
  AppEvent,
  CommandName,
  ErrorCode,
  ItemDetail,
  RequestOf,
  ResponseOf,
  SecretFieldKey,
  Settings,
  VaultSummary,
} from "../contract/index.ts";

export interface MockAccount {
  accountId: string;
  email: string;
  password: string;
  secretKey: string;
}

export interface MockItem {
  detail: ItemDetail;
  secrets: Partial<Record<SecretFieldKey, string>>;
  history: string[];
}

export interface MockState {
  account: MockAccount | null;
  secretKeyStored: boolean;
  unlocked: boolean;
  vaults: VaultSummary[];
  items: MockItem[];
  settings: Settings;
  lastGenerated: string | null;
  emit: (event: AppEvent) => void;
  now: () => number;
  nextId: () => string;
}

/** A handler's way to fail a command with a contract error code. */
export class MockFailure extends Error {
  constructor(readonly code: ErrorCode) {
    super(code);
    this.name = "MockFailure";
  }
}

export function fail(code: ErrorCode): never {
  throw new MockFailure(code);
}

export function requireUnlocked(state: MockState): void {
  if (state.account === null) {
    fail("noAccount");
  }
  if (!state.unlocked) {
    fail("locked");
  }
}

export type MockHandler<K extends CommandName> = (
  state: MockState,
  request: RequestOf<K>,
) => ResponseOf<K>;

export type MockHandlers = { [K in CommandName]?: MockHandler<K> };

/** Random values for the mock's Secret Keys and passwords (never `Math.random`). */
export function randomIndex(limit: number): number {
  const bound = Math.floor(0x1_0000_0000 / limit) * limit;
  const buffer = new Uint32Array(1);
  for (;;) {
    crypto.getRandomValues(buffer);
    const value = buffer[0] ?? 0;
    if (value < bound) {
      return value % limit;
    }
  }
}
