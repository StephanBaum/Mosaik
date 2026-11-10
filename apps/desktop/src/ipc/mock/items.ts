/** Mock item commands. `add_item` and `update_item` apply the parts of a draft the UI sends. */
import type {
  AddItemRequest,
  CustomFieldChange,
  CustomFieldEntry,
  ItemDraft,
  ItemKind,
  ItemRequest,
  ListItemsRequest,
  OtpSummary,
  TextFieldKey,
  UrlChange,
} from "../contract/index.ts";
import { passwordChanged, pushHistory, refreshDetail } from "./credentialRotation.ts";
import { MOCK_SSH_SUMMARY } from "./data.ts";
import { requireFreshReauth } from "./quickUnlock.ts";
import { hardwareVaultOf, vaultReadable } from "./vaults.ts";
import { fail, requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

export function findItem(state: MockState, request: ItemRequest): MockItem {
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

/** The text field the list shows under the title, by kind. */
const SUBTITLE_FIELD: Partial<Record<ItemKind, TextFieldKey>> = {
  login: "username",
  creditCard: "cardholderName",
  identity: "email",
  wifi: "wifiSsid",
  apiKey: "apiKeyHost",
  sshKey: "sshComment",
};

function setText(item: MockItem, field: TextFieldKey, value: string | null): void {
  const summary = item.detail.summary;
  if (field === "title") {
    summary.title = value;
  } else if (SUBTITLE_FIELD[summary.kind] === field) {
    summary.subtitle = value === "" ? null : value;
  }
  item.detail.text = item.detail.text.filter((entry) => entry.field !== field);
  if (value !== null && value !== "" && field !== "title") {
    item.detail.text.push({ field, value });
  }
}

function applyUrls(item: MockItem, changes: readonly UrlChange[]): void {
  const detail = item.detail;
  for (const change of changes) {
    if (change.op === "add") {
      detail.urls.push({
        elementId: crypto.randomUUID(),
        url: change.url,
        matchMode: change.matchMode,
        allowHttp: change.allowHttp,
      });
    } else {
      const at = detail.urls.findIndex((entry) => entry.elementId === change.elementId);
      if (at < 0) {
        fail("invalidInput");
      }
      if (change.op === "remove") {
        detail.urls.splice(at, 1);
      } else {
        detail.urls[at] = {
          elementId: change.elementId,
          url: change.url,
          matchMode: change.matchMode,
          allowHttp: change.allowHttp,
        };
      }
    }
  }
  const first = detail.urls[0]?.url;
  detail.summary.host = first !== undefined && URL.canParse(first) ? new URL(first).hostname : null;
}

function customEntry(
  item: MockItem,
  elementId: string,
  change: { label: string; kind: CustomFieldEntry["kind"] },
  value: string,
): CustomFieldEntry {
  const secret = change.kind === "hidden";
  if (secret) {
    item.customSecrets[elementId] = value;
  } else {
    Reflect.deleteProperty(item.customSecrets, elementId);
  }
  return {
    elementId,
    label: change.label,
    kind: change.kind,
    value: secret || change.kind === "otp" ? null : value,
  };
}

function applyCustomFields(item: MockItem, changes: readonly CustomFieldChange[]): void {
  const fields = item.detail.customFields;
  for (const change of changes) {
    if (change.op === "add") {
      const elementId = crypto.randomUUID();
      fields.push(customEntry(item, elementId, change, change.value));
      continue;
    }
    const at = fields.findIndex((entry) => entry.elementId === change.elementId);
    const stored = fields[at];
    if (stored === undefined) {
      fail("invalidInput");
    }
    if (change.op === "remove") {
      fields.splice(at, 1);
      Reflect.deleteProperty(item.customSecrets, change.elementId);
    } else if (change.value === null) {
      // Keep the stored value: only a rename, and only of the same kind.
      if (change.kind !== stored.kind) {
        fail("invalidInput");
      }
      fields[at] = { ...stored, label: change.label };
    } else {
      fields[at] = customEntry(item, change.elementId, change, change.value);
    }
  }
}

function applySshKey(item: MockItem, draft: ItemDraft): void {
  const change = draft.sshKey;
  if (change === null) {
    return;
  }
  if (item.detail.summary.kind !== "sshKey") {
    fail("invalidInput");
  }
  if (!change.privateKey.includes("BEGIN OPENSSH PRIVATE KEY")) {
    fail("sshKeyInvalid");
  }
  item.sshKey = { privateKey: change.privateKey };
  item.detail.sshKey = {
    ...MOCK_SSH_SUMMARY,
    encrypted: change.privateKey.includes("aes256-ctr"),
  };
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
      const old = item.secrets[change.field];
      item.secrets[change.field] = change.value;
      if (!item.detail.secrets.includes(change.field)) {
        item.detail.secrets.push(change.field);
      }
      // A changed password moves the old one to the history (Rust `ItemEdit::set_secret`).
      if (change.field === "password" && old !== undefined && old !== change.value) {
        pushHistory(state, item, old, "replaced");
        passwordChanged(state, item);
      }
    }
  }
  applyUrls(item, draft.urls);
  refreshDetail(item);
  applyCustomFields(item, draft.customFields);
  applySshKey(item, draft);
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
      history: [],
      rotationAttempts: [],
      rotationPolicy: null,
      changeUrl: null,
      review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
      sshKey: null,
      passkey: null,
    },
    secrets: {},
    history: [],
    customSecrets: {},
    sshKey: null,
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
    const item = findItem(state, request);
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
    return findItem(state, request).detail;
  },

  reveal_secret: (state, request) => {
    requireUnlocked(state);
    const item = findItem(state, request);
    const secret = request.secret;
    const value =
      secret.kind === "field"
        ? item.secrets[secret.field]
        : secret.kind === "customField"
          ? item.customSecrets[secret.elementId]
          : item.history[secret.index];
    return { value: value ?? fail("fieldEmpty"), hideAfterSeconds: 60 };
  },

  set_favorite: (state, request) => {
    requireUnlocked(state);
    const item = findItem(state, request);
    item.detail.summary.favorite = request.favorite;
    return item.detail.summary;
  },

  trash_item: (state, request) => {
    requireUnlocked(state);
    findItem(state, request).detail.summary.trashedAt = state.now();
    return null;
  },

  restore_item: (state, request) => {
    requireUnlocked(state);
    findItem(state, request).detail.summary.trashedAt = null;
    return null;
  },

  purge_item: (state, request) => {
    requireUnlocked(state);
    const item = findItem(state, request);
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
    const item = findItem(state, request);
    if (hardwareVaultOf(state, request.vaultId) !== undefined) {
      requireFreshReauth(state, false);
    }
    const itemId = state.nextId();
    item.detail.summary = { ...item.detail.summary, vaultId: request.targetVaultId, itemId };
    return { vaultId: request.targetVaultId, itemId };
  },
};
