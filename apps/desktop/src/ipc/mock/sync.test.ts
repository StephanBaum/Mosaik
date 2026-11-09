import { describe, expect, it } from "vitest";
import type { MosaicIpc } from "../client.ts";
import type {
  AppEvent,
  CommandName,
  ErrorCode,
  RequestOf,
  ResponseOf,
} from "../contract/index.ts";
import { createMockIpc, type MockOptions } from "../mock.ts";
import { MOCK_PASSWORD, sampleAccount } from "./data.ts";

const SERVER = "https://sync.example.com";

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

function unlocked(sync: MockOptions["sync"] = {}) {
  return createMockIpc({ start: "unlocked", sync });
}

const connect = (mode: "register" | "signIn", extra: { inviteCode?: string | null; password?: string } = {}) => ({
  serverUrl: SERVER,
  mode,
  password: extra.password ?? MOCK_PASSWORD,
  inviteCode: extra.inviteCode ?? null,
  deviceName: "Mosaic on Windows",
});

describe("mock sync: server", () => {
  it("starts not set up and checks the server address", async () => {
    const ipc = unlocked();
    const status = await value(ipc, "sync_status", null);
    expect(status.config).toBeNull();
    expect(status.state.status).toBe("notConfigured");
    expect(await code(ipc, "sync_check_server", { serverUrl: "nonsense" })).toBe("syncServerInvalid");
    expect(await code(ipc, "sync_check_server", { serverUrl: "http://sync.example.com" })).toBe(
      "syncServerInsecure",
    );
    expect(await value(ipc, "sync_check_server", { serverUrl: `${SERVER}/` })).toEqual({
      serverUrl: SERVER,
      registration: "open",
    });
  });

  it("answers the registration mode from the URL option", async () => {
    const ipc = unlocked({ server: "invite" });
    expect((await value(ipc, "sync_check_server", { serverUrl: SERVER })).registration).toBe("invite");
  });

  it("creates a sync account, checks the password and tells the app", async () => {
    const ipc = unlocked();
    const events: AppEvent[] = [];
    await ipc.subscribe((event) => events.push(event));
    expect(await code(ipc, "sync_hosted_connect", connect("register", { password: "wrong" }))).toBe(
      "wrongCredentials",
    );
    const status = await value(ipc, "sync_hosted_connect", connect("register"));
    expect(status.config).toMatchObject({ kind: "hosted", email: sampleAccount.email, totpEnabled: false });
    expect(status.state.status).toBe("upToDate");
    expect(events.some((event) => event.type === "syncStateChanged")).toBe(true);
    expect(await code(ipc, "sync_hosted_connect", connect("signIn"))).toBe("syncAlreadyConfigured");
  });

  it("enforces the registration mode and the invite code", async () => {
    const closed = unlocked({ server: "closed" });
    expect(await code(closed, "sync_hosted_connect", connect("register"))).toBe("syncRegistrationClosed");
    const invite = unlocked({ server: "invite" });
    expect(await code(invite, "sync_hosted_connect", connect("register"))).toBe("syncInviteInvalid");
    expect(await code(invite, "sync_hosted_connect", connect("register", { inviteCode: "nope" }))).toBe(
      "syncInviteInvalid",
    );
    expect(await code(invite, "sync_hosted_connect", connect("register", { inviteCode: "MOCK-INVITE" }))).toBe(
      "ok",
    );
  });

  it("signs in with the second factor after a disconnect", async () => {
    const ipc = unlocked({ variant: "hosted" });
    expect((await value(ipc, "sync_disconnect", null)).config).toBeNull();
    expect(await code(ipc, "sync_hosted_connect", connect("signIn", { password: "wrong" }))).toBe(
      "wrongCredentials",
    );
    expect(await code(ipc, "sync_hosted_connect", connect("signIn"))).toBe("syncTwoFactorRequired");
    expect(await code(ipc, "sync_hosted_two_factor", { code: "000000" })).toBe("syncTwoFactorInvalid");
    const status = await value(ipc, "sync_hosted_two_factor", { code: "123456" });
    expect(status.config).toMatchObject({ kind: "hosted", totpEnabled: true });
  });

  it("refuses a sign-in when the server knows no sync account", async () => {
    expect(await code(unlocked(), "sync_hosted_connect", connect("signIn"))).toBe("wrongCredentials");
  });

  it("lets a signed-out device sign in again", async () => {
    const ipc = unlocked({ variant: "signedOut" });
    expect((await value(ipc, "sync_status", null)).state.status).toBe("signedOut");
    expect(await code(ipc, "sync_now", null)).toBe("syncSignedOut");
    expect(await code(ipc, "sync_hosted_connect", connect("signIn"))).toBe("syncTwoFactorRequired");
    const status = await value(ipc, "sync_hosted_two_factor", { code: "123456" });
    expect(status.state.status).toBe("upToDate");
  });

  it("tells a device with an old password that the server holds a newer one, and recovers", async () => {
    const ipc = unlocked({ variant: "passwordChanged" });
    expect((await value(ipc, "sync_status", null)).passwordChanged).toBe(true);
    expect(await code(ipc, "sync_hosted_connect", connect("signIn"))).toBe("syncPasswordChanged");
    const recover = (newPassword: string, otp: string | null) => ({ newPassword, secretKey: null, code: otp });
    expect(await code(ipc, "sync_recover_password", recover("nope", "123456"))).toBe("wrongCredentials");
    expect(await code(ipc, "sync_recover_password", recover("new correct horse battery staple", null))).toBe(
      "syncTwoFactorRequired",
    );
    const status = await value(ipc, "sync_recover_password", recover("new correct horse battery staple", "123456"));
    expect(status.passwordChanged).toBe(false);
    expect(status.state.status).toBe("upToDate");
  });

  it("fails connects and syncs with the code from &syncFail", async () => {
    const ipc = unlocked({ fail: "syncRateLimited" });
    expect(await code(ipc, "sync_hosted_connect", connect("register"))).toBe("syncRateLimited");
    const connected = unlocked({ variant: "hosted", fail: "syncQuotaExceeded" });
    expect(await code(connected, "sync_now", null)).toBe("syncQuotaExceeded");
  });
});

