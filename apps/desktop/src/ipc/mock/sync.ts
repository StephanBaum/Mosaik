/**
 * Mock sync commands (M6): the same rules as `src-tauri/src/commands/sync.rs` where the UI
 * can see them. The mock server takes the sample account's master password and the code
 * `123456`; `syncData.ts` holds the state and the `?sync=` variants.
 */
import type { FolderChoice, SyncDevice } from "../contract/index.ts";
import { sampleAccount } from "./data.ts";
import { syncJoinHandlers } from "./syncJoin.ts";
import {
  MOCK_INVITE_CODE,
  MOCK_TOTP_CODE,
  MOCK_FOLDER_PATH,
  normalizeServer,
  settle,
  overview,
  publish,
  type MockSync,
} from "./syncData.ts";
import { fail, requireUnlocked, type MockHandlers, type MockState } from "./state.ts";

const HALT_KINDS = ["rollbackDetected", "identityDuplicated"] as const;
const FINDING_KINDS = ["serverInconsistent", "serverRollback", "noCommonView"] as const;
const CHOICE_PATHS = { empty: "D:\\Sync", mosaic: MOCK_FOLDER_PATH, other: "D:\\Documents" } as const;

function connected(state: MockState): NonNullable<MockSync["config"]> {
  return state.sync.config ?? fail("syncNotConfigured");
}

function hostedOnly(state: MockState): void {
  if (connected(state).kind !== "hosted") {
    fail("invalidInput");
  }
}

function checkPassword(state: MockState, password: string): void {
  if (password !== (state.account?.password ?? sampleAccount.password)) {
    fail("wrongCredentials");
  }
}

function checkCode(code: string | null): void {
  if (code === null || code === "") {
    fail("syncTwoFactorRequired");
  }
  if (code !== MOCK_TOTP_CODE) {
    fail("syncTwoFactorInvalid");
  }
}

