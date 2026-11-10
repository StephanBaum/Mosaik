/**
 * The field layout of each item kind: one source of truth for the editor and the detail.
 * Title, notes, tags and custom fields belong to every kind and are not listed here.
 */
import { secretFieldKeys } from "../../../ipc/contract/index.ts";
import type { ItemKind, SecretFieldKey, TextFieldKey } from "../../../ipc/contract/index.ts";

/** The kinds the editor can create (a passkey comes from a site, `unsupported` from a newer app). */
export type CreatableKind = Exclude<ItemKind, "passkey" | "unsupported">;

export type FieldInput =
  | "text"
  | "email"
  | "tel"
  | "url"
  | "date"
  | "cardNumber"
  | "expiry"
  | "digits"
  | "select"
  | "password";

export type FieldGroup = "name" | "contact" | "address" | "documents";

export interface FieldSpec {
  key: TextFieldKey | SecretFieldKey;
  /** Masked, revealed through `reveal_secret` and copied through Rust. */
  secret: boolean;
  input: FieldInput;
  group: FieldGroup | null;
  /** The detail offers a copy button. */
  copy: boolean;
  /** The editor offers "Generate". */
  generator: boolean;
  /** A label other than `item.field.<key>`. */
  labelKey: "item.field.sshPassphrase" | null;
}

interface Options {
  group?: FieldGroup;
  copy?: boolean;
  generator?: boolean;
}

function text(key: TextFieldKey, input: FieldInput = "text", options: Options = {}): FieldSpec {
  return {
    key,
    secret: false,
    input,
    group: options.group ?? null,
    copy: options.copy ?? false,
    generator: false,
    labelKey: null,
  };
}

function secret(key: SecretFieldKey, input: FieldInput = "password", options: Options = {}): FieldSpec {
  return {
    key,
    secret: true,
    input,
    group: options.group ?? null,
    copy: options.copy ?? false,
    generator: options.generator ?? false,
    labelKey: null,
  };
}

export const KIND_LAYOUT: Record<CreatableKind, readonly FieldSpec[]> = {
  login: [
    text("username", "text", { copy: true }),
    secret("password", "password", { copy: true, generator: true }),
    text("changePasswordUrl", "url", { copy: true }),
  ],
  totp: [],
  secureNote: [],
  creditCard: [
    text("cardholderName", "text", { copy: true }),
    secret("cardNumber", "cardNumber", { copy: true }),
    text("cardExpiry", "expiry"),
    secret("cardVerificationCode", "digits", { copy: true }),
    secret("cardPin", "digits"),
    text("cardBrand"),
  ],
  identity: [
    text("firstName", "text", { group: "name" }),
    text("middleName", "text", { group: "name" }),
    text("lastName", "text", { group: "name" }),
    text("birthDate", "date", { group: "name" }),
    text("email", "email", { group: "contact", copy: true }),
    text("phone", "tel", { group: "contact", copy: true }),
    text("company"),
    text("addressLine1", "text", { group: "address" }),
    text("addressLine2", "text", { group: "address" }),
    text("postalCode", "text", { group: "address" }),
    text("city", "text", { group: "address" }),
    text("region", "text", { group: "address" }),
    text("country", "text", { group: "address" }),
    secret("passportNumber", "password", { group: "documents", copy: true }),
    secret("licenseNumber", "password", { group: "documents", copy: true }),
    secret("nationalId", "password", { group: "documents", copy: true }),
  ],
  sshKey: [{ ...secret("password"), labelKey: "item.field.sshPassphrase" }, text("sshComment")],
  apiKey: [
    text("apiKeyId", "text", { copy: true }),
    secret("apiKeySecret", "password", { copy: true }),
    text("apiKeyHost"),
  ],
  wifi: [
    text("wifiSsid", "text", { copy: true }),
    secret("wifiPassword", "password", { copy: true, generator: true }),
    text("wifiSecurity", "select"),
  ],
  document: [],
};

/** The layout of a kind; passkeys and newer kinds have none (their fields list as "other"). */
export function layoutOf(kind: ItemKind): readonly FieldSpec[] {
  return kind === "passkey" || kind === "unsupported" ? [] : KIND_LAYOUT[kind];
}

export function hasUrls(kind: ItemKind): boolean {
  return kind === "login";
}

export function hasOtpSection(kind: ItemKind): boolean {
  return kind === "login" || kind === "totp";
}

export function hasSshKey(kind: ItemKind): boolean {
  return kind === "sshKey";
}

export function isSecretKey(key: string): key is SecretFieldKey {
  return (secretFieldKeys as readonly string[]).includes(key);
}

/** The choices of the Wi-Fi security select; the last is stored as `"None"`. */
export const WIFI_SECURITY = ["WPA3", "WPA2/WPA3", "WPA2", "WEP", "None"] as const;