describe("mock sync: folder", () => {
  it("answers the folder choice from the URL option", async () => {
    expect(await value(unlocked({ folder: "cancel" }), "sync_pick_folder", null)).toBeNull();
    const mosaic = await value(unlocked({ folder: "mosaic" }), "sync_pick_folder", null);
    expect(mosaic).toMatchObject({ contents: "mosaic", accounts: [sampleAccount.accountId] });
  });

  it("connects an empty folder and refuses a folder with other files", async () => {
    const other = unlocked({ folder: "other" });
    const bad = await value(other, "sync_pick_folder", null);
    expect(bad?.contents).toBe("other");
    expect(
      await code(other, "sync_folder_connect", { choiceId: bad?.choiceId ?? "", deviceName: "Laptop" }),
    ).toBe("syncFolderNotEmpty");

    const ipc = unlocked();
    const choice = await value(ipc, "sync_pick_folder", null);
    expect(await code(ipc, "sync_folder_connect", { choiceId: "unknown", deviceName: "x" })).toBe(
      "syncChoiceExpired",
    );
    const status = await value(ipc, "sync_folder_connect", {
      choiceId: choice?.choiceId ?? "",
      deviceName: "Laptop",
    });
    expect(status.config).toEqual({ kind: "folder", displayPath: choice?.displayPath, deviceName: "Laptop" });
  });
});

