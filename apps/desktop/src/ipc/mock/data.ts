/**
 * Sample data for the mock IPC. Illustrative only: the Secret Key is the one from the
 * design mockups (docs/design/README.md), the master password is `mosaic`.
 */
import type { ItemSummary, VaultSummary } from "../contract/index.ts";
import type { MockAccount, MockItem } from "./state.ts";

export const MOCK_PASSWORD = "mosaic";

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
    { vaultId: personal, name: "Personal", itemCount: 0, available: true },
    { vaultId: work, name: "Work", itemCount: 0, available: true },
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

function login(
  itemId: string,
  title: string,
  username: string,
  url: string,
  password: string,
  extra: Partial<ItemSummary> = {},
): MockItem {
  return {
    detail: {
      summary: summary({ itemId, title, subtitle: username, host: new URL(url).hostname, ...extra }),
      createdAt: 1_761_400_000_000,
      text: [{ field: "username", value: username }],
      secrets: ["password"],
      urls: [{ elementId: `${itemId}-url`, url, matchMode: "exactOrigin", allowHttp: false }],
      customFields: [],
      attachments: [],
      otp: extra.hasOtp === true
        ? { scheme: "totp", issuer: title, digits: 6, period: 30, hardwareProtected: false }
        : null,
      historyCount: 0,
      review: { conflictingFields: 0, deletedConcurrently: false, conflictCopyOf: null },
    },
    secrets: { password },
    history: [],
  };
}

export function sampleItems(): MockItem[] {
  return [
    login(
      "019a2b3c-4d5e-7020-8000-000000000001",
      "GitHub",
      "alex@example.com",
      "https://github.com/login",
      "correct-horse-battery-staple",
      { favorite: true, hasOtp: true, tags: ["dev"] },
    ),
    login(
      "019a2b3c-4d5e-7020-8000-000000000002",
      "Figma",
      "alex@example.com",
      "https://www.figma.com/login",
      "tulip-orbit-canyon-velvet",
    ),
    login(
      "019a2b3c-4d5e-7020-8000-000000000003",
      "Jira",
      "alex@acme.example",
      "https://acme.atlassian.net",
      "q7#Vt2!mZp9@Lx4$",
      { vaultId: work, tags: ["work"] },
    ),
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
      },
      secrets: { wifiPassword: "meadow-lantern-quiet-74" },
      history: [],
    },
  ];
}
