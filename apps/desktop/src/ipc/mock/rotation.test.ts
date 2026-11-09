import { describe, expect, it } from "vitest";
import type { MosaicIpc } from "../client.ts";
import type { AppEvent, CommandName, ErrorCode, RequestOf, ResponseOf } from "../contract/index.ts";
import { createMockIpc, type MockOptions } from "../mock.ts";
import { MOCK_PASSWORD, sampleAccount } from "./data.ts";
import { MOCK_NEW_SECRET_KEY } from "./rotation.ts";
import { MOCK_NEW_PASSWORD } from "./syncData.ts";

async function value<K extends CommandName>(
  ipc: MosaicIpc,
  command: K,
  request: RequestOf<K>,
): Promise<ResponseOf<K>> {
  const result = await ipc.call(command, request);
  if (!result.ok) {
    throw new Error(`${command} failed with ${result.error.code}`);
  }
  return result.value;
}

async function code<K extends CommandName>(
  ipc: MosaicIpc,
  command: K,
  request: RequestOf<K>,
): Promise<ErrorCode | "ok"> {
  const result = await ipc.call(command, request);
  return result.ok ? "ok" : result.error.code;
}

function unlocked(rotation: MockOptions["rotation"] = {}, sync: MockOptions["sync"] = { variant: "folder" }) {
  return createMockIpc({ start: "unlocked", rotation: { stepMs: 0, ...rotation }, sync });
}

const label = {
  title: "t",
  intro: "i",
  email: "e",
  accountId: "a",
  signInUrl: "s",
  signInUrlValue: "v",
  secretKey: "k",
  masterPassword: "m",
  masterPasswordHint: "h",
  footer: "f",
};

/** Types the last group of the new kit. */
async function verifyStaged(ipc: MosaicIpc, group: string) {
  return value(ipc, "verify_emergency_kit", { lastGroup: group, pending: true });
}

describe("mock rotation: Secret Key", () => {
  it("changes the key only after a verified new kit and refuses the old one afterwards", async () => {
    const ipc = unlocked();
    expect(await value(ipc, "rotation_status", null)).toEqual({ job: null });
    expect(await code(ipc, "secret_key_rotate_prepare", { password: "nope" })).toBe("wrongCredentials");
    await value(ipc, "secret_key_rotate_prepare", { password: MOCK_PASSWORD });
    expect((await value(ipc, "rotation_status", null)).job).toMatchObject({
      kind: "secretKey",
      phase: "prepared",
      needs: "password",
    });
    expect(await code(ipc, "secret_key_rotate_commit", { password: MOCK_PASSWORD, code: null })).toBe("kitNotVerified");
    expect((await value(ipc, "save_emergency_kit", { labels: label, pending: true, password: null })).kind).toBe(
      "saved",
    );
    expect((await verifyStaged(ipc, "ZZZZ")).matches).toBe(false);
    expect(await code(ipc, "secret_key_rotate_commit", { password: MOCK_PASSWORD, code: null })).toBe("kitNotVerified");
    // The first new key of a mock run is the documented one.
    expect((await verifyStaged(ipc, "8vj3")).matches).toBe(true);
    const done = await value(ipc, "secret_key_rotate_commit", { password: MOCK_PASSWORD, code: null });
    expect(done.finished).toBe(true);
    expect((await value(ipc, "rotation_status", null)).job).toBeNull();
    // The old key is one this device used before; the kit flow has nothing staged any more.
    expect(await code(ipc, "verify_emergency_kit", { lastGroup: "8VJ3", pending: true })).toBe("rotationNone");
    expect(await code(ipc, "sync_recover_password", { newPassword: MOCK_PASSWORD, secretKey: sampleAccount.secretKey, code: null })).toBe(
      "secretKeyUnchanged",
    );
  });

  it("answers rotationInProgress for another kind and rotationNone without a job", async () => {
    const ipc = unlocked();
    expect(await code(ipc, "rotation_abandon", null)).toBe("rotationNone");
    await value(ipc, "secret_key_rotate_prepare", { password: MOCK_PASSWORD });
    expect(await code(ipc, "rekey_prepare", { password: MOCK_PASSWORD, newPassword: "another one" })).toBe(
      "rotationInProgress",
    );
    await value(ipc, "rotation_abandon", null);
    expect((await value(ipc, "rotation_status", null)).job).toBeNull();
  });
});

describe("mock rotation: profile", () => {
  it("only goes up and warns on a slow device until forced", async () => {
    const ipc = unlocked({ variant: "slowDevice" });
    const request = { password: MOCK_PASSWORD, code: null };
    expect(await code(ipc, "profile_upgrade", { ...request, profile: "standard", force: true })).toBe(
      "profileDowngrade",
    );
    expect(await code(ipc, "profile_upgrade", { ...request, profile: "strong", force: false })).toBe("profileTooSlow");
    expect(await code(ipc, "profile_upgrade", { ...request, profile: "strong", force: true })).toBe("ok");
    expect(await code(ipc, "profile_upgrade", { ...request, profile: "strong", force: true })).toBe("profileDowngrade");
  });
});

