import { describe, expect, it } from "vitest";
import type { MosaicIpc } from "../client.ts";
import type { AppEvent, CommandName, ErrorCode, RequestOf, ResponseOf } from "../contract/index.ts";
import { createMockIpc, type MockOptions } from "../mock.ts";

const VAULT = "019a2b3c-4d5e-7010-8000-000000000001";
const FIGMA = { vaultId: VAULT, itemId: "019a2b3c-4d5e-7020-8000-000000000002" };
const GITHUB = { vaultId: VAULT, itemId: "019a2b3c-4d5e-7020-8000-000000000001" };
const NEW_PASSWORD = "Xy7@mB3^dF6!hJ9#zC2$";

async function value<K extends CommandName>(ipc: MosaicIpc, command: K, request: RequestOf<K>): Promise<ResponseOf<K>> {
  const result = await ipc.call(command, request);
  if (!result.ok) {
    throw new Error(`${command} failed with ${result.error.code}`);
  }
  return result.value;
}

async function code<K extends CommandName>(ipc: MosaicIpc, command: K, request: RequestOf<K>): Promise<ErrorCode | "ok"> {
  const result = await ipc.call(command, request);
  return result.ok ? "ok" : result.error.code;
}

function mock(options: MockOptions = {}) {
  return createMockIpc({ start: "unlocked", credRotation: { variant: "queue", stepMs: 0 }, ...options });
}

const passwordOf = async (ipc: MosaicIpc, item: typeof FIGMA) =>
  (await value(ipc, "reveal_secret", { ...item, secret: { kind: "field", field: "password" } })).value;

describe("mock rotate-now queue", () => {
  it("lists weak, reused, old and breached items, and the one it cannot guide", async () => {
    const queue = await value(mock(), "rotation_queue", null);
    const byTitle = new Map(queue.entries.map((entry) => [entry.title, entry]));
    expect(byTitle.get("Figma")?.reasons).toEqual(["weak", "reused"]);
    expect(byTitle.get("Figma")?.kind).toBe("guided");
    expect(byTitle.get("GitHub")?.reasons).toEqual(["old", "breached"]);
    expect(byTitle.get("Wi-Fi at home")?.kind).toBe("manual");
    expect(queue.breach.mode).toBe("manual");
  });

  it("is empty for the empty variant and never carries a password", async () => {
    const ipc = mock({ credRotation: { variant: "empty", stepMs: 0 } });
    const queue = await value(ipc, "rotation_queue", null);
    expect(queue.entries).toEqual([]);
    expect(queue.breach).toMatchObject({ mode: "off", consentAt: null, running: false });
    const busy = await value(mock(), "rotation_queue", null);
    expect(JSON.stringify(busy)).not.toContain("Sommer2024");
  });

  it("snoozes (listed with the time) and dismisses (until the password changes)", async () => {
    const ipc = mock();
    await value(ipc, "rotation_snooze", { ...FIGMA, days: 7 });
    const snoozed = (await value(ipc, "rotation_queue", null)).entries.find((e) => e.title === "Figma");
    expect(snoozed?.snoozedUntil).toBeGreaterThan(Date.now());
    await value(ipc, "rotation_dismiss", FIGMA);
    expect((await value(ipc, "rotation_queue", null)).entries.some((e) => e.title === "Figma")).toBe(false);
    // A new password brings it back as soon as it is weak again.
    await value(ipc, "update_item", {
      ...FIGMA,
      draft: {
        text: [],
        secrets: [{ field: "password", value: "short" }],
        urls: [],
        customFields: [],
        tags: null,
        otp: null,
        sshKey: null,
      },
    });
    expect((await value(ipc, "rotation_queue", null)).entries.some((e) => e.title === "Figma")).toBe(true);
  });
});

