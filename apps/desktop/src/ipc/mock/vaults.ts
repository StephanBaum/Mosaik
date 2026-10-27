/** Mock vault commands. */
import type { VaultSummary } from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

function withCount(state: MockState, vault: VaultSummary): VaultSummary {
  return {
    ...vault,
    itemCount: state.items.filter((item) => item.detail.summary.vaultId === vault.vaultId).length,
  };
}

function validName(name: string): string {
  const trimmed = name.trim();
  if (trimmed === "" || trimmed.length > 256) {
    fail("invalidInput");
  }
  return trimmed;
}

export const vaultHandlers: MockHandlers = {
  list_vaults: (state) => {
    requireUnlocked(state);
    return state.vaults.map((vault) => withCount(state, vault));
  },

  create_vault: (state, request) => {
    requireUnlocked(state);
    const vault = { vaultId: state.nextId(), name: validName(request.name), itemCount: 0, available: true };
    state.vaults.push(vault);
    return vault;
  },

  rename_vault: (state, request) => {
    requireUnlocked(state);
    const vault = state.vaults.find((v) => v.vaultId === request.vaultId) ?? fail("vaultNotFound");
    vault.name = validName(request.name);
    return withCount(state, vault);
  },
};
