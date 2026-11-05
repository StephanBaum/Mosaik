/**
 * The item editor's form: its state shape, how it is checked and the draft sent to Rust.
 * Only the fields in the selected kind's layout are sent (plus title, notes, tags and
 * custom fields), and only what changed against the form the editor started with.
 */
import type {
  CustomFieldChange,
  CustomFieldKind,
  ItemDetail,
  ItemDraft,
  ItemKind,
  SecretFieldKey,
  TextFieldChange,
  TextFieldKey,
  UrlChange,
  UrlMatchMode,
  VaultSummary,
} from "../../ipc/contract/index.ts";
import { OTP_UNCHANGED, type OtpFormValue } from "./otp/OtpSection.tsx";
import { normalizeCardNumber, normalizeExpiry } from "./kinds/cardFormat.ts";
import { hasSshKey, hasUrls, layoutOf } from "./kinds/layouts.ts";

/** Most custom fields an item can have (`MAX_DRAFT_ENTRIES` in Rust). */
export const MAX_CUSTOM_FIELDS = 512;
/** Longest custom-field label, in bytes (`MAX_LABEL_LEN` in Rust). */
export const MAX_LABEL_BYTES = 256;
/** What an OpenSSH private key starts with; Rust decides in the end. */
export const SSH_KEY_HEADER = "-----BEGIN OPENSSH PRIVATE KEY-----";

export interface UrlRow {
  rowKey: string;
  /** `null` for a website added in this editor. */
  elementId: string | null;
  url: string;
  matchMode: UrlMatchMode;
  allowHttp: boolean;
}

export interface CustomRow {
  rowKey: string;
  /** `null` for a field added in this editor. */
  elementId: string | null;
  label: string;
  kind: CustomFieldKind;
  /** Empty for a stored hidden or one-time-code field: its value stays in Rust. */
  value: string;
  /** The value lives in Rust (a stored hidden or one-time-code field); empty means "keep". */
  stored: boolean;
  /** The kind the field has in the vault (`null` for a new field). */
  storedKind: CustomFieldKind | null;
}

export interface Form {
  /** A new item (the kind and vault can still be chosen). */
  creating: boolean;
  kind: ItemKind;
  vault: string;
  title: string;
  notes: string;
  tags: string;
  text: Partial<Record<TextFieldKey, string>>;
  /** Typed secrets; an empty one means "keep the stored value". */
  secrets: Partial<Record<SecretFieldKey, string>>;
  /** Stored secrets to remove on save. */
  clearSecrets: SecretFieldKey[];
  urls: UrlRow[];
  custom: CustomRow[];
  sshPrivateKey: string;
  /** What to do with the one-time code (ADR-0031). */
  otp: OtpFormValue;
}

export type CustomRowError = "labelRequired" | "labelTooLong" | "kindNeedsValue";

export interface FormErrors {
  /** A new item, or one that had a title, needs one (an untitled import may stay as it is). */
  titleRequired: boolean;
  cardExpiry: boolean;
  sshKey: "missing" | "shape" | null;
  custom: Record<string, CustomRowError>;
}

export function newUrlRow(): UrlRow {
  return {
    rowKey: crypto.randomUUID(),
    elementId: null,
    url: "",
    matchMode: "exactOrigin",
    allowHttp: false,
  };
}

export function newCustomRow(): CustomRow {
  return {
    rowKey: crypto.randomUUID(),
    elementId: null,
    label: "",
    kind: "text",
    value: "",
    stored: false,
    storedKind: null,
  };
}

function storedText(editing: ItemDetail | null): Form["text"] {
  const text: Form["text"] = {};
  for (const entry of editing?.text ?? []) {
    if (entry.field !== "title" && entry.field !== "notes") {
      text[entry.field] = entry.value;
    }
  }
  return text;
}

function storedUrls(editing: ItemDetail | null): UrlRow[] {
  if (editing === null) {
    return [newUrlRow()];
  }
  return editing.urls.map((entry) => ({
    rowKey: entry.elementId,
    elementId: entry.elementId,
    url: entry.url,
    matchMode: entry.matchMode,
    allowHttp: entry.allowHttp,
  }));
}

