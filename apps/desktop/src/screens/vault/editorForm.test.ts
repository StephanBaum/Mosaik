import { describe, expect, it } from "vitest";
import type { ItemDetail, ItemKind, VaultSummary } from "../../ipc/contract/index.ts";
import {
  buildDraft,
  canAddCustomField,
  changeKind,
  hasErrors,
  initialForm,
  MAX_CUSTOM_FIELDS,
  newCustomRow,
  validateForm,
  type Form,
} from "./editorForm.ts";

const vaults: VaultSummary[] = [
  { vaultId: "v1", name: "Personal", itemCount: 0, available: true, protection: "accountKey", openUntil: null },
];

function detail(kind: ItemKind, over: Partial<ItemDetail> = {}): ItemDetail {
  return {
    summary: {
      vaultId: "v1",
      itemId: "i1",
      kind,
      title: "Item",
      subtitle: null,
      host: null,
      favorite: false,
      hasOtp: false,
      tags: ["a"],
      modifiedAt: null,
      trashedAt: null,
      needsReview: false,
    },
    createdAt: null,
    text: [],
    secrets: [],
    urls: [],
    customFields: [],
    attachments: [],
    otp: null,
    historyCount: 0,
    review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
    sshKey: null,
    ...over,
  };
}

function must<T>(value: T | undefined): T {
  if (value === undefined) {
    throw new Error("missing");
  }
  return value;
}

function created(kind: ItemKind): Form {
  return changeKind(initialForm(null, vaults, "v1"), kind);
}

describe("buildDraft for a new item", () => {
  it("sends the title, the layout fields and nothing else", () => {
    const initial = initialForm(null, vaults, "v1");
    const form: Form = {
      ...initial,
      title: "GitHub",
      text: { username: "alex", cardholderName: "foreign" },
      secrets: { password: "pw", cardNumber: "4111" },
      urls: [{ ...must(initial.urls[0]), url: " https://github.com " }],
    };
    expect(buildDraft(form, initial)).toEqual({
      text: [
        { field: "title", value: "GitHub" },
        { field: "username", value: "alex" },
      ],
      secrets: [{ field: "password", value: "pw" }],
      urls: [{ op: "add", url: "https://github.com", matchMode: "exactOrigin", allowHttp: false }],
      customFields: [],
      tags: [],
      otp: null,
      sshKey: null,
    });
  });

  it("normalizes the number and the expiry of a card", () => {
    const initial = initialForm(null, vaults, "v1");
    const form: Form = {
      ...created("creditCard"),
      title: "Visa",
      text: { cardholderName: "Alex", cardExpiry: "0429", username: "foreign" },
      secrets: { cardNumber: "4111 1111 1111 1111", cardPin: "1234", password: "foreign" },
    };
    const draft = buildDraft(form, initial);
    expect(draft.text).toEqual([
      { field: "title", value: "Visa" },
      { field: "cardholderName", value: "Alex" },
      { field: "cardExpiry", value: "04/2029" },
    ]);
    expect(draft.secrets).toEqual([
      { field: "cardNumber", value: "4111111111111111" },
      { field: "cardPin", value: "1234" },
    ]);
    expect(draft.urls).toEqual([]);
  });

  it("drops the fields of the old kind when the kind changes", () => {
    const initial = initialForm(null, vaults, "v1");
    const login: Form = { ...initial, title: "Home", text: { username: "alex" }, secrets: { password: "pw" } };
    const wifi = changeKind(login, "wifi");
    const draft = buildDraft({ ...wifi, text: { ...wifi.text, wifiSsid: "Mosaic-5G" } }, initial);
    expect(draft.text.map((change) => change.field)).toEqual(["title", "wifiSsid", "wifiSecurity"]);
    expect(draft.text[2]).toEqual({ field: "wifiSecurity", value: "WPA2" });
    expect(draft.secrets).toEqual([]);
    expect(draft.urls).toEqual([]);
    expect(changeKind(login, "login").text.wifiSecurity).toBeUndefined();
  });

  it("sends the SSH key only for an SSH key item", () => {
    const initial = initialForm(null, vaults, "v1");
    const key = "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA\n-----END OPENSSH PRIVATE KEY-----";
    expect(buildDraft({ ...created("sshKey"), sshPrivateKey: key }, initial).sshKey).toEqual({
      op: "set",
      privateKey: key,
    });
    expect(buildDraft({ ...created("login"), sshPrivateKey: key }, initial).sshKey).toBeNull();
    expect(buildDraft({ ...created("sshKey"), sshPrivateKey: "  " }, initial).sshKey).toBeNull();
  });

  it("drops a new custom field with no name and no value and adds the rest", () => {
    const initial = initialForm(null, vaults, "v1");
    const form: Form = {
      ...initial,
      custom: [
        newCustomRow(),
        { ...newCustomRow(), label: "Door code", kind: "hidden", value: "1234" },
        { ...newCustomRow(), label: "Site", kind: "url" },
      ],
    };
    expect(buildDraft(form, initial).customFields).toEqual([
      { op: "add", label: "Door code", kind: "hidden", value: "1234" },
      { op: "add", label: "Site", kind: "url", value: "" },
    ]);
  });
});

