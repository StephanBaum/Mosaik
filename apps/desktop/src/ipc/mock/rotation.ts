/**
 * Mock account keys (M7): the Secret Key rotation, the key derivation upgrade, the re-key, the
 * journal that resumes them and the adoption of a re-key made on another device. The same rules
 * as `src-tauri/src/sync/rotation_flow.rs` where the UI can see them; `&rotation=<variant>` seeds
 * the state (see the header of `mock.ts`). The mock "other device" holds `MOCK_NEW_SECRET_KEY`
 * and the password `MOCK_NEW_PASSWORD` (the `rekeyElsewhere` and `rekeyPending` variants).
 */
import type {
  ErrorCode,
  RekeyMarkerView,
  RekeyReviewView,
  RotationDone,
  RotationJobView,
  RotationPhaseName,
  RotationProfile,
  SyncOverview,
} from "../contract/index.ts";
import { sampleAccount } from "./data.ts";
import { newSecretKey, normalizeKey } from "./session.ts";
import {
  fail,
  requireUnlocked,
  type MockAsyncHandlers,
  type MockHandlers,
  type MockState,
} from "./state.ts";
import { MOCK_NEW_PASSWORD, MOCK_TOTP_CODE, publish } from "./syncData.ts";

export const MOCK_NEW_SECRET_KEY = "M1-7KQ4-X29H-DM3P-RT6W-ZB04-NCFG-8VJ3";

export const rotationVariants = [
  "none",
  "shared",
  "resumePrepared",
  "resumeAccepted",
  "resumeAttachments",
  "resumeSecretKey",
  "resumeStuck",
  "rekeyElsewhere",
  "rekeyPending",
  "unknownFile",
  "conflict",
  "slowDevice",
] as const;
export type RotationVariant = (typeof rotationVariants)[number];

/** The URL options of the rotation mock (`&rotation=`, `&rotationFail=`, `&rotationStepMs=`). */
export interface MockRotationOptions {
  variant?: RotationVariant;
  fail?: ErrorCode;
  stepMs?: number;
}

export interface MockRotation {
  job: RotationJobView | null;
  /** The staged Secret Key (display form); the webview never gets it. */
  pendingKey: string | null;
  /** The new kit was saved and verified in this run. */
  kitVerified: boolean;
  profile: RotationProfile | "baseline";
  review: RekeyReviewView;
  rekeyPending: boolean;
  /** The re-key notices the folder shows and the user has not ignored. */
  markers: RekeyMarkerView[];
  /** A continue that cannot finish (an attachment re-seal keeps failing). */
  stuck: boolean;
  /** Secret Keys this device has used. */
  retired: string[];
  /** The key and password another device set (adopt flows). */
  elsewhere: { secretKey: string; password: string } | null;
  fail: ErrorCode | null;
  slow: boolean;
  stepMs: number;
}

const PROFILE_ORDER = ["baseline", "standard", "strong", "max"] as const;

const family = "019a2b3c-4d5e-7000-8000-000000000002";
const team = "019a2b3c-4d5e-7000-8000-000000000009";

function baseReview(): RekeyReviewView {
  return {
    ownedVaults: 2,
    sharedOwned: [],
    memberVaults: [],
    hardwareVaults: 0,
    grants: 0,
    hardwareKeys: 0,
    quickUnlock: false,
    unreachableAttachments: 0,
    backend: "hosted",
    accountId: sampleAccount.accountId,
  };
}

function jobOf(
  kind: RotationJobView["kind"],
  phase: RotationPhaseName,
  needs: RotationJobView["needs"],
  counts: { done: number; total: number } = { done: 0, total: 0 },
): RotationJobView {
  return { kind, phase, needs, ...counts };
}

function seededJob(variant: RotationVariant): RotationJobView | null {
  switch (variant) {
    case "resumePrepared":
      return jobOf("rekey", "prepared", "password");
    case "resumeAccepted":
      return jobOf("rekey", "backendAccepted", "kitGroup");
    case "resumeAttachments":
    case "resumeStuck":
      return jobOf("rekey", "attachments", "none", { done: 3, total: 12 });
    case "resumeSecretKey":
      return jobOf("secretKey", "backendStarted", "kitGroup");
    default:
      return null;
  }
}

export function seedRotation(options: MockRotationOptions): MockRotation {
  const variant = options.variant ?? "none";
  const elsewhere = variant === "rekeyElsewhere" || variant === "rekeyPending";
  const staged = variant.startsWith("resume") && variant !== "resumeAttachments" && variant !== "resumeStuck";
  return {
    job: seededJob(variant),
    pendingKey: staged ? MOCK_NEW_SECRET_KEY : null,
    kitVerified: false,
    profile: "standard",
    review:
      variant === "shared"
        ? {
            ...baseReview(),
            sharedOwned: [{ vaultId: family, name: "Family", members: 2 }],
            memberVaults: [{ vaultId: team, name: "Team" }],
            grants: 1,
            hardwareKeys: 1,
            hardwareVaults: 1,
            quickUnlock: true,
            unreachableAttachments: 2,
          }
        : baseReview(),
    rekeyPending: variant === "rekeyPending",
    markers: variant === "rekeyPending" ? [{ name: "rekey-00000002.mrk", sha256: "cd".repeat(32), toEpoch: 2 }] : [],
    stuck: variant === "resumeStuck",
    retired: [],
    elsewhere: elsewhere ? { secretKey: MOCK_NEW_SECRET_KEY, password: MOCK_NEW_PASSWORD } : null,
    fail: options.fail ?? null,
    slow: variant === "slowDevice",
    stepMs: options.stepMs ?? 450,
  };
}