describe("mock sync: status, warnings and acknowledge", () => {
  it("syncs now: sets the time and emits the state", async () => {
    const ipc = unlocked({ variant: "hosted" });
    const events: AppEvent[] = [];
    await ipc.subscribe((event) => events.push(event));
    const before = (await value(ipc, "sync_status", null)).state.lastSyncedAt ?? 0;
    const status = await value(ipc, "sync_now", null);
    expect(status.state.lastSyncedAt ?? 0).toBeGreaterThan(before);
    expect(events.some((event) => event.type === "syncStateChanged")).toBe(true);
    expect(await code(unlocked(), "sync_now", null)).toBe("syncNotConfigured");
  });

  it.each([
    ["rollback", "rollbackDetected"],
    ["identity", "identityDuplicated"],
    ["serverInconsistent", "serverInconsistent"],
    ["serverRollback", "serverRollback"],
    ["noCommonView", "noCommonView"],
    ["freeze", "freezeSuspected"],
    ["missingFiles", "missingFiles"],
    ["undecryptable", "undecryptable"],
    ["hwLocal", "hardwareVaultsLocal"],
    ["notRepaired", "attachmentsNotRepaired"],
  ] as const)("shows the %s variant as the %s warning", async (variant, kind) => {
    const status = await value(unlocked({ variant }), "sync_status", null);
    expect(status.warnings.map((warning) => warning.kind)).toEqual([kind]);
  });

  it("acknowledge clears halts and findings and returns the status without them", async () => {
    const ipc = unlocked({ variant: "allWarnings" });
    const before = await value(ipc, "sync_status", null);
    expect(before.warnings).toHaveLength(10);
    expect(before.state.status).toBe("error");
    const after = await value(ipc, "sync_acknowledge", {});
    expect(after.warnings.map((warning) => warning.kind).sort()).toEqual(
      ["attachmentsNotRepaired", "freezeSuspected", "hardwareVaultsLocal", "missingFiles", "undecryptable"].sort(),
    );
    expect(after.state.status).toBe("upToDate");
  });

  it("keeps a halted device in error when syncing now", async () => {
    const ipc = unlocked({ variant: "rollback" });
    expect((await value(ipc, "sync_now", null)).state.status).toBe("error");
  });
});

describe("mock sync: devices and two-step verification", () => {
  it("lists, renames and removes devices of a server account", async () => {
    const ipc = unlocked({ variant: "hosted" });
    const { devices } = await value(ipc, "sync_devices", null);
    const current = devices.find((device) => device.current);
    const other = devices.find((device) => device.canRemove);
    expect(current).toBeDefined();
    expect(other).toBeDefined();
    await value(ipc, "sync_device_rename", { deviceId: current?.deviceId ?? "", name: "Work laptop" });
    expect((await value(ipc, "sync_status", null)).config).toMatchObject({ deviceName: "Work laptop" });

    const remove = (password: string, totp: string | null) => ({
      deviceId: other?.deviceId ?? "",
      password,
      code: totp,
    });
    expect(await code(ipc, "sync_device_remove", remove("wrong", "123456"))).toBe("wrongCredentials");
    expect(await code(ipc, "sync_device_remove", remove(MOCK_PASSWORD, null))).toBe("syncTwoFactorRequired");
    expect(await code(ipc, "sync_device_remove", remove(MOCK_PASSWORD, "000000"))).toBe("syncTwoFactorInvalid");
    expect(
      await code(ipc, "sync_device_remove", {
        deviceId: current?.deviceId ?? "",
        password: MOCK_PASSWORD,
        code: "123456",
      }),
    ).toBe("syncDeviceIsCurrent");
    expect(await code(ipc, "sync_device_remove", remove(MOCK_PASSWORD, "123456"))).toBe("ok");
    expect((await value(ipc, "sync_devices", null)).devices).toHaveLength(devices.length - 1);
  });

  it("makes no device removable in a folder, and the folder renames only this device", async () => {
    const ipc = unlocked({ variant: "folder" });
    const { devices } = await value(ipc, "sync_devices", null);
    expect(devices.some((device) => device.canRemove)).toBe(false);
    const other = devices.find((device) => !device.current);
    expect(await code(ipc, "sync_device_rename", { deviceId: other?.deviceId ?? "", name: "x" })).toBe(
      "invalidInput",
    );
    expect(
      await code(ipc, "sync_device_remove", { deviceId: other?.deviceId ?? "", password: MOCK_PASSWORD, code: null }),
    ).toBe("invalidInput");
  });

  it("turns two-step verification on and off", async () => {
    const ipc = unlocked({ variant: "hostedNoTotp" });
    expect(await code(ipc, "sync_totp_begin", { password: "wrong" })).toBe("wrongCredentials");
    expect(await code(ipc, "sync_totp_confirm", { code: "123456" })).toBe("invalidInput");
    const setup = await value(ipc, "sync_totp_begin", { password: MOCK_PASSWORD });
    expect(setup.secret).not.toBe("");
    expect(setup.otpauthUri.startsWith("otpauth://totp/")).toBe(true);
    expect(await code(ipc, "sync_totp_confirm", { code: "000000" })).toBe("syncTwoFactorInvalid");
    expect(await code(ipc, "sync_totp_confirm", { code: "123456" })).toBe("ok");
    expect((await value(ipc, "sync_status", null)).config).toMatchObject({ totpEnabled: true });
    expect(await code(ipc, "sync_totp_disable", { password: MOCK_PASSWORD, code: "000000" })).toBe(
      "syncTwoFactorInvalid",
    );
    expect(await code(ipc, "sync_totp_disable", { password: MOCK_PASSWORD, code: "123456" })).toBe("ok");
    expect((await value(ipc, "sync_status", null)).config).toMatchObject({ totpEnabled: false });
  });

  it("disconnects and keeps no warnings", async () => {
    const ipc = unlocked({ variant: "allWarnings" });
    const status = await value(ipc, "sync_disconnect", null);
    expect(status).toMatchObject({ config: null, warnings: [], state: { status: "notConfigured" } });
  });
});