const stored = detail("login", {
  text: [{ field: "username", value: "alex" }],
  secrets: ["password"],
  urls: [
    { elementId: "u1", url: "https://a.example", matchMode: "exactOrigin", allowHttp: true },
    { elementId: "u2", url: "https://b.example", matchMode: "exactOrigin", allowHttp: false },
  ],
  customFields: [
    { elementId: "c1", label: "Note", kind: "text", value: "hello" },
    { elementId: "c2", label: "PIN", kind: "hidden", value: null },
    { elementId: "c3", label: "Backup", kind: "otp", value: null },
    { elementId: "c4", label: "Old", kind: "text", value: "x" },
  ],
});

describe("buildDraft for an existing item", () => {
  it("sends only the title when nothing changed", () => {
    const initial = initialForm(stored, vaults, null);
    expect(buildDraft(initial, initial)).toEqual({
      text: [{ field: "title", value: "Item" }],
      secrets: [],
      urls: [],
      customFields: [],
      tags: ["a"],
      otp: null,
      sshKey: null,
    });
  });

  it("diffs custom fields: add, update, remove, and keep an untouched hidden value", () => {
    const initial = initialForm(stored, vaults, null);
    const form: Form = {
      ...initial,
      custom: [
        { ...must(initial.custom[0]), value: "changed" },
        { ...must(initial.custom[1]), label: "PIN code" },
        must(initial.custom[2]),
        { ...newCustomRow(), label: "New", value: "v" },
      ],
    };
    expect(buildDraft(form, initial).customFields).toEqual([
      { op: "update", elementId: "c1", label: "Note", kind: "text", value: "changed" },
      { op: "update", elementId: "c2", label: "PIN code", kind: "hidden", value: null },
      { op: "add", label: "New", kind: "text", value: "v" },
      { op: "remove", elementId: "c4" },
    ]);
  });

  it("sends a typed value for a hidden field", () => {
    const initial = initialForm(stored, vaults, null);
    const typed = { ...must(initial.custom[1]), value: "5678" };
    const rows = [must(initial.custom[0]), typed, must(initial.custom[2]), must(initial.custom[3])];
    expect(buildDraft({ ...initial, custom: rows }, initial).customFields).toEqual([
      { op: "update", elementId: "c2", label: "PIN", kind: "hidden", value: "5678" },
    ]);
  });

  it("renames a one-time code field without a value", () => {
    const initial = initialForm(stored, vaults, null);
    const renamed = { ...must(initial.custom[2]), label: "Recovery" };
    const rows = [must(initial.custom[0]), must(initial.custom[1]), renamed, must(initial.custom[3])];
    expect(buildDraft({ ...initial, custom: rows }, initial).customFields).toEqual([
      { op: "update", elementId: "c3", label: "Recovery", kind: "otp", value: null },
    ]);
  });

  it("diffs websites: update, add and remove, keeping allowHttp", () => {
    const initial = initialForm(stored, vaults, null);
    const form: Form = {
      ...initial,
      urls: [
        { ...must(initial.urls[0]), matchMode: "etld1" },
        { ...must(initial.urls[1]), url: "" },
        { rowKey: "n", elementId: null, url: "https://c.example", matchMode: "exactOrigin", allowHttp: false },
      ],
    };
    expect(buildDraft(form, initial).urls).toEqual([
      { op: "update", elementId: "u1", url: "https://a.example", matchMode: "etld1", allowHttp: true },
      { op: "add", url: "https://c.example", matchMode: "exactOrigin", allowHttp: false },
      { op: "remove", elementId: "u2" },
    ]);
  });

  it("sends null for a secret marked for removal and the typed value otherwise", () => {
    const initial = initialForm(stored, vaults, null);
    expect(buildDraft({ ...initial, clearSecrets: ["password"] }, initial).secrets).toEqual([
      { field: "password", value: null },
    ]);
    const typed = { ...initial, secrets: { password: "new" }, clearSecrets: ["password" as const] };
    expect(buildDraft(typed, initial).secrets).toEqual([{ field: "password", value: "new" }]);
  });

  it("clears a text field that was emptied", () => {
    const initial = initialForm(stored, vaults, null);
    const draft = buildDraft({ ...initial, text: { username: "" }, notes: "n" }, initial);
    expect(draft.text).toEqual([
      { field: "title", value: "Item" },
      { field: "notes", value: "n" },
      { field: "username", value: null },
    ]);
  });
});