describe("mock guided change", () => {
  it("stores the new password before anything else and keeps the current one", async () => {
    const ipc = mock();
    const before = await passwordOf(ipc, FIGMA);
    const begun = await value(ipc, "rotation_begin", { ...FIGMA, newPassword: NEW_PASSWORD, rulesSource: null });
    expect(begun.changeUrl).toEqual({ url: "https://www.figma.com/.well-known/change-password", source: "wellKnown" });
    expect(await passwordOf(ipc, FIGMA)).toBe(before);
    const detail = await value(ipc, "get_item", FIGMA);
    expect(detail.rotationAttempts).toHaveLength(1);
    expect(JSON.stringify(detail)).not.toContain(NEW_PASSWORD);
    const request = { ...FIGMA, attemptId: begun.attemptId };
    expect((await value(ipc, "rotation_reveal", request)).value).toBe(NEW_PASSWORD);
  });

  it("succeeded swaps the values and keeps the old one in the history", async () => {
    const ipc = mock();
    const before = await passwordOf(ipc, FIGMA);
    const { attemptId } = await value(ipc, "rotation_begin", { ...FIGMA, newPassword: NEW_PASSWORD, rulesSource: null });
    await value(ipc, "rotation_close", { ...FIGMA, attemptId, outcome: "succeeded" });
    expect(await passwordOf(ipc, FIGMA)).toBe(NEW_PASSWORD);
    const detail = await value(ipc, "get_item", FIGMA);
    expect(detail.rotationAttempts).toEqual([]);
    expect(detail.history).toMatchObject([{ index: 0, reason: "replaced" }]);
    const old = await value(ipc, "reveal_secret", { ...FIGMA, secret: { kind: "history", index: 0 } });
    expect(old.value).toBe(before);
  });

  it("failed keeps the current password and the value as not confirmed, which Restore brings back", async () => {
    const ipc = mock();
    const before = await passwordOf(ipc, FIGMA);
    const { attemptId } = await value(ipc, "rotation_begin", { ...FIGMA, newPassword: NEW_PASSWORD, rulesSource: null });
    await value(ipc, "rotation_close", { ...FIGMA, attemptId, outcome: "failed" });
    expect(await passwordOf(ipc, FIGMA)).toBe(before);
    const detail = await value(ipc, "get_item", FIGMA);
    expect(detail.history[0]?.reason).toBe("unconfirmed");
    await value(ipc, "password_history_restore", { ...FIGMA, historyId: detail.history[0]?.historyId ?? "" });
    expect(await passwordOf(ipc, FIGMA)).toBe(NEW_PASSWORD);
    // The value that was the password is in the history now.
    const after = await value(ipc, "get_item", FIGMA);
    expect(after.history).toHaveLength(1);
    expect(after.history[0]?.reason).toBe("replaced");
  });

  it("refuses what Rust refuses", async () => {
    const ipc = mock();
    const note = (await value(ipc, "list_items", { vaultId: null, trash: "exclude", favoritesOnly: false, query: null })).find(
      (item) => item.kind === "secureNote" || item.kind === "creditCard",
    );
    expect(await code(ipc, "rotation_begin", { vaultId: VAULT, itemId: note?.itemId ?? "x", newPassword: "a", rulesSource: null })).toBe(
      "rotationNotSupported",
    );
    expect(await code(ipc, "rotation_begin", { ...FIGMA, newPassword: "", rulesSource: null })).toBe("invalidInput");
    for (let n = 0; n < 4; n += 1) {
      await value(ipc, "rotation_begin", { ...FIGMA, newPassword: `${NEW_PASSWORD}${String(n)}`, rulesSource: null });
    }
    expect(await code(ipc, "rotation_begin", { ...FIGMA, newPassword: "one more", rulesSource: null })).toBe("rotationLimit");
    expect(await code(ipc, "rotation_close", { ...FIGMA, attemptId: "nope", outcome: "failed" })).toBe("rotationNotFound");
    expect(await code(ipc, "password_history_restore", { ...FIGMA, historyId: "nope" })).toBe("historyNotFound");
  });

  it("shows an open change on the pending variant, with a not-confirmed entry", async () => {
    const ipc = mock({ credRotation: { variant: "pending", stepMs: 0 } });
    const detail = await value(ipc, "get_item", GITHUB);
    expect(detail.rotationAttempts[0]?.stage).toBe("delivered");
    expect(detail.history[0]?.reason).toBe("unconfirmed");
    const queue = await value(ipc, "rotation_queue", null);
    expect(queue.entries.find((e) => e.title === "GitHub")?.reasons).toContain("unfinished");
  });
});

describe("mock breach check", () => {
  function events(ipc: MosaicIpc): AppEvent[] {
    const seen: AppEvent[] = [];
    void ipc.subscribe((event) => {
      seen.push(event);
    });
    return seen;
  }

  it("needs the consent first and sends nothing until then", async () => {
    const ipc = mock({ credRotation: { variant: "consent", stepMs: 0 } });
    expect(await code(ipc, "breach_scan_start", null)).toBe("breachConsentRequired");
    const settings = await value(ipc, "get_settings", null);
    expect(settings).toMatchObject({ breachCheck: "off", breachConsentAt: null, rotationReminders: true });
    // A mode without the consent reads as off; with it, it is kept.
    expect((await value(ipc, "update_settings", { settings: { ...settings, breachCheck: "weekly" }, password: null })).breachCheck).toBe("off");
    const agreed = await value(ipc, "update_settings", {
      settings: { ...settings, breachCheck: "manual", breachConsentAt: 1_000 },
      password: null,
    });
    expect(agreed).toMatchObject({ breachCheck: "manual", breachConsentAt: 1_000 });
  });

  it("scans, finds a known password and clears the finding when the password changes", async () => {
    const ipc = mock({ credRotation: { variant: "consent", stepMs: 0 } });
    const seen = events(ipc);
    const settings = await value(ipc, "get_settings", null);
    await value(ipc, "update_settings", { settings: { ...settings, breachCheck: "manual", breachConsentAt: 5 }, password: null });
    await value(ipc, "update_item", {
      ...FIGMA,
      draft: { text: [], secrets: [{ field: "password", value: "hunter2" }], urls: [], customFields: [], tags: null, otp: null, sshKey: null },
    });
    await value(ipc, "breach_scan_start", null);
    expect(await code(ipc, "breach_scan_start", null)).toBe("breachRunning");
    await new Promise((resolve) => setTimeout(resolve, 60));
    const finished = seen.find((event) => event.type === "breachScanFinished");
    // Figma's new password and Jira's "Sommer2024!" are both on the mock's list.
    expect(finished).toMatchObject({ type: "breachScanFinished", found: 2, error: null });
    expect((await value(ipc, "security_report", null)).counts.breached).toBe(2);

    const { attemptId } = await value(ipc, "rotation_begin", { ...FIGMA, newPassword: NEW_PASSWORD, rulesSource: null });
    await value(ipc, "rotation_close", { ...FIGMA, attemptId, outcome: "succeeded" });
    expect((await value(ipc, "security_report", null)).counts.breached).toBe(1);
  });

  it("stops when cancelled and reports the offline variant", async () => {
    const ipc = mock({ credRotation: { variant: "breachOffline", stepMs: 1_000 } });
    const seen = events(ipc);
    await value(ipc, "breach_scan_start", null);
    await value(ipc, "breach_scan_cancel", null);
    expect(seen.at(-1)).toMatchObject({ type: "breachScanFinished", error: "cancelled" });
    expect((await value(ipc, "rotation_queue", null)).breach.lastError).toBe("cancelled");
  });
});