function done(state: MockState, finished = true): RotationDone {
  return {
    quickUnlockRemoved: state.quick.biometricEnrolledAt !== null,
    hardwareKeysRemoved: state.quick.keys.length,
    unreachableAttachments: state.rotation.review.unreachableAttachments,
    unreachable: Array.from({ length: state.rotation.review.unreachableAttachments }, (_, index) =>
      index === 0
        ? { itemTitle: "Bank", fileName: "statement.pdf" }
        : { itemTitle: null, fileName: null },
    ),
    finished,
  };
}

function checkPassword(state: MockState, password: string): void {
  if (password === "") {
    fail("emptyPassword");
  }
  if (password !== (state.account?.password ?? sampleAccount.password)) {
    fail("wrongCredentials");
  }
}

function checkCode(state: MockState, code: string | null): void {
  if (state.sync.config?.kind === "hosted" && state.sync.totp && code !== MOCK_TOTP_CODE) {
    fail(code === null || code === "" ? "syncTwoFactorRequired" : "syncTwoFactorInvalid");
  }
}

function lastGroup(key: string): string {
  return normalizeKey(key).slice(-4);
}

function stage(state: MockState, kind: RotationJobView["kind"]): void {
  const { rotation } = state;
  if (rotation.job !== null && (rotation.job.kind !== kind || rotation.job.phase !== "prepared")) {
    fail("rotationInProgress");
  }
  rotation.job = jobOf(kind, "prepared", "password");
  rotation.kitVerified = false;
  if (kind !== "profile" && rotation.pendingKey === null) {
    // The first new key of a mock run is the documented one, so a reviewer can type its last group
    // (`8VJ3`); a later one is random.
    const inUse = state.account !== null && normalizeKey(state.account.secretKey) === normalizeKey(MOCK_NEW_SECRET_KEY);
    rotation.pendingKey = inUse ? newSecretKey() : MOCK_NEW_SECRET_KEY;
  }
}

function forcedFailure(state: MockState): void {
  if (state.rotation.fail !== null) {
    fail(state.rotation.fail);
  }
}

function announce(state: MockState): void {
  const { job } = state.rotation;
  if (job !== null) {
    state.emit({ type: "rotationProgress", kind: job.kind, phase: job.phase, done: job.done, total: job.total });
  }
}

function sleep(ms: number): Promise<void> {
  return ms <= 0 ? Promise.resolve() : new Promise((resolve) => setTimeout(resolve, ms));
}

const PHASES: RotationPhaseName[] = [
  "backendStarted",
  "backendAccepted",
  "localCommitted",
  "attachments",
  "uploading",
];

/** The rest of a job after `from`: the server, the local commit, the attachments, the uploads. */
async function runPhases(state: MockState, from: RotationPhaseName): Promise<void> {
  const { rotation } = state;
  const start = PHASES.indexOf(from);
  for (const phase of PHASES.slice(Math.max(start, 0) + 1)) {
    await sleep(rotation.stepMs);
    if (rotation.job === null) {
      return;
    }
    const counted = phase === "attachments" || phase === "uploading";
    rotation.job = {
      ...rotation.job,
      phase,
      needs: "none",
      total: counted ? 12 : 0,
      done: phase === "uploading" ? 12 : phase === "attachments" ? 6 : 0,
    };
    announce(state);
  }
  await sleep(rotation.stepMs);
}

/** The new Secret Key replaces the old one and the job closes. */
function renew(state: MockState): void {
  const { rotation } = state;
  if (state.account !== null && rotation.pendingKey !== null) {
    rotation.retired.push(state.account.secretKey);
    state.account.secretKey = rotation.pendingKey;
  }
  rotation.pendingKey = null;
  rotation.kitVerified = false;
  rotation.job = null;
  rotation.review.unreachableAttachments = 0;
  state.quick.biometricEnrolledAt = null;
}

