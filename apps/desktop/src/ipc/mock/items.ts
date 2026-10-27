/** Mock item commands. `add_item` and `update_item` belong to the vault UI track. */
import type { ItemRequest, ListItemsRequest } from "../contract/index.ts";
import { fail, requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

function find(state: MockState, request: ItemRequest): MockItem {
  if (!state.vaults.some((vault) => vault.vaultId === request.vaultId)) {
    fail("vaultNotFound");
  }
  return (
    state.items.find(
      (item) =>
        item.detail.summary.vaultId === request.vaultId &&
        item.detail.summary.itemId === request.itemId,
    ) ?? fail("itemNotFound")
  );
}

function matches(item: MockItem, request: ListItemsRequest): boolean {
  const summary = item.detail.summary;
  if (request.vaultId !== null && summary.vaultId !== request.vaultId) {
    return false;
  }
  if (request.favoritesOnly && !summary.favorite) {
    return false;
  }
  const trashed = summary.trashedAt !== null;
  if ((request.trash === "exclude" && trashed) || (request.trash === "only" && !trashed)) {
    return false;
  }
  const query = request.query?.trim().toLowerCase() ?? "";
  if (query === "") {
    return true;
  }
  return [summary.title, summary.subtitle, summary.host, ...summary.tags].some(
    (text) => text?.toLowerCase().includes(query) === true,
  );
}

export const itemHandlers: MockHandlers = {
  list_items: (state, request) => {
    requireUnlocked(state);
    return state.items.filter((item) => matches(item, request)).map((item) => item.detail.summary);
  },

  get_item: (state, request) => {
    requireUnlocked(state);
    return find(state, request).detail;
  },

  reveal_secret: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    const secret = request.secret;
    const value =
      secret.kind === "field"
        ? item.secrets[secret.field]
        : secret.kind === "history"
          ? item.history[secret.index]
          : undefined;
    return { value: value ?? fail("fieldEmpty"), hideAfterSeconds: 60 };
  },

  set_favorite: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    item.detail.summary.favorite = request.favorite;
    return item.detail.summary;
  },

  trash_item: (state, request) => {
    requireUnlocked(state);
    find(state, request).detail.summary.trashedAt = state.now();
    return null;
  },

  restore_item: (state, request) => {
    requireUnlocked(state);
    find(state, request).detail.summary.trashedAt = null;
    return null;
  },

  purge_item: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    state.items = state.items.filter((each) => each !== item);
    return null;
  },

  move_item: (state, request) => {
    requireUnlocked(state);
    if (request.vaultId === request.targetVaultId) {
      fail("sameVault");
    }
    if (!state.vaults.some((vault) => vault.vaultId === request.targetVaultId)) {
      fail("vaultNotFound");
    }
    const item = find(state, request);
    const itemId = state.nextId();
    item.detail.summary = { ...item.detail.summary, vaultId: request.targetVaultId, itemId };
    return { vaultId: request.targetVaultId, itemId };
  },
};
