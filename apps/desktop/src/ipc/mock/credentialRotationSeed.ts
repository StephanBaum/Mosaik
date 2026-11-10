/**
 * The seeds of the credential rotation variants (`&credRotation=`, see `credentialRotation.ts`):
 * they change the sample items and the settings of a mock that already exists.
 */
import { pushHistory, refreshDetail, startAttempt } from "./credentialRotation.ts";
import type { MockItem, MockState } from "./state.ts";

const DAY_MS = 86_400_000;

function byTitle(state: MockState, title: string): MockItem | undefined {
  return state.items.find((item) => item.detail.summary.title === title);
}

/** Weak, reused and old passwords, and one Wi-Fi password Mosaic cannot guide a change for. */
function seedFindings(state: MockState, now: number): void {
  const figma = byTitle(state, "Figma");
  const jira = byTitle(state, "Jira");
  const github = byTitle(state, "GitHub");
  const wifi = byTitle(state, "Wi-Fi at home");
  if (figma !== undefined && jira !== undefined) {
    figma.secrets.password = "Sommer2024!";
    jira.secrets.password = "Sommer2024!";
  }
  if (github !== undefined) {
    github.detail.summary.modifiedAt = now - 400 * DAY_MS;
  }
  if (wifi !== undefined) {
    wifi.secrets.wifiPassword = "mosaic5g";
  }
}

/** An open change on GitHub, with a value from an earlier change that was never confirmed. */
function seedPending(state: MockState, now: number): void {
  const github = byTitle(state, "GitHub");
  if (github === undefined) {
    return;
  }
  pushHistory(state, github, `${github.secrets.password ?? ""}-v1`, "unconfirmed");
  startAttempt(state, github, "Xy7@mB3^dF6!hJ9#zC2$", "delivered");
  const entry = github.detail.history[0];
  if (entry !== undefined) {
    entry.changedAt = now - DAY_MS;
  }
}

/** The breach check is on (agreed 20 days ago), with GitHub's password found by the last scan. */
function seedBreach(state: MockState, now: number): void {
  const variant = state.credRotation.variant;
  state.settings = {
    ...state.settings,
    breachCheck: variant === "breachRunning" ? "weekly" : "manual",
    breachConsentAt: now - 20 * DAY_MS,
  };
  state.credRotation.scan.lastFinishedAt = now - 3 * DAY_MS;
  const github = byTitle(state, "GitHub");
  if (github !== undefined && variant !== "breachRunning") {
    state.credRotation.found.set(github.detail.summary.itemId, github.secrets.password ?? "");
  }
  if (variant === "breachOffline") {
    state.credRotation.scan.lastError = "offline";
  }
  if (variant === "breachRunning") {
    state.credRotation.scan = { ...state.credRotation.scan, running: true, done: 3, total: 9 };
  }
}

/** The date of the sample items (`data.ts`): a fixed day in 2025. */
const SAMPLE_MODIFIED_AT = 1_761_480_000_000;

/** Seeds the items and settings of a variant after the state exists. */
export function applyCredRotationSeed(state: MockState): void {
  const now = state.now();
  for (const item of state.items) {
    refreshDetail(item);
    // The samples carry a fixed date that grows old with the real clock; "changed 30 days ago" keeps the
    // default mock free of reminders. A variant seeds what it needs (GitHub's old password) on top.
    if (item.detail.summary.modifiedAt === SAMPLE_MODIFIED_AT) {
      item.detail.summary.modifiedAt = now - 30 * DAY_MS;
    }
  }
  const variant = state.credRotation.variant;
  if (variant === null || variant === "empty") {
    return;
  }
  seedFindings(state, now);
  if (variant === "pending" || variant === "browserResult" || variant === "addOrigin") {
    seedPending(state, now);
  }
  if (variant !== "consent" && variant !== "pending") {
    seedBreach(state, now);
  }
}