describe("mock sync: join", () => {
  const join = {
    serverUrl: SERVER,
    email: "alex@example.com",
    password: MOCK_PASSWORD,
    secretKey: sampleAccount.secretKey,
    deviceName: "Mosaic on Windows",
  };

  it("joins a server account and unlocks with the sample vaults", async () => {
    const ipc = createMockIpc();
    expect(await code(ipc, "join_hosted", { ...join, secretKey: "M1-AAAA-AAAA-AAAA-AAAA-AAAA-AAAA-AAAA" })).toBe(
      "wrongCredentials",
    );
    expect(await code(ipc, "join_hosted", { ...join, password: "wrong" })).toBe("wrongCredentials");
    expect(await value(ipc, "join_hosted", join)).toEqual({ state: "unlocked" });
    expect(await value(ipc, "session_state", null)).toMatchObject({ state: "unlocked", kitVerified: true });
    expect((await value(ipc, "list_vaults", null)).length).toBeGreaterThan(0);
    expect((await value(ipc, "sync_status", null)).config).toMatchObject({ kind: "hosted" });
    expect(await code(ipc, "join_hosted", join)).toBe("accountExists");
  });

  it("asks for the code first with &syncJoin2fa=1", async () => {
    const ipc = createMockIpc({ sync: { join2fa: true } });
    expect(await value(ipc, "join_hosted", join)).toEqual({ state: "twoFactorRequired" });
    expect(await code(ipc, "join_hosted_two_factor", { code: "000000" })).toBe("syncTwoFactorInvalid");
    expect(await value(ipc, "join_hosted_two_factor", { code: "123456" })).toEqual({ state: "unlocked" });
    expect((await value(ipc, "sync_status", null)).config).toMatchObject({ totpEnabled: true });
  });

  it("joins from a folder that lists the account", async () => {
    const ipc = createMockIpc({ sync: { folder: "mosaic" } });
    const choice = await value(ipc, "sync_pick_folder", null);
    const request = {
      choiceId: choice?.choiceId ?? "",
      accountId: sampleAccount.accountId,
      password: MOCK_PASSWORD,
      secretKey: sampleAccount.secretKey,
      deviceName: "Mosaic on Windows",
    };
    expect(await code(ipc, "join_folder", { ...request, accountId: "other" })).toBe("syncFolderNoAccount");
    expect(await code(ipc, "join_folder", { ...request, choiceId: "gone" })).toBe("syncChoiceExpired");
    expect(await value(ipc, "join_folder", request)).toEqual({ state: "unlocked" });
    expect((await value(ipc, "sync_status", null)).config).toMatchObject({ kind: "folder" });
  });

  it("changes the master password, asking for the code when the account has one", async () => {
    const ipc = unlocked({ variant: "hosted" });
    const request = { currentPassword: MOCK_PASSWORD, newPassword: "new one", code: null };
    expect(await code(ipc, "change_master_password", { ...request, currentPassword: "wrong" })).toBe(
      "wrongCredentials",
    );
    expect(await code(ipc, "change_master_password", request)).toBe("syncTwoFactorRequired");
    expect(await code(ipc, "change_master_password", { ...request, code: "123456" })).toBe("ok");
  });
});
