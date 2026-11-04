/** Mock item commands. `add_item` and `update_item` apply the parts of a draft the UI sends. */
import type {
  AddItemRequest,
  ItemDraft,
  ItemKind,
  ItemRequest,
  ListItemsRequest,
  OtpSummary,
  TextFieldKey,
} from "../contract/index.ts";
import { requireFreshReauth } from "./quickUnlock.ts";
import { hardwareVaultOf, vaultReadable } from "./vaults.ts";
import { fail, requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

function find(state: MockState, request: ItemRequest): MockItem {
  if (!state.vaults.some((vault) => vault.vaultId === request.vaultId)) {
    fail("vaultNotFound");
  }
  if (!vaultReadable(state, request.vaultId)) {
    fail("vaultUnavailable");
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

/** The one-time code a draft sets, or `undefined` if it leaves the code alone. */
function otpOf(state: MockState, vaultId: string, draft: ItemDraft): OtpSummary | null | undefined {
  const change = draft.otp;
  if (change === null) {
    return undefined;
  }
  if (change.op === "remove") {
    return null;
  }
  if (change.op === "set") {
    return { scheme: "totp", issuer: null, digits: 6, period: 30, hardwareProtected: false };
  }
  const pending = state.otpPending.get(change.pendingId) ?? fail("otpPendingExpired");
  state.otpPending.delete(change.pendingId);
  return {
    scheme: pending.scheme,
    issuer: change.issuer === "" ? null : change.issuer,
    digits: pending.digits,
    period: pending.period,
    hardwareProtected: hardwareVaultOf(state, vaultId) !== undefined,
  };
}

function setText(item: MockItem, field: TextFieldKey, value: string | null): void {
  const summary = item.detail.summary;
  if (field === "title") {
    summary.title = value;
  } else if (field === "username") {
    summary.subtitle = value;
  }
  item.detail.text = item.detail.text.filter((entry) => entry.field !== field);
  if (value !== null && value !== "" && field !== "title") {
    item.detail.text.push({ field, value });
  }
}

function applyDraft(state: MockState, item: MockItem, draft: ItemDraft): void {
  const summary = item.detail.summary;
  const otp = otpOf(state, summary.vaultId, draft);
  for (const change of draft.text) {
    setText(item, change.field, change.value);
  }
  for (const change of draft.secrets) {
    if (change.value === null || change.value === "") {
      Reflect.deleteProperty(item.secrets, change.field);
      item.detail.secrets = item.detail.secrets.filter((field) => field !== change.field);
    } else {
      item.secrets[change.field] = change.value;
      if (!item.detail.secrets.includes(change.field)) {
        item.detail.secrets.push(change.field);
      }
    }
  }
  for (const change of draft.urls) {
    if (change.op === "add") {
      item.detail.urls.push({
        elementId: crypto.randomUUID(),
        url: change.url,
        matchMode: change.matchMode,
        allowHttp: change.allowHttp,
      });
      summary.host = URL.canParse(change.url) ? new URL(change.url).hostname : summary.host;
    }
  }
  if (draft.tags !== null) {
    summary.tags = draft.tags;
  }
  if (otp !== undefined) {
    item.detail.otp = otp;
    summary.hasOtp = otp !== null;
  }
  summary.modifiedAt = state.now();
}

function newItem(state: MockState, request: AddItemRequest): MockItem {
  const kind: ItemKind = request.kind;
  return {
    detail: {
      summary: {
        vaultId: request.vaultId,
        itemId: crypto.randomUUID(),
        kind,
        title: null,
        subtitle: null,
        host: null,
        favorite: false,
        hasOtp: false,
        tags: [],
        modifiedAt: state.now(),
        trashedAt: null,
        needsReview: false,
      },
      createdAt: state.now(),
      text: [],
      secrets: [],
      urls: [],
      customFields: [],
      attachments: [],
      otp: null,
      historyCount: 0,
      review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
    },
    secrets: {},
    history: [],
  };
}

export const itemHandlers: MockHandlers = {
  add_item: (state, request) => {
    requireUnlocked(state);
    if (!state.vaults.some((vault) => vault.vaultId === request.vaultId)) {
      fail("vaultNotFound");
    }
    if (!vaultReadable(state, request.vaultId)) {
      fail("vaultUnavailable");
    }
    if (hardwareVaultOf(state, request.vaultId) !== undefined && request.kind !== "totp") {
      fail("itemTypeNotAllowed");
    }
    const item = newItem(state, request);
    applyDraft(state, item, request.draft);
    state.items.push(item);
    return item.detail;
  },

  update_item: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    applyDraft(state, item, request.draft);
    return item.detail;
  },

  list_items: (state, request) => {
    requireUnlocked(state);
    return state.items
      .filter((item) => vaultReadable(state, item.detail.summary.vaultId) && matches(item, request))
      .map((item) => item.detail.summary);
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
    if (hardwareVaultOf(state, request.vaultId) !== undefined) {
      requireFreshReauth(state, false);
    }
    const itemId = state.nextId();
    item.detail.summary = { ...item.detail.summary, vaultId: request.targetVaultId, itemId };
    return { vaultId: request.targetVaultId, itemId };
  },
};