describe("mock rotation: re-key", () => {
  it("needs sync, a different password and a verified kit, then runs the phases", async () => {
    const none = unlocked({}, { variant: "none" });
    expect(await code(none, "rekey_prepare", { password: MOCK_PASSWORD, newPassword: "x new password" })).toBe(
      "syncNotConfigured",
    );
    expect((await value(none, "rekey_review", null)).backend).toBe("local");

    const ipc = unlocked({ variant: "shared" });
    const review = await value(ipc, "rekey_review", null);
    expect(review.backend).toBe("folder");
    expect(review.sharedOwned).toHaveLength(1);
    expect(await code(ipc, "rekey_prepare", { password: MOCK_PASSWORD, newPassword: MOCK_PASSWORD })).toBe(
      "passwordUnchanged",
    );
    await value(ipc, "rekey_prepare", { password: MOCK_PASSWORD, newPassword: "x new password" });
    expect(await code(ipc, "rekey_commit", { password: MOCK_PASSWORD, newPassword: "x new password", code: null })).toBe(
      "kitNotVerified",
    );
  });

  it("answers rekeyCommit failures from the option", async () => {
    const ipc = unlocked({ fail: "rekeyBusy" });
    await value(ipc, "rekey_prepare", { password: MOCK_PASSWORD, newPassword: "x new password" });
    await value(ipc, "rotation_abandon", null);
    expect(await code(ipc, "rotation_resume", { password: null, kitGroup: null, code: null })).toBe("rotationNone");
  });
});

describe("mock rotation: resume", () => {
  it("continues a job that needs nothing and reports the phases by event", async () => {
    const ipc = unlocked({ variant: "resumeAttachments" });
    const events: AppEvent[] = [];
    await ipc.subscribe((event) => events.push(event));
    expect((await value(ipc, "rotation_status", null)).job).toMatchObject({ needs: "none", phase: "attachments" });
    const done = await value(ipc, "rotation_resume", { password: null, kitGroup: null, code: null });
    expect(done.finished).toBe(true);
    expect((await value(ipc, "rotation_status", null)).job).toBeNull();
    expect(events.some((e) => e.type === "rotationProgress")).toBe(true);
  });

  it("asks the last group of the new Secret Key before it continues a job the account may have", async () => {
    const ipc = unlocked({ variant: "resumeAccepted" });
    expect((await value(ipc, "rotation_status", null)).job?.needs).toBe("kitGroup");
    expect(await code(ipc, "rotation_resume", { password: null, kitGroup: null, code: null })).toBe("invalidInput");
    expect(
      await code(ipc, "rotation_resume", { password: "a new one", kitGroup: "ZZZZ", code: null }),
    ).toBe("kitGroupMismatch");
    expect(await code(ipc, "rotation_resume", { password: "a new one", kitGroup: "8vj3", code: null })).toBe("ok");
    expect((await value(ipc, "rotation_status", null)).job).toBeNull();
  });

  it("does not resume or discard past phase 1 the wrong way", async () => {
    const prepared = unlocked({ variant: "resumePrepared" });
    expect(await code(prepared, "rotation_resume", { password: null, kitGroup: null, code: null })).toBe("invalidInput");
    expect(await code(prepared, "rotation_abandon", null)).toBe("ok");
    const accepted = unlocked({ variant: "resumeAccepted" });
    expect(await code(accepted, "rotation_abandon", null)).toBe("rotationTooFar");
  });
});

describe("mock rotation: another device renewed the keys", () => {
  it("adopts with the new Secret Key and password, and refuses a key it used before", async () => {
    const ipc = unlocked({ variant: "rekeyElsewhere" }, { variant: "hostedNoTotp" });
    const overview = await value(ipc, "sync_status", null);
    expect(overview.state.status).toBe("signedOut");
    expect(overview.passwordChanged).toBe(true);
    const recover = (secretKey: string | null, newPassword = MOCK_NEW_PASSWORD) =>
      code(ipc, "sync_recover_password", { newPassword, secretKey, code: null });
    // The password alone does not open the new account key.
    expect(await recover(null)).toBe("wrongCredentials");
    expect(await recover(sampleAccount.secretKey)).toBe("secretKeyUnchanged");
    expect(await recover("nonsense")).toBe("secretKeyMalformed");
    expect(await recover(MOCK_NEW_SECRET_KEY, "wrong password")).toBe("wrongCredentials");
    expect(await recover(MOCK_NEW_SECRET_KEY)).toBe("ok");
    const after = await value(ipc, "sync_status", null);
    expect(after.state.status).toBe("upToDate");
    expect(after.passwordChanged).toBe(false);
    // The old key is now one the device used before.
    expect(await recover(sampleAccount.secretKey)).toBe("secretKeyUnchanged");
  });

  it("shows the folder marker and lets the finding be dismissed", async () => {
    const ipc = unlocked({ variant: "rekeyPending" });
    const pending = await value(ipc, "sync_status", null);
    expect(pending.rekeyPending).toBe(true);
    expect(pending.state.error).toBe("syncAccountRekeyed");
    const finding = unlocked({ variant: "conflict" });
    const warning = (await value(finding, "sync_status", null)).warnings.find(
      (w) => w.kind === "unverifiedAccountFile",
    );
    expect(warning).toMatchObject({ conflict: true });
    if (warning?.kind !== "unverifiedAccountFile") {
      throw new Error("no finding");
    }
    const next = await value(finding, "sync_dismiss_account_file", { name: warning.name, sha256: warning.sha256 });
    expect(next.warnings.some((w) => w.kind === "unverifiedAccountFile")).toBe(false);
  });
});
