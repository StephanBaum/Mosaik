import { describe, expect, it } from "vitest";
import type { ItemSummary } from "../../ipc/contract/index.ts";
import { applySaved, categoryCounts, matchesCategory, type FetchedList } from "./useVaultData.ts";

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

describe("applySaved", () => {
  const list = (items: ItemSummary[]): FetchedList => ({ items, vaultId: null, unfiltered: true });
  const all = { vaultId: null, trash: false, query: "" };
  const a = item({ itemId: "a", title: "A" });
  const c = item({ itemId: "c", title: "C" });

  it("replaces an edited row where it is", () => {
    const edited = item({ itemId: "a", title: "A2" });
    expect(applySaved(list([a, c]), edited, all)?.items).toEqual([edited, c]);
  });

  it("inserts a new item at its place by ID", () => {
    const added = item({ itemId: "b", title: "B" });
    expect(applySaved(list([a, c]), added, all)?.items).toEqual([a, added, c]);
    const last = item({ itemId: "d", title: "D" });
    expect(applySaved(list([a, c]), last, all)?.items).toEqual([a, c, last]);
  });

  it("drops a row that left the list (trashed, or another vault)", () => {
    expect(applySaved(list([a, c]), item({ itemId: "a", trashedAt: 1 }), all)?.items).toEqual([c]);
    const inVault = { ...list([a, c]), vaultId: "v" };
    const moved = item({ vaultId: "w", itemId: "a" });
    expect(applySaved(inVault, moved, { ...all, vaultId: "v" })?.items).toEqual([c]);
  });

  it("gives up (the caller loads again) for a search, the trash or no list", () => {
    expect(applySaved(list([a]), a, { ...all, query: "x" })).toBeNull();
    expect(applySaved(list([a]), a, { ...all, trash: true })).toBeNull();
    expect(applySaved(null, a, all)).toBeNull();
    expect(applySaved({ ...list([a]), unfiltered: false }, a, all)).toBeNull();
  });
});