export const syncHandlers: MockHandlers = {
  ...syncJoinHandlers,

  sync_status: (state) => {
    requireUnlocked(state);
    return overview(state);
  },

  sync_check_server: (state, request) => ({
    serverUrl: normalizeServer(request.serverUrl),
    registration: state.sync.server,
  }),

  sync_hosted_connect: (state, request) => {
    requireUnlocked(state);
    const { sync } = state;
    // A signed-out device signs in again with the same command.
    if (sync.config !== null && sync.status !== "signedOut") {
      fail("syncAlreadyConfigured");
    }
    if (sync.fail !== null) {
      fail(sync.fail);
    }
    const serverUrl = normalizeServer(request.serverUrl);
    if (request.mode === "register") {
      if (sync.server === "closed") {
        fail("syncRegistrationClosed");
      }
      if (sync.server === "invite" && request.inviteCode !== MOCK_INVITE_CODE) {
        fail("syncInviteInvalid");
      }
      if (sync.registered) {
        fail("syncAccountExists");
      }
      checkPassword(state, request.password);
      sync.registered = true;
      return settle(state, { kind: "hosted", serverUrl, deviceName: request.deviceName });
    }
    checkPassword(state, request.password);
    if (!sync.registered) {
      fail("wrongCredentials");
    }
    if (sync.serverPassword !== null) {
      // The device's password is right (checked above) and the server refuses it: the server
      // holds a newer one.
      sync.passwordChanged = true;
      publish(state);
      fail("syncPasswordChanged");
    }
    if (sync.totp) {
      sync.pendingTwoFactor = { kind: "connect", serverUrl, deviceName: request.deviceName };
      fail("syncTwoFactorRequired");
    }
    return settle(state, { kind: "hosted", serverUrl, deviceName: request.deviceName });
  },

  sync_hosted_two_factor: (state, request) => {
    requireUnlocked(state);
    const pending = state.sync.pendingTwoFactor;
    if (pending?.kind !== "connect") {
      fail("syncNotConfigured");
    }
    checkCode(request.code);
    return settle(state, { kind: "hosted", serverUrl: pending.serverUrl, deviceName: pending.deviceName });
  },

  sync_pick_folder: (state) => {
    const answer = state.sync.folder;
    if (answer === "cancel") {
      return null;
    }
    const choice: FolderChoice = {
      choiceId: state.nextId(),
      displayPath: CHOICE_PATHS[answer],
      contents: answer,
      accounts: answer === "mosaic" ? [sampleAccount.accountId] : [],
    };
    state.sync.choices.set(choice.choiceId, choice);
    return choice;
  },

  sync_folder_connect: (state, request) => {
    requireUnlocked(state);
    const { sync } = state;
    if (sync.config !== null) {
      fail("syncAlreadyConfigured");
    }
    if (sync.fail !== null) {
      fail(sync.fail);
    }
    const choice = sync.choices.get(request.choiceId) ?? fail("syncChoiceExpired");
    if (choice.contents === "other") {
      fail("syncFolderNotEmpty");
    }
    if (choice.contents === "mosaic" && !choice.accounts.includes(state.account?.accountId ?? "")) {
      fail("syncFolderNoAccount");
    }
    sync.choices.delete(request.choiceId);
    return settle(state, { kind: "folder", path: choice.displayPath, deviceName: request.deviceName });
  },

  sync_now: (state) => {
    requireUnlocked(state);
    const { sync } = state;
    connected(state);
    if (sync.fail !== null) {
      fail(sync.fail);
    }
    if (sync.status === "signedOut") {
      fail("syncSignedOut");
    }
    const halted = sync.warnings.some((warning) => (HALT_KINDS as readonly string[]).includes(warning.kind));
    sync.lastSyncedAt = state.now();
    sync.status = halted ? "error" : "upToDate";
    sync.error = null;
    return publish(state);
  },

  sync_disconnect: (state) => {
    requireUnlocked(state);
    const { sync } = state;
    sync.config = null;
    sync.status = "notConfigured";
    sync.error = null;
    sync.lastSyncedAt = null;
    sync.warnings = [];
    sync.pendingTwoFactor = null;
    sync.totpPending = false;
    sync.serverPassword = null;
    sync.passwordChanged = false;
    return publish(state);
  },

  // The real command asks in a native dialog first; the mock answers yes.
  sync_acknowledge: (state) => {
    requireUnlocked(state);
    const { sync } = state;
    connected(state);
    const clear = [...HALT_KINDS, ...FINDING_KINDS] as readonly string[];
    const halted = sync.warnings.some((warning) => (HALT_KINDS as readonly string[]).includes(warning.kind));
    sync.warnings = sync.warnings.filter((warning) => !clear.includes(warning.kind));
    state.sharing.findings = [];
    if (halted && sync.status === "error") {
      sync.status = "upToDate";
      sync.lastSyncedAt = state.now();
    }
    return publish(state);
  },

  sync_recover_password: (state, request) => {
    requireUnlocked(state);
    const config = connected(state);
    const { sync } = state;
    if (request.newPassword === "") {
      fail("emptyPassword");
    }
    if (sync.serverPassword === null || request.newPassword !== sync.serverPassword) {
      fail("wrongCredentials");
    }
    if (config.kind === "hosted" && sync.totp) {
      checkCode(request.code);
    }
    if (state.account !== null) {
      state.account.password = sync.serverPassword;
    }
    sync.serverPassword = null;
    sync.passwordChanged = false;
    sync.status = "upToDate";
    sync.error = null;
    sync.lastSyncedAt = state.now();
    return publish(state);
  },

  sync_devices: (state) => {
    requireUnlocked(state);
    connected(state);
    return { devices: state.sync.devices };
  },

  sync_device_rename: (state, request) => {
    requireUnlocked(state);
    const config = connected(state);
    const device = state.sync.devices.find((d) => d.deviceId === request.deviceId);
    const name = request.name.trim();
    if (device === undefined || name === "" || (config.kind === "folder" && !device.current)) {
      fail("invalidInput");
    }
    device.name = name;
    if (device.current) {
      state.sync.config = { ...config, deviceName: name };
    }
    return null;
  },

  sync_device_remove: (state, request) => {
    requireUnlocked(state);
    const config = connected(state);
    hostedOnly(state);
    const device: SyncDevice | undefined = state.sync.devices.find((d) => d.deviceId === request.deviceId);
    if (device === undefined) {
      fail("invalidInput");
    }
    if (device.current) {
      fail("syncDeviceIsCurrent");
    }
    checkPassword(state, request.password);
    if (config.kind === "hosted" && config.totpEnabled) {
      checkCode(request.code);
    }
    state.sync.devices = state.sync.devices.filter((d) => d.deviceId !== request.deviceId);
    return null;
  },

  sync_totp_begin: (state, request) => {
    requireUnlocked(state);
    hostedOnly(state);
    checkPassword(state, request.password);
    if (state.sync.totp) {
      fail("invalidInput");
    }
    state.sync.totpPending = true;
    return {
      secret: "JBSWY3DPEHPK3PXP",
      otpauthUri: `otpauth://totp/Mosaic%20sync:${sampleAccount.email}?secret=JBSWY3DPEHPK3PXP&issuer=Mosaic%20sync`,
    };
  },

  sync_totp_confirm: (state, request) => {
    requireUnlocked(state);
    const config = connected(state);
    if (!state.sync.totpPending) {
      fail("invalidInput");
    }
    checkCode(request.code);
    state.sync.totpPending = false;
    state.sync.totp = true;
    if (config.kind === "hosted") {
      state.sync.config = { ...config, totpEnabled: true };
    }
    return null;
  },

  sync_totp_disable: (state, request) => {
    requireUnlocked(state);
    const config = connected(state);
    hostedOnly(state);
    checkPassword(state, request.password);
    checkCode(request.code);
    state.sync.totp = false;
    if (config.kind === "hosted") {
      state.sync.config = { ...config, totpEnabled: false };
    }
    return null;
  },
};