function storedCustom(editing: ItemDetail | null): CustomRow[] {
  return (editing?.customFields ?? []).map((field) => ({
    rowKey: field.elementId,
    elementId: field.elementId,
    label: field.label,
    kind: field.kind,
    value: field.value ?? "",
    stored: field.kind === "hidden" || field.kind === "otp",
    storedKind: field.kind,
  }));
}

function storedMeta(editing: ItemDetail | null): Pick<Form, "title" | "notes" | "tags"> {
  return {
    title: editing?.summary.title ?? "",
    notes: editing?.text.find((entry) => entry.field === "notes")?.value ?? "",
    tags: editing?.summary.tags.join(", ") ?? "",
  };
}

export function initialForm(
  editing: ItemDetail | null,
  vaults: readonly VaultSummary[],
  vaultId: string | null,
): Form {
  return {
    creating: editing === null,
    kind: editing?.summary.kind ?? "login",
    vault: vaultId ?? vaults[0]?.vaultId ?? "",
    ...storedMeta(editing),
    text: storedText(editing),
    secrets: {},
    clearSecrets: [],
    urls: storedUrls(editing),
    custom: storedCustom(editing),
    sshPrivateKey: "",
    otp: OTP_UNCHANGED,
  };
}

/** The form after the user picks another kind (a new item only). Typed values stay in the form. */
export function changeKind(form: Form, kind: ItemKind): Form {
  const text =
    kind === "wifi" && (form.text.wifiSecurity ?? "") === ""
      ? { ...form.text, wifiSecurity: "WPA2" }
      : form.text;
  return { ...form, kind, text };
}

function emptyToNull(value: string): string | null {
  return value === "" ? null : value;
}

function textChanges(form: Form, initial: Form): TextFieldChange[] {
  const changes: TextFieldChange[] = [{ field: "title", value: form.title }];
  if (form.notes !== initial.notes) {
    changes.push({ field: "notes", value: emptyToNull(form.notes) });
  }
  for (const spec of layoutOf(form.kind)) {
    if (spec.secret) {
      continue;
    }
    const field = spec.key as TextFieldKey;
    const typed = form.text[field] ?? "";
    const value = field === "cardExpiry" ? (normalizeExpiry(typed) ?? typed) : typed;
    if (value !== (initial.text[field] ?? "")) {
      changes.push({ field, value: emptyToNull(value) });
    }
  }
  return changes;
}

function secretChanges(form: Form): ItemDraft["secrets"] {
  const changes: ItemDraft["secrets"] = [];
  for (const spec of layoutOf(form.kind)) {
    if (!spec.secret) {
      continue;
    }
    const field = spec.key as SecretFieldKey;
    const typed = form.secrets[field] ?? "";
    if (typed !== "") {
      changes.push({ field, value: field === "cardNumber" ? normalizeCardNumber(typed) : typed });
    } else if (form.clearSecrets.includes(field)) {
      changes.push({ field, value: null });
    }
  }
  return changes;
}

function urlChange(row: UrlRow, before: UrlRow | undefined): UrlChange | null {
  const url = row.url.trim();
  if (row.elementId === null) {
    return url === "" ? null : { op: "add", url, matchMode: row.matchMode, allowHttp: row.allowHttp };
  }
  const changed =
    before !== undefined &&
    (url !== before.url || row.matchMode !== before.matchMode || row.allowHttp !== before.allowHttp);
  return changed
    ? { op: "update", elementId: row.elementId, url, matchMode: row.matchMode, allowHttp: row.allowHttp }
    : null;
}

function urlChanges(form: Form, initial: Form): UrlChange[] {
  if (!hasUrls(form.kind)) {
    return [];
  }
  const changes: UrlChange[] = [];
  const kept = new Set<string>();
  for (const row of form.urls) {
    // An emptied stored website is removed below.
    if (row.elementId !== null && row.url.trim() !== "") {
      kept.add(row.elementId);
    }
    const change = urlChange(row, initial.urls.find((each) => each.elementId === row.elementId));
    if (change !== null && (row.elementId === null || row.url.trim() !== "")) {
      changes.push(change);
    }
  }
  for (const before of initial.urls) {
    if (before.elementId !== null && !kept.has(before.elementId)) {
      changes.push({ op: "remove", elementId: before.elementId });
    }
  }
  return changes;
}