describe("validateForm", () => {
  it("blocks an expiry that cannot be stored, but not an untouched stored one", () => {
    const card = detail("creditCard", { text: [{ field: "cardExpiry", value: "2029-04" }] });
    const initial = initialForm(card, vaults, null);
    expect(hasErrors(validateForm(initial, initial))).toBe(false);
    const typed = { ...initial, text: { cardExpiry: "13/29" } };
    expect(validateForm(typed, initial).cardExpiry).toBe(true);
    expect(validateForm({ ...initial, text: { cardExpiry: "0429" } }, initial).cardExpiry).toBe(false);
    expect(validateForm({ ...initial, text: { cardExpiry: "" } }, initial).cardExpiry).toBe(false);
  });

  it("requires a title for a new item and for one that had a title", () => {
    const fresh = initialForm(null, vaults, "v1");
    expect(validateForm(fresh, fresh).titleRequired).toBe(true);
    expect(hasErrors(validateForm(fresh, fresh))).toBe(true);
    expect(validateForm({ ...fresh, title: "   " }, fresh).titleRequired).toBe(true);
    expect(validateForm({ ...fresh, title: "Router" }, fresh).titleRequired).toBe(false);

    const initial = initialForm(detail("login"), vaults, null);
    expect(validateForm({ ...initial, title: " " }, initial).titleRequired).toBe(true);
  });

  it("lets an untitled import be saved without a title as long as it stays untitled", () => {
    const untitled = detail("login", { summary: { ...detail("login").summary, title: null } });
    const initial = initialForm(untitled, vaults, null);
    expect(validateForm(initial, initial).titleRequired).toBe(false);
  });

  it("checks custom field labels", () => {
    const initial = initialForm(stored, vaults, null);
    const rows = [
      { ...newCustomRow(), rowKey: "empty" },
      { ...newCustomRow(), rowKey: "noName", value: "x" },
      { ...newCustomRow(), rowKey: "long", label: "é".repeat(129) },
      { ...newCustomRow(), rowKey: "ok", label: "x".repeat(256) },
      { ...must(initial.custom[0]), label: " " },
    ];
    const errors = validateForm({ ...initial, custom: rows }, initial).custom;
    expect(errors).toEqual({ noName: "labelRequired", long: "labelTooLong", c1: "labelRequired" });
  });

  it("needs a value to change the kind of a stored hidden field", () => {
    const initial = initialForm(stored, vaults, null);
    const retyped = { ...must(initial.custom[1]), kind: "text" as const };
    const form = { ...initial, custom: [retyped] };
    expect(validateForm(form, initial).custom).toEqual({ c2: "kindNeedsValue" });
    const typed = { ...retyped, value: "now text" };
    expect(validateForm({ ...form, custom: [typed] }, initial).custom).toEqual({});
  });

  it("requires an SSH key on create only, and one that looks like an OpenSSH key", () => {
    const initial = initialForm(null, vaults, "v1");
    expect(validateForm(created("sshKey"), initial).sshKey).toBe("missing");
    expect(validateForm({ ...created("sshKey"), sshPrivateKey: "hunter2" }, initial).sshKey).toBe("shape");
    const key = "-----BEGIN OPENSSH PRIVATE KEY-----\nAAAA";
    expect(validateForm({ ...created("sshKey"), sshPrivateKey: key }, initial).sshKey).toBeNull();
    expect(validateForm(created("login"), initial).sshKey).toBeNull();
    const existing = initialForm(detail("sshKey"), vaults, null);
    expect(validateForm(existing, existing).sshKey).toBeNull();
  });
});

describe("custom field cap", () => {
  it("lets 511 fields grow to 512 and no further", () => {
    const initial = initialForm(null, vaults, "v1");
    const rows = (count: number) => Array.from({ length: count }, () => newCustomRow());
    expect(canAddCustomField({ ...initial, custom: rows(MAX_CUSTOM_FIELDS - 1) })).toBe(true);
    expect(canAddCustomField({ ...initial, custom: rows(MAX_CUSTOM_FIELDS) })).toBe(false);
  });
});