export const rotationHandlers: MockHandlers = {
  rotation_status: (state) => {
    requireUnlocked(state);
    return { job: state.rotation.job };
  },

  secret_key_rotate_prepare: (state, request) => {
    requireUnlocked(state);
    checkPassword(state, request.password);
    stage(state, "secretKey");
    return null;
  },

  secret_key_rotate_commit: (state, request) => {
    requireUnlocked(state);
    checkPassword(state, request.password);
    if (!state.rotation.kitVerified || state.rotation.pendingKey === null) {
      fail("kitNotVerified");
    }
    checkCode(state, request.code);
    forcedFailure(state);
    const result = { ...done(state), hardwareKeysRemoved: 0 };
    renew(state);
    return result;
  },

  profile_upgrade: (state, request) => {
    requireUnlocked(state);
    const { rotation } = state;
    if (PROFILE_ORDER.indexOf(request.profile) <= PROFILE_ORDER.indexOf(rotation.profile)) {
      fail("profileDowngrade");
    }
    if (rotation.slow && !request.force) {
      fail("profileTooSlow");
    }
    checkPassword(state, request.password);
    checkCode(state, request.code);
    forcedFailure(state);
    const result = { ...done(state), hardwareKeysRemoved: 0 };
    rotation.profile = request.profile;
    rotation.job = null;
    state.quick.biometricEnrolledAt = null;
    return result;
  },

  rekey_review: (state) => {
    requireUnlocked(state);
    return { ...state.rotation.review, backend: state.sync.config?.kind ?? "local" };
  },

  rekey_prepare: (state, request) => {
    requireUnlocked(state);
    checkPassword(state, request.password);
    if (request.newPassword === "") {
      fail("emptyPassword");
    }
    if (request.newPassword === request.password) {
      fail("passwordUnchanged");
    }
    if (state.sync.config === null) {
      fail("syncNotConfigured");
    }
    stage(state, "rekey");
    return null;
  },

  rotation_abandon: (state) => {
    requireUnlocked(state);
    const { rotation } = state;
    if (rotation.job === null) {
      fail("rotationNone");
    }
    if (rotation.job.phase !== "prepared") {
      fail("rotationTooFar");
    }
    rotation.job = null;
    rotation.pendingKey = null;
    rotation.kitVerified = false;
    return null;
  },

  sync_dismiss_account_file: (state, request) => {
    requireUnlocked(state);
    state.sync.warnings = state.sync.warnings.filter(
      (w) => !(w.kind === "unverifiedAccountFile" && w.sha256 === request.sha256),
    );
    return publish(state);
  },
};

export const rotationAsyncHandlers: MockAsyncHandlers = {
  rekey_commit: async (state, request) => {
    requireUnlocked(state);
    const { rotation } = state;
    checkPassword(state, request.password);
    if (request.newPassword === request.password) {
      fail("passwordUnchanged");
    }
    if (!rotation.kitVerified || rotation.pendingKey === null) {
      fail("kitNotVerified");
    }
    checkCode(state, request.code);
    forcedFailure(state);
    const result = done(state);
    rotation.job = jobOf("rekey", "backendStarted", "none");
    announce(state);
    await runPhases(state, "backendStarted");
    if (state.account !== null) {
      state.account.password = request.newPassword;
    }
    state.quick.keys = [];
    renew(state);
    return result;
  },

  rotation_resume: async (state, request) => {
    requireUnlocked(state);
    const { rotation } = state;
    const job = rotation.job ?? fail("rotationNone");
    if (job.needs === "password" || job.phase === "prepared") {
      fail("invalidInput");
    }
    if (job.needs === "kitGroup") {
      if (request.password === null || request.kitGroup === null) {
        fail("invalidInput");
      }
      if (rotation.pendingKey === null || normalizeKey(request.kitGroup) !== lastGroup(rotation.pendingKey)) {
        fail("kitGroupMismatch");
      }
      checkCode(state, request.code);
      if (state.account !== null && job.kind === "rekey") {
        state.account.password = request.password;
      }
    }
    forcedFailure(state);
    if (rotation.stuck) {
      // The job cannot finish now: the person may wait or give it up.
      return done(state, false);
    }
    const result = done(state);
    await runPhases(state, job.phase);
    renew(state);
    return result;
  },
};

/**
 * `sync_recover_password` with the Secret Key of the newest kit: the keys were renewed on
 * another device. A Secret Key this device used before is refused with its own message.
 */
export function adoptRenewedKeys(
  state: MockState,
  request: { newPassword: string; secretKey: string; code: string | null },
): SyncOverview {
  const { rotation, sync } = state;
  const typed = normalizeKey(request.secretKey);
  if (!typed.startsWith("M1") || typed.length < 30) {
    fail("secretKeyMalformed");
  }
  const own = state.account === null ? [] : [state.account.secretKey, ...rotation.retired];
  if (own.some((key) => normalizeKey(key) === typed)) {
    fail("secretKeyUnchanged");
  }
  const other = rotation.elsewhere;
  if (other === null || normalizeKey(other.secretKey) !== typed || request.newPassword !== other.password) {
    fail("wrongCredentials");
  }
  if (sync.config?.kind === "hosted" && sync.totp) {
    checkCode(state, request.code);
  }
  if (state.account !== null) {
    rotation.retired.push(state.account.secretKey);
    state.account.password = other.password;
    state.account.secretKey = other.secretKey;
  }
  state.quick.biometricEnrolledAt = null;
  rotation.elsewhere = null;
  rotation.rekeyPending = false;
  rotation.markers = [];
  sync.serverPassword = null;
  sync.passwordChanged = false;
  sync.status = "upToDate";
  sync.error = null;
  sync.warnings = sync.warnings.filter((w) => w.kind !== "unverifiedAccountFile");
  sync.lastSyncedAt = state.now();
  return publish(state);
}
