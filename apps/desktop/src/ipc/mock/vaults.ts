/** Mock vault commands, including hardware-protected vaults. */
import type { HardwareVaultKeys, VaultSummary } from "../contract/index.ts";
import {
  closeHardwareVault,
  requireFreshReauth,
  newKey,
  verifyPin,
  verifyTouch,
} from "./quickUnlock.ts";
import {
  fail,
  requireUnlocked,
  type MockAsyncHandlers,
  type MockHandlers,
  type MockHardwareVault,
  type MockState,
} from "./state.ts";

const MAX_VAULT_KEYS = 8;

export function hardwareVaultOf(state: MockState, vaultId: string): MockHardwareVault | undefined {
  return state.hardwareVaults.find((vault) => vault.vaultId === vaultId);
}

/** Whether items of `vaultId` may be read: always, except for a closed hardware vault. */
export function vaultReadable(state: MockState, vaultId: string): boolean {
  const hardware = hardwareVaultOf(state, vaultId);
  return hardware === undefined || hardware.open;
}

export function summaryOf(state: MockState, vault: VaultSummary): VaultSummary {
  const hardware = hardwareVaultOf(state, vault.vaultId);
  if (hardware !== undefined && !hardware.open) {
    // A closed hardware vault's metadata is encrypted under its key: nothing to show.
    return { ...vault, name: null, itemCount: 0, available: false, openUntil: null };
  }
  return {
    ...vault,
    available: true,
    itemCount: state.items.filter((item) => item.detail.summary.vaultId === vault.vaultId).length,
    openUntil: hardware?.openUntil ?? null,
  };
}

function validName(name: string): string {
  const trimmed = name.trim();
  if (trimmed === "" || trimmed.length > 256) {
    fail("invalidInput");
  }
  return trimmed;
}

function hardwareVault(state: MockState, vaultId: string): MockHardwareVault {
  return hardwareVaultOf(state, vaultId) ?? fail("vaultNotFound");
}

function keysOf(vault: MockHardwareVault): HardwareVaultKeys {
  return { keys: vault.keys };
}

function requireOpen(vault: MockHardwareVault): void {
  if (!vault.open) {
    fail("vaultUnavailable");
  }
}

function open(state: MockState, vault: MockHardwareVault): void {
  vault.open = true;
  vault.openUntil = state.now() + state.quick.hwOpenSeconds * 1000;
  clearTimeout(vault.timer);
  vault.timer = setTimeout(() => {
    closeHardwareVault(state, vault, "timer");
  }, state.quick.hwOpenSeconds * 1000);
}

export const vaultHandlers: MockHandlers = {
  list_vaults: (state) => {
    requireUnlocked(state);
    return state.vaults.map((vault) => summaryOf(state, vault));
  },

  create_vault: (state, request) => {
    requireUnlocked(state);
    const vault: VaultSummary = {
      vaultId: state.nextId(),
      name: validName(request.name),
      itemCount: 0,
      available: true,
      protection: "accountKey",
      openUntil: null,
    };
    state.vaults.push(vault);
    return vault;
  },

  rename_vault: (state, request) => {
    requireUnlocked(state);
    const vault = state.vaults.find((v) => v.vaultId === request.vaultId) ?? fail("vaultNotFound");
    if (!vaultReadable(state, vault.vaultId)) {
      fail("vaultUnavailable");
    }
    vault.name = validName(request.name);
    return summaryOf(state, vault);
  },



  close_hardware_vault: (state, request) => {
    requireUnlocked(state);
    closeHardwareVault(state, hardwareVault(state, request.vaultId), "lock");
    return null;
  },

  hardware_vault_keys: (state, request) => {
    requireUnlocked(state);
    return keysOf(hardwareVault(state, request.vaultId));
  },


  remove_hardware_vault_key: (state, request) => {
    requireUnlocked(state);
    const hardware = hardwareVault(state, request.vaultId);
    requireOpen(hardware);
    if (!hardware.keys.some((key) => key.methodId === request.methodId)) {
      fail("hardwareKeyNotRecognized");
    }
    if (hardware.keys.length === 1) {
      fail("hardwareKeyLast");
    }
    hardware.keys = hardware.keys.filter((key) => key.methodId !== request.methodId);
    return keysOf(hardware);
  },
};

export const vaultAsyncHandlers: MockAsyncHandlers = {
  create_hardware_vault: async (state, request) => {
    requireUnlocked(state);
    if (!state.quick.keysSupported) {
      fail("hardwareKeyUnsupported");
    }
    const name = validName(request.name);
    verifyPin(request.pin);
    await verifyTouch(state);
    const vault: VaultSummary = {
      vaultId: state.nextId(),
      name,
      itemCount: 0,
      available: true,
      protection: "hardwareKey",
      openUntil: null,
    };
    state.vaults.push(vault);
    const hardware: MockHardwareVault = {
      vaultId: vault.vaultId,
      name,
      open: false,
      openUntil: null,
      keys: [newKey(state)],
      timer: undefined,
    };
    state.hardwareVaults.push(hardware);
    open(state, hardware);
    return summaryOf(state, vault);
  },
  open_hardware_vault: async (state, request) => {
    requireUnlocked(state);
    const hardware = hardwareVault(state, request.vaultId);
    verifyPin(request.pin);
    await verifyTouch(state);
    open(state, hardware);
    const vault = state.vaults.find((v) => v.vaultId === request.vaultId) ?? fail("vaultNotFound");
    return summaryOf(state, vault);
  },
  add_hardware_vault_key: async (state, request) => {
    requireUnlocked(state);
    const hardware = hardwareVault(state, request.vaultId);
    requireOpen(hardware);
    requireFreshReauth(state, true, "hardwareVaultKey");
    if (hardware.keys.length >= MAX_VAULT_KEYS) {
      fail("hardwareKeyLimit");
    }
    verifyPin(request.pin);
    await verifyTouch(state);
    hardware.keys = [...hardware.keys, newKey(state)];
    return keysOf(hardware);
  },
};
