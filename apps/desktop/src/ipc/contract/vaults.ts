/** Vault contract: mirrors `src-tauri/src/contract/vaults.rs`. */
import type { VaultId } from "./common.ts";

export interface VaultSummary {
  vaultId: VaultId;
  /** `null` if the metadata is missing or unreadable; show a placeholder. */
  name: string | null;
  /** Live items, in and outside the trash. */
  itemCount: number;
  /** Whether this session holds the vault's key (hardware-protected vaults may not). */
  available: boolean;
}

export interface CreateVaultRequest {
  name: string;
}

export interface RenameVaultRequest {
  vaultId: VaultId;
  name: string;
}
