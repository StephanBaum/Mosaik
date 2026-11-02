import { describe, expect, it } from "vitest";
import type { ItemSummary } from "../../ipc/contract/index.ts";
import { categoryCounts, matchesCategory } from "./useVaultData.ts";

function item(patch: Partial<ItemSummary>): ItemSummary {
  return {
    vaultId: "v",
    itemId: "i",
    kind: "login",
    title: "T",
    subtitle: null,
    host: null,
    favorite: false,
    hasOtp: false,
    tags: [],
    modifiedAt: null,
    trashedAt: null,
    needsReview: false,
    ...patch,
  };
}

describe("matchesCategory", () => {
  it("matches by kind, flag and one-time code", () => {
    expect(matchesCategory(item({}), "all")).toBe(true);
    expect(matchesCategory(item({ favorite: true }), "favorites")).toBe(true);
    expect(matchesCategory(item({}), "favorites")).toBe(false);
    expect(matchesCategory(item({}), "logins")).toBe(true);
    expect(matchesCategory(item({ kind: "passkey" }), "passkeys")).toBe(true);
    expect(matchesCategory(item({ hasOtp: true }), "codes")).toBe(true);
    expect(matchesCategory(item({ kind: "totp" }), "codes")).toBe(true);
    expect(matchesCategory(item({ kind: "creditCard" }), "cards")).toBe(true);
    expect(matchesCategory(item({ kind: "secureNote" }), "notes")).toBe(true);
    expect(matchesCategory(item({ kind: "sshKey" }), "ssh")).toBe(true);
    expect(matchesCategory(item({ kind: "sshKey" }), "logins")).toBe(false);
  });

  it("counts every category", () => {
    const counts = categoryCounts([
      item({ favorite: true, hasOtp: true }),
      item({ kind: "secureNote" }),
    ]);
    expect(counts).toMatchObject({ all: 2, favorites: 1, logins: 1, codes: 1, notes: 1, ssh: 0 });
  });
});
