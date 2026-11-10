/**
 * Mock credential rotation (M7b): the rotate-now queue, the guided change of a login, the
 * password history restore, snooze, dismissal, the reminder interval and the opt-in breach scan.
 * It follows `src-tauri/src/commands/credential_rotation.rs` and `breach.rs`:
 *
 * - `begin` stores the new password in the item before it returns; the current password stays;
 * - `close` succeeded swaps the values (old to the history as replaced, other open attempts as
 *   not confirmed), failed keeps the current password (the attempt's value goes to the history as
 *   not confirmed); no value is ever deleted;
 * - a dismissal and a breach finding hold only while the password is the one they were made for;
 * - the breach scan needs the consent (`settings.breachCheck` not `off` with `breachConsentAt`),
 *   finds the passwords of a small sample list and never leaves the mock.
 *
 * URL variants (`&credRotation=`): queue (weak, reused, old and breached items, one manual item),
 * empty, pending (an open change with a not-confirmed history entry), breachRunning (a scan that
 * stays at 3 of 9), breachOffline (the last scan could not reach the service), consent (a queue
 * with the breach check off, so "Check now" asks for the consent), and the three dialogs of the
 * browser extension (`mock/browser.ts`, 500 ms after the app subscribes): browserBegin (the
 * extension starts a change on GitHub), browserResult (it reports that an open change worked) and
 * addOrigin (it asks to add a related address to GitHub, which has an open change).
 */
