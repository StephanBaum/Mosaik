import { describe, expect, it } from "vitest";
import { createMockIpc } from "../mock.ts";
import { sampleItems, sampleVaults } from "./data.ts";
import { mockSecurityReport } from "./insights.ts";
import { defaultSettings } from "./settings.ts";
import type { MockItem, MockState } from "./state.ts";

const NOW = 1_800_000_000_000;

function state(items: MockItem[]): MockState {
  return {
    account: null,
    secretKeyStored: false,
    unlocked: true,
    vaults: sampleVaults(),
    items,
    settings: { ...defaultSettings },
    lastGenerated: null,
    emit: () => undefined,
    now: () => NOW,
    nextId: () => "019a2b3c-0000-7000-8000-000000000000",
  };
}

function variant(base: MockItem, itemId: string, password: string, url: string): MockItem {
  const copy = structuredClone(base);
  copy.detail.summary.itemId = itemId;
  copy.detail.summary.modifiedAt = NOW - 10 * 86_400_000;
  copy.detail.otp = null;
  copy.detail.urls = [{ elementId: `${itemId}-url`, url, matchMode: "exactOrigin", allowHttp: false }];
  copy.secrets = { password };
  return copy;
}

describe("mock security report", () => {
  it("needs an unlocked vault", async () => {
    const ipc = createMockIpc({ start: "locked" });
    expect(await ipc.call("security_report", null)).toEqual({
      ok: false,
      error: { code: "locked" },
    });
  });

  it("finds nothing wrong with the sample vault", async () => {
    // 30 days after the sample items were last modified.
    const ipc = createMockIpc({ start: "unlocked", now: () => 1_761_480_000_000 + 30 * 86_400_000 });
    const result = await ipc.call("security_report", null);
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value.itemsChecked).toBe(sampleItems().length);
      expect(result.value.findings.map((f) => f.kinds)).toEqual([["missingTotp"]]);
    }
  });

  it("reports each kind with partners and counts", () => {
    const [base] = sampleItems();
    if (base === undefined) {
      throw new Error("sample data");
    }
    const a = variant(base, "019a2b3c-0000-7000-8000-00000000000a", "short", "http://shop.example");
    const b = variant(base, "019a2b3c-0000-7000-8000-00000000000b", "short", "https://github.com");
    const old = variant(base, "019a2b3c-0000-7000-8000-00000000000c", "long-enough-pass", "http://localhost:8080");
    old.detail.summary.modifiedAt = NOW - 400 * 86_400_000;
    const trashed = variant(base, "019a2b3c-0000-7000-8000-00000000000d", "short", "http://x.example");
    trashed.detail.summary.trashedAt = NOW;

    const report = mockSecurityReport(state([a, b, old, trashed]));
    expect(report.itemsChecked).toBe(3);
    expect(report.counts).toEqual({ weak: 2, reused: 2, old: 1, missingTotp: 1, insecureUrl: 1 });
    expect(report.findings).toEqual([
      {
        vaultId: a.detail.summary.vaultId,
        itemId: a.detail.summary.itemId,
        kinds: ["weak", "reused", "insecureUrl"],
        reusedWith: [b.detail.summary.itemId],
      },
      {
        vaultId: b.detail.summary.vaultId,
        itemId: b.detail.summary.itemId,
        kinds: ["weak", "reused", "missingTotp"],
        reusedWith: [a.detail.summary.itemId],
      },
      {
        vaultId: old.detail.summary.vaultId,
        itemId: old.detail.summary.itemId,
        kinds: ["old"],
        reusedWith: [],
      },
    ]);
    expect(JSON.stringify(report)).not.toContain("short");
  });
});
