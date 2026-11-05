/**
 * Sample data for the mock IPC. Illustrative only: the Secret Key is the one from the
 * design mockups (docs/design/README.md), the master password is `mosaic`.
 */
import type {
  ItemKind,
  ItemSummary,
  SecretFieldKey,
  SshKeySummary,
  TextFieldEntry,
  VaultSummary,
} from "../contract/index.ts";
import type { MockAccount, MockAttachment, MockHardwareVault, MockItem } from "./state.ts";

export const MOCK_PASSWORD = "mosaic";

/** What the mock derives from any pasted OpenSSH key (the real derivation is Rust's). */
export const MOCK_SSH_SUMMARY: SshKeySummary = {
  publicKey: "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMockMockMockMockMockMockMockMockMockMock mock",
  fingerprint: "SHA256:mockmockmockmockmockmockmockmockmockmockmo",
  algorithm: "ssh-ed25519",
  encrypted: false,
  agentSupported: true,
};

export const sampleAccount: MockAccount = {
  accountId: "019a2b3c-4d5e-7030-8000-000000000001",
  email: "alex@example.com",
  password: MOCK_PASSWORD,
  secretKey: "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ2",
};

const personal = "019a2b3c-4d5e-7010-8000-000000000001";
const work = "019a2b3c-4d5e-7010-8000-000000000002";

export function sampleVaults(): VaultSummary[] {
  return [
    { vaultId: personal, name: "Personal", itemCount: 0, available: true, protection: "accountKey", openUntil: null },
    { vaultId: work, name: "Work", itemCount: 0, available: true, protection: "accountKey", openUntil: null },
  ];
}

function summary(fields: Partial<ItemSummary> & Pick<ItemSummary, "itemId" | "title">): ItemSummary {
  return {
    vaultId: personal,
    kind: "login",
    subtitle: null,
    host: null,
    favorite: false,
    hasOtp: false,
    tags: [],
    modifiedAt: 1_761_480_000_000,
    trashedAt: null,
    needsReview: false,
    ...fields,
  };
}

interface LoginSpec {
  itemId: string;
  title: string;
  username: string;
  url: string;
  password: string;
  extra?: Partial<ItemSummary>;
}

function login(spec: LoginSpec): MockItem {
  const { itemId, title, username, url, password } = spec;
  const extra = spec.extra ?? {};
  return {
    detail: {
      summary: summary({ itemId, title, subtitle: username, host: new URL(url).hostname, ...extra }),
      createdAt: 1_761_400_000_000,
      text: [{ field: "username", value: username }],
      secrets: ["password"],
      urls: [{ elementId: `${itemId}-url`, url, matchMode: "exactOrigin", allowHttp: false }],
      customFields: [],
      attachments: [],
      otp:
        extra.hasOtp === true
          ? { scheme: "totp", issuer: title, digits: 6, period: 30, hardwareProtected: false }
          : null,
      historyCount: 0,
      review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
      sshKey: null,
    },
    secrets: { password },
    history: [],
    customSecrets: {},
    sshKey: null,
  };
}

/** `count` generated logins, for long-list checks (`?items=5000`). */
export function bulkItems(count: number): MockItem[] {
  return Array.from({ length: count }, (_, index) => {
    const n = String(index + 1).padStart(5, "0");
    return login({
      itemId: `019a2b3c-4d5e-7020-9000-0000000${n}`,
      title: `Service ${n}`,
      username: `user${n}@example.com`,
      url: `https://service${n}.example.com`,
      password: `bulk-password-${n}`,
    });
  });
}

interface KindSpec {
  itemId: string;
  title: string;
  kind: ItemKind;
  subtitle: string | null;
  text: TextFieldEntry[];
  secrets: Partial<Record<SecretFieldKey, string>>;
  sshKey?: SshKeySummary;
}

function kindItem(spec: KindSpec): MockItem {
  const { itemId, title, kind, subtitle } = spec;
  return {
    detail: {
      summary: summary({ itemId, title, kind, subtitle }),
      createdAt: 1_761_400_000_000,
      text: spec.text,
      secrets: Object.keys(spec.secrets) as SecretFieldKey[],
      urls: [],
      customFields: [],
      attachments: [],
      otp: null,
      historyCount: 0,
      review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
      sshKey: spec.sshKey ?? null,
    },
    secrets: spec.secrets,
    history: [],
    customSecrets: {},
    sshKey: spec.sshKey === undefined ? null : { privateKey: "mock-private-key" },
  };
}

const PASSPORT_PNG = "019a2b3c-4d5e-7040-8000-000000000001";
const CONTRACT_PDF = "019a2b3c-4d5e-7040-8000-000000000002";

/** The mock attachments the sample document refers to. */
export function sampleAttachments(): Map<string, MockAttachment> {
  return new Map<string, MockAttachment>([
    [PASSPORT_PNG, { kind: "png", bytes: 2_400_000 }],
    [CONTRACT_PDF, { kind: "pdf", bytes: 310_000 }],
  ]);
}

/** A document with two files, one previewable and one not. */
function documentSample(): MockItem {
  const item = kindItem({
    itemId: "019a2b3c-4d5e-7020-8000-000000000008",
    title: "Passport scan",
    kind: "document",
    subtitle: null,
    text: [{ field: "notes", value: "Scans for the visa application." }],
    secrets: {},
  });
  item.detail.attachments = [
    {
      attachmentId: PASSPORT_PNG,
      fileName: "passport.png",
      mime: "image/png",
      size: 2_400_000,
      addedAt: 1_761_450_000_000,
      available: true,
      previewable: true,
    },
    {
      attachmentId: CONTRACT_PDF,
      fileName: "contract.pdf",
      mime: "application/pdf",
      size: 310_000,
      addedAt: 1_761_460_000_000,
      available: true,
      previewable: false,
    },
  ];
  return item;
}