import type {
  AttemptStage,
  BreachScanError,
  ChangeUrlView,
  HistoryEntryView,
  InsightIssue,
  InsightSeverity,
  QueueEntry,
  QueueReason,
  RotationQueue,
} from "../contract/index.ts";
import { queueReasons } from "../contract/index.ts";
import { breachedItems, mockSecurityReport } from "./insights.ts";
import { fail, requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

export const credRotationVariants = [
  "queue",
  "empty",
  "pending",
  "breachRunning",
  "breachOffline",
  "consent",
  "browserBegin",
  "browserResult",
  "addOrigin",
] as const;
export type CredRotationVariant = (typeof credRotationVariants)[number];

export interface MockCredRotationOptions {
  variant?: CredRotationVariant;
  /** Pause between the steps of a scan in ms (default 250). */
  stepMs?: number;
}

interface MockScan {
  running: boolean;
  done: number;
  total: number;
  lastFinishedAt: number | null;
  lastError: BreachScanError | null;
  timer: ReturnType<typeof setTimeout> | undefined;
}

export interface MockCredRotation {
  variant: CredRotationVariant | null;
  stepMs: number;
  /** Item ID to the time the snooze ends. */
  snoozed: Map<string, number>;
  /** Item ID to the password at the time of the dismissal. */
  dismissed: Map<string, string>;
  /** `${itemId}/${attemptId}` to the new password of the attempt. */
  pending: Map<string, string>;
  /** Item ID to the password the breach check found. */
  found: Map<string, string>;
  scan: MockScan;
  counter: number;
}

const DAY_MS = 86_400_000;
const MAX_ATTEMPTS = 4;
/** What the mock's breach check "finds". */
const BREACHED_PASSWORDS: readonly string[] = ["password", "123456", "hunter2", "Sommer2024!", "letmein"];

export function seedCredRotation(options: MockCredRotationOptions | undefined): MockCredRotation {
  return {
    variant: options?.variant ?? null,
    stepMs: options?.stepMs ?? 250,
    snoozed: new Map(),
    dismissed: new Map(),
    pending: new Map(),
    found: new Map(),
    scan: { running: false, done: 0, total: 0, lastFinishedAt: null, lastError: null, timer: undefined },
    counter: 0,
  };
}

function passwordOf(item: MockItem): string | undefined {
  return item.detail.summary.kind === "wifi" ? item.secrets.wifiPassword : item.secrets.password;
}

/** The change page of a login: its first https URL's well-known address (Rust `change_url::resolve`). */
export function changeUrlOf(item: MockItem): ChangeUrlView | null {
  if (item.detail.summary.kind !== "login") {
    return null;
  }
  for (const entry of item.detail.urls) {
    if (URL.canParse(entry.url) && new URL(entry.url).protocol === "https:") {
      const stored = item.detail.text.find((text) => text.field === "changePasswordUrl")?.value;
      if (stored !== undefined && URL.canParse(stored) && new URL(stored).protocol === "https:") {
        return { url: stored, source: "stored" };
      }
      return { url: `${new URL(entry.url).origin}/.well-known/change-password`, source: "wellKnown" };
    }
  }
  return null;
}

/** Keeps the derived fields of a detail current after any change. */
export function refreshDetail(item: MockItem): void {
  item.detail.changeUrl = changeUrlOf(item);
  item.detail.historyCount = item.history.length;
}

function find(state: MockState, request: { vaultId: string; itemId: string }): MockItem {
  const item = state.items.find(
    (each) =>
      each.detail.summary.vaultId === request.vaultId && each.detail.summary.itemId === request.itemId,
  );
  if (item === undefined || item.detail.summary.trashedAt !== null) {
    fail("itemNotFound");
  }
  return item;
}

export function pushHistory(state: MockState, item: MockItem, value: string, reason: HistoryEntryView["reason"]): void {
  state.credRotation.counter += 1;
  item.history.unshift(value);
  item.detail.history.unshift({
    historyId: `h${state.credRotation.counter.toString(16).padStart(16, "0")}.mock.0`,
    index: 0,
    reason,
    changedAt: state.now(),
  });
  item.detail.history.forEach((entry, index) => {
    entry.index = index;
  });
  refreshDetail(item);
}

/** A password changed: the cached answers and the dismissal were for the old one. */
export function passwordChanged(state: MockState, item: MockItem): void {
  state.credRotation.dismissed.delete(item.detail.summary.itemId);
  item.detail.summary.modifiedAt = state.now();
}

function attemptKey(itemId: string, attemptId: string): string {
  return `${itemId}/${attemptId}`;
}

export function startAttempt(state: MockState, item: MockItem, value: string, stage: AttemptStage): string {
  state.credRotation.counter += 1;
  const attemptId = `a${state.credRotation.counter.toString(16).padStart(16, "0")}.mock.0`;
  item.detail.rotationAttempts.push({ attemptId, stage, startedAt: state.now() });
  state.credRotation.pending.set(attemptKey(item.detail.summary.itemId, attemptId), value);
  return attemptId;
}

function attemptValue(state: MockState, item: MockItem, attemptId: string): string {
  const value = state.credRotation.pending.get(attemptKey(item.detail.summary.itemId, attemptId));
  if (value === undefined || !item.detail.rotationAttempts.some((a) => a.attemptId === attemptId)) {
    fail("rotationNotFound");
  }
  return value;
}

function dropAttempt(state: MockState, item: MockItem, attemptId: string): void {
  item.detail.rotationAttempts = item.detail.rotationAttempts.filter((a) => a.attemptId !== attemptId);
  state.credRotation.pending.delete(attemptKey(item.detail.summary.itemId, attemptId));
}

/** Closes an attempt as the desktop does: success swaps the values, failure files the value as not confirmed. */
export function closeAttempt(
  state: MockState,
  item: MockItem,
  attemptId: string,
  outcome: "succeeded" | "failed",
): void {
  if (outcome === "succeeded") {
    confirm(state, item, attemptId);
    return;
  }
  const value = attemptValue(state, item, attemptId);
  pushHistory(state, item, value, "unconfirmed");
  dropAttempt(state, item, attemptId);
}

function confirm(state: MockState, item: MockItem, attemptId: string): void {
  const value = attemptValue(state, item, attemptId);
  const old = item.secrets.password;
  for (const other of item.detail.rotationAttempts.filter((a) => a.attemptId !== attemptId)) {
    pushHistory(state, item, attemptValue(state, item, other.attemptId), "unconfirmed");
    dropAttempt(state, item, other.attemptId);
  }
  if (old !== undefined) {
    pushHistory(state, item, old, "replaced");
  }
  item.secrets.password = value;
  dropAttempt(state, item, attemptId);
  passwordChanged(state, item);
}

function reasonsOf(state: MockState, item: MockItem, issues: readonly { kind: string }[]): QueueReason[] {
  const kinds = new Set<string>(issues.map((issue) => issue.kind));
  if (item.detail.rotationAttempts.length > 0) {
    kinds.add("unfinished");
  }
  // `breached` also comes from the cached finding when the report is stale.
  if (breachedItems(state).has(item.detail.summary.itemId)) {
    kinds.add("breached");
  }
  return queueReasons.filter((reason) => kinds.has(reason));
}

const SEVERITY_RANK = { high: 0, medium: 1, low: 2 } as const;

function severityOf(issues: readonly { severity: InsightSeverity }[], reasons: readonly QueueReason[]): InsightSeverity {
  if (reasons.includes("breached") || issues.some((issue) => issue.severity === "high")) {
    return "high";
  }
  return issues[0]?.severity ?? "medium";
}

/** One queue row of an item, or `null` when the item is not listed. */
function queueEntry(
  state: MockState,
  item: MockItem,
  issues: readonly InsightIssue[],
  now: number,
): QueueEntry | null {
  const summary = item.detail.summary;
  if (summary.trashedAt !== null || item.detail.review.conflictCopyOf !== null) {
    return null;
  }
  const reasons = reasonsOf(state, item, issues);
  const open = item.detail.rotationAttempts.length > 0;
  // Dismissed until the password changes; an open change is never dismissed away.
  const dismissed = state.credRotation.dismissed.get(summary.itemId) === passwordOf(item);
  if (reasons.length === 0 || (dismissed && !open)) {
    return null;
  }
  const until = state.credRotation.snoozed.get(summary.itemId);
  return {
    vaultId: summary.vaultId,
    itemId: summary.itemId,
    title: summary.title,
    username: item.detail.text.find((text) => text.field === "username")?.value ?? null,
    host: summary.host,
    kind: item.detail.changeUrl !== null ? "guided" : "manual",
    reasons,
    severity: severityOf(issues, reasons),
    attempts: item.detail.rotationAttempts,
    snoozedUntil: until !== undefined && until > now ? until : null,
  };
}

function buildQueue(state: MockState): RotationQueue {
  const now = state.now();
  const byItem = new Map(mockSecurityReport(state).findings.map((finding) => [finding.itemId, finding]));
  const entries = state.items
    .map((item) => {
      const issues = (byItem.get(item.detail.summary.itemId)?.issues ?? []).filter(
        (issue) => issue.action === "changePassword",
      );
      return queueEntry(state, item, issues, now);
    })
    .filter((entry): entry is QueueEntry => entry !== null);
  entries.sort(
    (a, b) =>
      SEVERITY_RANK[a.severity] - SEVERITY_RANK[b.severity] ||
      Number(b.attempts.length > 0) - Number(a.attempts.length > 0) ||
      (a.title ?? "").localeCompare(b.title ?? ""),
  );
  return { entries, breach: breachStatus(state) };
}

function breachStatus(state: MockState): RotationQueue["breach"] {
  const { scan } = state.credRotation;
  return {
    mode: state.settings.breachCheck,
    consentAt: state.settings.breachConsentAt,
    lastFinishedAt: scan.lastFinishedAt,
    running: scan.running,
    lastError: scan.running ? null : scan.lastError,
  };
}

function finishScan(state: MockState, error: BreachScanError | null): void {
  const { scan } = state.credRotation;
  clearTimeout(scan.timer);
  scan.timer = undefined;
  scan.running = false;
  scan.lastError = error;
  let checked = 0;
  let found = 0;
  if (error !== "offline") {
    for (const item of state.items) {
      const password = passwordOf(item);
      const kind = item.detail.summary.kind;
      if (item.detail.summary.trashedAt !== null || password === undefined || (kind !== "login" && kind !== "wifi")) {
        continue;
      }
      checked += 1;
      if (BREACHED_PASSWORDS.includes(password)) {
        found += 1;
        state.credRotation.found.set(item.detail.summary.itemId, password);
      } else {
        state.credRotation.found.delete(item.detail.summary.itemId);
      }
    }
  }
  scan.lastFinishedAt = state.now();
  state.emit({ type: "breachScanFinished", checked, found, error });
}

function stepScan(state: MockState): void {
  const { scan } = state.credRotation;
  scan.done += 1;
  state.emit({ type: "breachScanProgress", done: scan.done, total: scan.total });
  if (scan.done >= scan.total) {
    finishScan(state, state.credRotation.variant === "breachOffline" ? "offline" : null);
    return;
  }
  scan.timer = setTimeout(() => {
    stepScan(state);
  }, state.credRotation.stepMs);
}

function startScan(state: MockState): void {
  const { scan } = state.credRotation;
  if (state.settings.breachCheck === "off" || state.settings.breachConsentAt === null) {
    fail("breachConsentRequired");
  }
  if (scan.running) {
    fail("breachRunning");
  }
  scan.running = true;
  scan.done = 0;
  scan.total = 3;
  scan.timer = setTimeout(() => {
    stepScan(state);
  }, state.credRotation.stepMs);
}

/** The scan's progress after (re)subscribing, for a scan that is running (`&credRotation=breachRunning`). */
export function announceCredRotation(state: MockState): void {
  const { scan } = state.credRotation;
  if (scan.running && scan.done > 0) {
    state.emit({ type: "breachScanProgress", done: scan.done, total: scan.total });
  }
}

export const credentialRotationHandlers: MockHandlers = {
  rotation_queue: (state) => {
    requireUnlocked(state);
    return buildQueue(state);
  },

  rotation_begin: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    refreshDetail(item);
    if (request.newPassword === "" || request.newPassword.length > 1024) {
      fail("invalidInput");
    }
    if (item.detail.changeUrl === null) {
      fail("rotationNotSupported");
    }
    if (item.detail.rotationAttempts.length >= MAX_ATTEMPTS) {
      fail("rotationLimit");
    }
    const attemptId = startAttempt(state, item, request.newPassword, "prepared");
    return { attemptId, changeUrl: item.detail.changeUrl };
  },

  rotation_close: (state, request) => {
    requireUnlocked(state);
    closeAttempt(state, find(state, request), request.attemptId, request.outcome);
    return null;
  },

  rotation_reveal: (state, request) => {
    requireUnlocked(state);
    return { value: attemptValue(state, find(state, request), request.attemptId), hideAfterSeconds: 60 };
  },

  rotation_copy: (state, request) => {
    requireUnlocked(state);
    attemptValue(state, find(state, request), request.attemptId);
    const seconds = state.settings.clipboardClearSeconds;
    return {
      clearsAt: seconds === null ? null : state.now() + seconds * 1000,
      clearAfterSeconds: seconds,
    };
  },

  password_history_restore: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    const at = item.detail.history.findIndex((entry) => entry.historyId === request.historyId);
    const value = item.history[at];
    if (at < 0 || value === undefined) {
      fail("historyNotFound");
    }
    item.history.splice(at, 1);
    item.detail.history.splice(at, 1);
    const current = item.secrets.password;
    item.secrets.password = value;
    if (current !== undefined) {
      pushHistory(state, item, current, "replaced");
    } else {
      refreshDetail(item);
    }
    passwordChanged(state, item);
    return null;
  },

  rotation_snooze: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    state.credRotation.snoozed.set(item.detail.summary.itemId, state.now() + request.days * DAY_MS);
    return null;
  },

  rotation_dismiss: (state, request) => {
    requireUnlocked(state);
    const item = find(state, request);
    const password = passwordOf(item);
    if (password === undefined) {
      fail("fieldEmpty");
    }
    state.credRotation.dismissed.set(item.detail.summary.itemId, password);
    return null;
  },

  rotation_set_policy: (state, request) => {
    requireUnlocked(state);
    find(state, request).detail.rotationPolicy = request.everyDays;
    return null;
  },

  breach_scan_start: (state) => {
    requireUnlocked(state);
    startScan(state);
    return null;
  },

  breach_scan_cancel: (state) => {
    requireUnlocked(state);
    if (state.credRotation.scan.running) {
      finishScan(state, "cancelled");
    }
    return null;
  },
};
