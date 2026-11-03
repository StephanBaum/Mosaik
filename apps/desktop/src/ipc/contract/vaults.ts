/** Vault contract: mirrors `src-tauri/src/contract/vaults.rs`. */
import type { UnixMillis, VaultId } from "./common.ts";

export const vaultProtections = ["accountKey", "hardwareKey"] as const;
export type VaultProtection = (typeof vaultProtections)[number];

export interface VaultSummary {
  vaultId: VaultId;
  /** `null` if the metadata is missing or unreadable; show a placeholder. */
  name: string | null;
  /** Live items, in and outside the trash. */
  itemCount: number;
  /** Whether this session holds the vault's key (hardware-protected vaults may not). */
  available: boolean;
  /** `hardwareKey`: the vault holds one-time codes only and opens with a security key. */
  protection: VaultProtection;
  /** Wall-clock end of the open window of a hardware vault; display only. */
  openUntil: UnixMillis | null;
}

export interface CreateVaultRequest {
  name: string;
}

export interface RenameVaultRequest {
  vaultId: VaultId;
  name: string;
}

/** A request naming one vault (`close_hardware_vault`, `hardware_vault_keys`). */
export interface VaultRequest {
  vaultId: VaultId;
}

export interface CreateHardwareVaultRequest {
  name: string;
  pin: string | null;
}

/** Open a hardware vault or add a key to it. */
export interface HardwareVaultRequest {
  vaultId: VaultId;
  pin: string | null;
}

export interface HardwareVaultKeyRequest {
  vaultId: VaultId;
  methodId: string;
}
