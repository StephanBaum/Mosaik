/**
 * Credential rotation contract (M7b): mirrors `src-tauri/src/contract/credential_rotation.rs`.
 * The rotate-now queue, the guided password change of one login and the opt-in breach check.
 * Not the key rotation of the account (`rotation.ts`).
 */
import type { CopyReceipt } from "./clipboard.ts";
import type { ItemId, UnixMillis, VaultId } from "./common.ts";
import type { InsightSeverity } from "./insights.ts";
import type { ItemRequest, RevealedSecret } from "./items.ts";
import type { BreachMode } from "./settings.ts";

/** Whether Mosaic can guide the change (a login with an https URL) or only offer "Edit item". */
export const queueKinds = ["guided", "manual"] as const;
export type QueueKind = (typeof queueKinds)[number];

/** In this order. */
export const queueReasons = ["weak", "reused", "old", "breached", "unfinished"] as const;
export type QueueReason = (typeof queueReasons)[number];

export const attemptStages = ["prepared", "delivered"] as const;
export type AttemptStage = (typeof attemptStages)[number];

/** Where the generator rules of a new password came from, when not the page itself. */
export const rulesSourceKeys = ["siteQuirks"] as const;
export type RulesSourceKey = (typeof rulesSourceKeys)[number];

export const changeUrlSources = ["stored", "quirks", "wellKnown"] as const;
export type ChangeUrlSource = (typeof changeUrlSources)[number];

export const rotationOutcomeKeys = ["succeeded", "failed"] as const;
export type RotationOutcomeKey = (typeof rotationOutcomeKeys)[number];

export const historyReasonKeys = ["replaced", "unconfirmed"] as const;
export type HistoryReasonKey = (typeof historyReasonKeys)[number];

export const breachScanErrors = ["offline", "rateLimited", "protocol", "cancelled"] as const;
export type BreachScanError = (typeof breachScanErrors)[number];

/** The snooze buttons' days. */
export const snoozeDays = [1, 7, 30] as const;
export type SnoozeDays = (typeof snoozeDays)[number];

/** The reminder intervals of an item. */
export const policyDays = [30, 90, 180, 365] as const;
export type PolicyDays = (typeof policyDays)[number];

/** An open change attempt, without its value. */
export interface RotationAttemptView {
  attemptId: string;
  stage: AttemptStage;
  startedAt: UnixMillis;
}

/** One entry of the password history, without its value. */
export interface HistoryEntryView {
  historyId: string;
  /** 0 = newest (what `reveal_secret` takes). */
  index: number;
  reason: HistoryReasonKey;
  changedAt: UnixMillis;
}

/** An https page to change the password on. Mosaic never opens or probes it. */
export interface ChangeUrlView {
  url: string;
  source: ChangeUrlSource;
}

export interface QueueEntry {
  vaultId: VaultId;
  itemId: ItemId;
  title: string | null;
  username: string | null;
  host: string | null;
  kind: QueueKind;
  reasons: QueueReason[];
  severity: InsightSeverity;
  attempts: RotationAttemptView[];
  /** Snoozed entries stay listed; dismissed ones do not. */
  snoozedUntil: UnixMillis | null;
}

export interface BreachStatus {
  mode: BreachMode;
  /** When the user agreed to the check; `null` while it is off. */
  consentAt: UnixMillis | null;
  lastFinishedAt: UnixMillis | null;
  running: boolean;
  lastError: BreachScanError | null;
}

export interface RotationQueue {
  /** More severe first, then unfinished ones, then by title. */
  entries: QueueEntry[];
  breach: BreachStatus;
}

export interface RotationBeginRequest {
  vaultId: VaultId;
  itemId: ItemId;
  /** 1 to 1024 characters. */
  newPassword: string;
  rulesSource: RulesSourceKey | null;
}

export interface RotationBegun {
  attemptId: string;
  /** `null` unless the login has an https URL. */
  changeUrl: ChangeUrlView | null;
}

/** `rotation_reveal` and `rotation_copy`. */
export interface RotationAttemptRequest {
  vaultId: VaultId;
  itemId: ItemId;
  attemptId: string;
}

export interface RotationCloseRequest {
  vaultId: VaultId;
  itemId: ItemId;
  attemptId: string;
  outcome: RotationOutcomeKey;
}

export interface HistoryRestoreRequest {
  vaultId: VaultId;
  itemId: ItemId;
  historyId: string;
}

export interface RotationSnoozeRequest {
  vaultId: VaultId;
  itemId: ItemId;
  days: SnoozeDays;
}

export interface RotationPolicyRequest {
  vaultId: VaultId;
  itemId: ItemId;
  /** `null` clears the reminder. */
  everyDays: PolicyDays | null;
}

/** The commands of this area (`Commands` in `commands.ts` extends it). */
export interface CredentialRotationCommands {
  // Credential rotation and the breach check
  rotation_queue: { request: null; response: RotationQueue };
  rotation_begin: { request: RotationBeginRequest; response: RotationBegun };
  rotation_close: { request: RotationCloseRequest; response: null };
  rotation_reveal: { request: RotationAttemptRequest; response: RevealedSecret };
  rotation_copy: { request: RotationAttemptRequest; response: CopyReceipt };
  password_history_restore: { request: HistoryRestoreRequest; response: null };
  rotation_snooze: { request: RotationSnoozeRequest; response: null };
  rotation_dismiss: { request: ItemRequest; response: null };
  rotation_undismiss: { request: ItemRequest; response: null };
  rotation_set_policy: { request: RotationPolicyRequest; response: null };
  breach_scan_start: { request: null; response: null };
  breach_scan_cancel: { request: null; response: null };
}
