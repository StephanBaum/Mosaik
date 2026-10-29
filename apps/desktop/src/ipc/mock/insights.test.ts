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
      expect(result.value.findings.map((f) => f.issues.map((i) => i.kind))).toEqual([["missingTotp"]]);
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
    expect(report.counts).toEqual({
      weak: 2,
      reused: 2,
      reusedGroups: 1,
      old: 1,
      missingTotp: 1,
      insecureUrl: 1,
      high: 2,
      medium: 0,
      low: 1,
    });
    const ref = (item: MockItem) => ({
      vaultId: item.detail.summary.vaultId,
      itemId: item.detail.summary.itemId,
    });
    expect(report.findings).toEqual([
      {
        ...ref(a),
        issues: [
          { kind: "weak", severity: "high", action: "changePassword" },
          { kind: "reused", severity: "high", action: "changePassword" },
          { kind: "insecureUrl", severity: "medium", action: "reviewUrl" },
        ],
        reusedWith: [ref(b)],
        totpSite: null,
        insecureHost: "shop.example",
      },
      {
        ...ref(b),
        issues: [
          { kind: "weak", severity: "high", action: "changePassword" },
          { kind: "reused", severity: "high", action: "changePassword" },
          { kind: "missingTotp", severity: "medium", action: "setUpTotp" },
        ],
        reusedWith: [ref(a)],
        totpSite: { name: "GitHub", documentation: "https://docs.github.com/en/authentication" },
        insecureHost: null,
      },
      {
        ...ref(old),
        issues: [{ kind: "old", severity: "low", action: "changePassword" }],
        reusedWith: [],
        totpSite: null,
        insecureHost: null,
      },
    ]);
    expect(JSON.stringify(report)).not.toContain("short");
  });

  it("orders like the Rust report and rates the local network low", () => {
    const [base] = sampleItems();
    if (base === undefined) {
      throw new Error("sample data");
    }
    // Severity first, regardless of how many issues an item has.
    const router = variant(base, "019a2b3c-0000-7000-8000-000000000001", "long-enough-pass", "http://192.168.178.1");
    router.detail.summary.modifiedAt = NOW - 400 * 86_400_000;
    const weak = variant(base, "019a2b3c-0000-7000-8000-000000000002", "abc", "https://shop.example");
    const otpField = variant(base, "019a2b3c-0000-7000-8000-000000000003", "long-enough-pass2", "https://github.com");
    otpField.detail.customFields = [{ elementId: "f1", label: "2FA", kind: "otp", value: null }];

    const report = mockSecurityReport(state([router, weak, otpField]));
    expect(report.findings.map((f) => f.itemId)).toEqual([weak.detail.summary.itemId, router.detail.summary.itemId]);
    expect(report.findings[1]?.issues).toEqual([
      { kind: "old", severity: "low", action: "changePassword" },
      { kind: "insecureUrl", severity: "low", action: "reviewUrl" },
    ]);
    expect(report.findings[1]?.insecureHost).toBe("192.168.178.1");
    expect(report.counts.missingTotp).toBe(0);
    expect(report.counts.low).toBe(1);
    expect(report.counts.high).toBe(1);
  });
});