/** A payment card, an identity and an SSH key, so every kind has a sample. */
function kindSamples(): MockItem[] {
  return [
    kindItem({
      itemId: "019a2b3c-4d5e-7020-8000-000000000005",
      title: "Visa Debit",
      kind: "creditCard",
      subtitle: "Alex Example",
      text: [
        { field: "cardholderName", value: "Alex Example" },
        { field: "cardExpiry", value: "04/2029" },
        { field: "cardBrand", value: "Visa" },
      ],
      secrets: { cardNumber: "4111111111111111", cardVerificationCode: "737" },
    }),
    kindItem({
      itemId: "019a2b3c-4d5e-7020-8000-000000000006",
      title: "Alex Example",
      kind: "identity",
      subtitle: "alex@example.com",
      text: [
        { field: "firstName", value: "Alex" },
        { field: "lastName", value: "Example" },
        { field: "email", value: "alex@example.com" },
        { field: "phone", value: "+49 30 1234567" },
        { field: "addressLine1", value: "Musterstrasse 1" },
        { field: "postalCode", value: "10115" },
        { field: "city", value: "Berlin" },
        { field: "country", value: "Germany" },
      ],
      secrets: { passportNumber: "C01X00T47" },
    }),
    kindItem({
      itemId: "019a2b3c-4d5e-7020-8000-000000000007",
      title: "Deploy key",
      kind: "sshKey",
      subtitle: "deploy@example.com",
      text: [{ field: "sshComment", value: "deploy@example.com" }],
      secrets: {},
      sshKey: MOCK_SSH_SUMMARY,
    }),
    documentSample(),
  ];
}

export function sampleItems(): MockItem[] {
  return [
    login({
      itemId: "019a2b3c-4d5e-7020-8000-000000000001",
      title: "GitHub",
      username: "alex@example.com",
      url: "https://github.com/login",
      password: "correct-horse-battery-staple",
      extra: { favorite: true, hasOtp: true, tags: ["dev"] },
    }),
    login({
      itemId: "019a2b3c-4d5e-7020-8000-000000000002",
      title: "Figma",
      username: "alex@example.com",
      url: "https://www.figma.com/login",
      password: "tulip-orbit-canyon-velvet",
    }),
    login({
      itemId: "019a2b3c-4d5e-7020-8000-000000000003",
      title: "Jira",
      username: "alex@acme.example",
      url: "https://acme.atlassian.net",
      password: "q7#Vt2!mZp9@Lx4$",
      extra: { vaultId: work, tags: ["work"] },
    }),
    {
      detail: {
        summary: summary({
          itemId: "019a2b3c-4d5e-7020-8000-000000000004",
          title: "Wi-Fi at home",
          kind: "wifi",
          subtitle: "Mosaic-5G",
        }),
        createdAt: 1_761_400_000_000,
        text: [
          { field: "wifiSsid", value: "Mosaic-5G" },
          { field: "wifiSecurity", value: "WPA3" },
        ],
        secrets: ["wifiPassword"],
        urls: [],
        customFields: [],
        attachments: [],
        otp: null,
        historyCount: 0,
        review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
        sshKey: null,
      },
      secrets: { wifiPassword: "meadow-lantern-quiet-74" },
      history: [],
      customSecrets: {},
      sshKey: null,
    },
    ...kindSamples(),
  ];
}

export const HARDWARE_VAULT_ID = "019a2b3c-4d5e-7010-8000-000000000003";

/** The key the mock device seeds (`?quick=hardwareKey`): a stable 32-hex method id. */
export function sampleKey(addedAt: number): { methodId: string; addedAt: number } {
  return { methodId: "0123456789abcdef0123456789abcdef", addedAt };
}

function codeItem(itemId: string, title: string, issuer: string): MockItem {
  return {
    detail: {
      summary: summary({
        itemId,
        vaultId: HARDWARE_VAULT_ID,
        title,
        kind: "totp",
        subtitle: issuer,
        hasOtp: true,
      }),
      createdAt: 1_761_400_000_000,
      text: [],
      secrets: [],
      urls: [],
      customFields: [],
      attachments: [],
      otp: { scheme: "totp", issuer, digits: 6, period: 30, hardwareProtected: true },
      historyCount: 0,
      review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
      sshKey: null,
    },
    secrets: {},
    history: [],
    customSecrets: {},
    sshKey: null,
  };
}

/** A hardware-protected vault with two one-time codes, closed at the start. */
export function sampleHardwareVault(addedAt: number): {
  vault: VaultSummary;
  items: MockItem[];
  hardware: MockHardwareVault;
} {
  return {
    vault: {
      vaultId: HARDWARE_VAULT_ID,
      name: "Codes (security key)",
      itemCount: 0,
      available: false,
      protection: "hardwareKey",
      openUntil: null,
    },
    items: [
      codeItem("019a2b3c-4d5e-7020-8000-000000000011", "Bank", "Example Bank"),
      codeItem("019a2b3c-4d5e-7020-8000-000000000012", "Cloud console", "Acme Cloud"),
    ],
    hardware: {
      vaultId: HARDWARE_VAULT_ID,
      name: "Codes (security key)",
      open: false,
      openUntil: null,
      keys: [sampleKey(addedAt)],
      timer: undefined,
    },
  };
}