function customChange(row: CustomRow, before: CustomRow | undefined): CustomFieldChange | null {
  const label = row.label.trim();
  if (row.elementId === null) {
    return label === "" && row.value === "" ? null : { op: "add", label, kind: row.kind, value: row.value };
  }
  const unchanged =
    before === undefined ||
    (row.label === before.label && row.kind === before.kind && row.value === before.value);
  if (unchanged) {
    return null;
  }
  const keep = row.stored && row.value === "" && row.kind === row.storedKind;
  return {
    op: "update",
    elementId: row.elementId,
    label,
    kind: row.kind,
    value: keep ? null : row.value,
  };
}

function customChanges(form: Form, initial: Form): CustomFieldChange[] {
  const changes: CustomFieldChange[] = [];
  const kept = new Set<string>();
  for (const row of form.custom) {
    if (row.elementId !== null) {
      kept.add(row.elementId);
    }
    const change = customChange(row, initial.custom.find((each) => each.elementId === row.elementId));
    if (change !== null) {
      changes.push(change);
    }
  }
  for (const before of initial.custom) {
    if (before.elementId !== null && !kept.has(before.elementId)) {
      changes.push({ op: "remove", elementId: before.elementId });
    }
  }
  return changes;
}

/** What the editor sends for `otp`; a code kept in a hardware vault is saved separately. */
function otpChange(value: OtpFormValue): ItemDraft["otp"] {
  if (value.kind === "remove") {
    return { op: "remove" };
  }
  if (value.kind === "pending" && value.result.target.kind === "item") {
    const { pending, issuer, accountName } = value.result;
    return { op: "setPending", pendingId: pending.pendingId, issuer, accountName };
  }
  return null;
}

/** The draft for `add_item` or `update_item`: what differs from `initial`, for this kind only. */
export function buildDraft(form: Form, initial: Form): ItemDraft {
  const sshKey = form.sshPrivateKey.trim();
  return {
    text: textChanges(form, initial),
    secrets: secretChanges(form),
    urls: urlChanges(form, initial),
    customFields: customChanges(form, initial),
    tags: form.tags
      .split(",")
      .map((tag) => tag.trim())
      .filter((tag) => tag !== ""),
    otp: otpChange(form.otp),
    sshKey: hasSshKey(form.kind) && sshKey !== "" ? { op: "set", privateKey: form.sshPrivateKey } : null,
  };
}

// eslint-disable-next-line no-control-regex -- C0 and C1 control characters are refused in labels.
const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;

function customRowError(row: CustomRow): CustomRowError | null {
  const label = row.label.trim();
  if (row.elementId === null && label === "" && row.value === "") {
    return null; // Dropped silently.
  }
  if (label === "" || CONTROL.test(label)) {
    return "labelRequired";
  }
  if (new TextEncoder().encode(label).length > MAX_LABEL_BYTES) {
    return "labelTooLong";
  }
  return row.stored && row.kind !== row.storedKind && row.value === "" ? "kindNeedsValue" : null;
}

function sshKeyError(form: Form): FormErrors["sshKey"] {
  if (!hasSshKey(form.kind)) {
    return null;
  }
  const key = form.sshPrivateKey.trim();
  if (key === "") {
    return form.creating ? "missing" : null;
  }
  return key.includes(SSH_KEY_HEADER) ? null : "shape";
}

/** What blocks saving. Hints (card number, email) never do. */
export function validateForm(form: Form, initial: Form): FormErrors {
  const expiry = form.text.cardExpiry ?? "";
  const legacy = expiry === (initial.text.cardExpiry ?? "");
  const custom: FormErrors["custom"] = {};
  for (const row of form.custom) {
    const error = customRowError(row);
    if (error !== null) {
      custom[row.rowKey] = error;
    }
  }
  return {
    titleRequired: form.title.trim() === "" && (form.creating || initial.title.trim() !== ""),
    cardExpiry:
      layoutOf(form.kind).some((spec) => spec.key === "cardExpiry") &&
      expiry.trim() !== "" &&
      !legacy &&
      normalizeExpiry(expiry) === null,
    sshKey: sshKeyError(form),
    custom,
  };
}

export function hasErrors(errors: FormErrors): boolean {
  return errors.titleRequired || errors.cardExpiry || errors.sshKey !== null || Object.keys(errors.custom).length > 0;
}

/** Whether another custom field fits. */
export function canAddCustomField(form: Form): boolean {
  return form.custom.length < MAX_CUSTOM_FIELDS;
}
