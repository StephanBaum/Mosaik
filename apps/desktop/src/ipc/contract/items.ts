/**
 * Item contract: mirrors `src-tauri/src/contract/items.rs`. Summaries and details never
 * contain a secret; `RevealedSecret.value` is the only secret here.
 */
import type { AttachmentId, ItemId, UnixMillis, VaultId } from "./common.ts";

export const itemKinds = [
  "login",
  "passkey",
  "totp",
  "secureNote",
  "creditCard",
  "identity",
  "sshKey",
  "apiKey",
  "wifi",
  "document",
  "unsupported",
] as const;
export type ItemKind = (typeof itemKinds)[number];

export const textFieldKeys = [
  "title",
  "notes",
  "username",
  "cardholderName",
  "cardBrand",
  "cardExpiry",
  "firstName",
  "middleName",
  "lastName",
  "email",
  "phone",
  "company",
  "addressLine1",
  "addressLine2",
  "city",
  "region",
  "postalCode",
  "country",
  "birthDate",
  "apiKeyId",
  "apiKeyHost",
  "wifiSsid",
  "wifiSecurity",
  "sshComment",
] as const;
export type TextFieldKey = (typeof textFieldKeys)[number];

export const secretFieldKeys = [
  "password",
  "cardNumber",
  "cardVerificationCode",
  "cardPin",
  "passportNumber",
  "licenseNumber",
  "nationalId",
  "apiKeySecret",
  "wifiPassword",
] as const;
export type SecretFieldKey = (typeof secretFieldKeys)[number];

export const trashFilters = ["exclude", "only", "include"] as const;
export type TrashFilter = (typeof trashFilters)[number];

export const urlMatchModes = ["exactOrigin", "etld1"] as const;
export type UrlMatchMode = (typeof urlMatchModes)[number];

export const customFieldKinds = ["text", "hidden", "url", "email", "phone", "date", "otp"] as const;
export type CustomFieldKind = (typeof customFieldKinds)[number];

export const otpSchemes = ["totp", "hotp", "steam"] as const;
export type OtpScheme = (typeof otpSchemes)[number];

export interface ListItemsRequest {
  vaultId: VaultId | null;
  trash: TrashFilter;
  favoritesOnly: boolean;
  /** Search over title, user name, email, URLs and tags; `null` or empty: no search. */
  query: string | null;
}

export interface ItemSummary {
  vaultId: VaultId;
  itemId: ItemId;
  kind: ItemKind;
  title: string | null;
  /** User name, cardholder, network name, ... Never derived from a secret. */
  subtitle: string | null;
  host: string | null;
  favorite: boolean;
  hasOtp: boolean;
  tags: string[];
  modifiedAt: UnixMillis | null;
  trashedAt: UnixMillis | null;
  /** A sync conflict, conflict copy or concurrent deletion waits for review. */
  needsReview: boolean;
}

export interface ItemRequest {
  vaultId: VaultId;
  itemId: ItemId;
}

export interface TextFieldEntry {
  field: TextFieldKey;
  value: string;
}

export interface UrlEntry {
  elementId: string;
  url: string;
  matchMode: UrlMatchMode;
  allowHttp: boolean;
}

export interface CustomFieldEntry {
  elementId: string;
  label: string;
  kind: CustomFieldKind;
  /** `null` for `hidden` and `otp`: reveal or copy instead. */
  value: string | null;
}

export interface AttachmentEntry {
  attachmentId: AttachmentId;
  fileName: string;
  mime: string;
  size: number;
}

export interface OtpSummary {
  scheme: OtpScheme;
  issuer: string | null;
  digits: number;
  period: number | null;
  hardwareProtected: boolean;
}

export interface ReviewState {
  conflictingFields: number;
  deletedConcurrently: boolean;
  conflictCopyOf: ItemId | null;
}

export interface ItemDetail {
  summary: ItemSummary;
  createdAt: UnixMillis | null;
  text: TextFieldEntry[];
  /** Secret fields that hold a value, masked: reveal or copy them through Rust. */
  secrets: SecretFieldKey[];
  urls: UrlEntry[];
  customFields: CustomFieldEntry[];
  attachments: AttachmentEntry[];
  otp: OtpSummary | null;
  historyCount: number;
  review: ReviewState;
}

export type SecretRef =
  | { kind: "field"; field: SecretFieldKey }
  | { kind: "customField"; elementId: string }
  | { kind: "history"; index: number };

export interface RevealRequest {
  vaultId: VaultId;
  itemId: ItemId;
  secret: SecretRef;
}

/** Hide it again after `hideAfterSeconds`, on blur and on lock (UX_GUIDELINES §8). */
export interface RevealedSecret {
  value: string;
  hideAfterSeconds: number;
}

export interface TextFieldChange {
  field: TextFieldKey;
  value: string | null;
}

export interface SecretFieldChange {
  field: SecretFieldKey;
  value: string | null;
}

export type UrlChange =
  | { op: "add"; url: string; matchMode: UrlMatchMode; allowHttp: boolean }
  | { op: "update"; elementId: string; url: string; matchMode: UrlMatchMode; allowHttp: boolean }
  | { op: "remove"; elementId: string };

export type CustomFieldChange =
  | { op: "add"; label: string; kind: CustomFieldKind; value: string }
  | { op: "update"; elementId: string; label: string; kind: CustomFieldKind; value: string }
  | { op: "remove"; elementId: string };

export type OtpChange = { op: "set"; uri: string } | { op: "remove" };

/** One edit: empty lists and `null` leave things unchanged. */
export interface ItemDraft {
  text: TextFieldChange[];
  secrets: SecretFieldChange[];
  urls: UrlChange[];
  customFields: CustomFieldChange[];
  tags: string[] | null;
  otp: OtpChange | null;
}

export interface AddItemRequest {
  vaultId: VaultId;
  kind: ItemKind;
  draft: ItemDraft;
}

export interface UpdateItemRequest {
  vaultId: VaultId;
  itemId: ItemId;
  draft: ItemDraft;
}

export interface SetFavoriteRequest {
  vaultId: VaultId;
  itemId: ItemId;
  favorite: boolean;
}

export interface MoveItemRequest {
  vaultId: VaultId;
  itemId: ItemId;
  targetVaultId: VaultId;
}

/** A move creates a new item ID. */
export interface MovedItem {
  vaultId: VaultId;
  itemId: ItemId;
}
