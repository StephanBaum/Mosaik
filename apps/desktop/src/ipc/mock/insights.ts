/**
 * Mock security report. It follows the shape and ordering of `mosaic-insights`, with
 * simplified rules, so the dashboard can be built against realistic data:
 * - weak: a login or Wi-Fi password shorter than 12 characters, `high` below 8 (the real
 *   check estimates entropy and patterns),
 * - reused: the same password on two or more of those items (`high`),
 * - old: a login not modified for more than 365 days (`low`),
 * - missingTotp: a login without a one-time code (its own or an `otp` custom field) for a
 *   site in a short sample list (`medium`; the real check uses the bundled 2FA Directory
 *   data and lowers it to `low` when a passkey exists for the site),
 * - insecureUrl: a login URL starting with `http://`, except loopback (`medium`; `low` if
 *   every such host is on the local network).
 *
 * Ordering, as in Rust: issues most severe first, then by kind; findings most severe
 * first, then more issues first, then by vault and item ID.
 */
import type {
  InsightAction,
  InsightCounts,
  InsightFinding,
  InsightIssue,
  InsightKind,
  InsightSeverity,
  SecurityReport,
  TotpSiteInfo,
} from "../contract/index.ts";
import { requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

const DAY_MS = 86_400_000;
const MAX_AGE_MS = 365 * DAY_MS;
const SAMPLE_TOTP_SITES: ReadonlyMap<string, TotpSiteInfo> = new Map([
  ["github.com", { name: "GitHub", documentation: "https://docs.github.com/en/authentication" }],
  ["gitlab.com", { name: "GitLab", documentation: null }],
  ["figma.com", { name: "Figma", documentation: null }],
  ["dropbox.com", { name: "Dropbox", documentation: null }],
  ["amazon.com", { name: "Amazon", documentation: null }],
]);
const KIND_ORDER: readonly InsightKind[] = ["weak", "reused", "old", "missingTotp", "insecureUrl"];
const SEVERITY_RANK: Readonly<Record<InsightSeverity, number>> = { low: 0, medium: 1, high: 2 };
const ACTIONS: Readonly<Record<InsightKind, InsightAction>> = {
  weak: "changePassword",
  reused: "changePassword",
  old: "changePassword",
  missingTotp: "setUpTotp",
  insecureUrl: "reviewUrl",
};
const LOCAL_SUFFIXES = [".local", ".lan", ".home", ".home.arpa", ".internal", ".intranet", ".corp"];

function secretOf(item: MockItem): string | null {
  const kind = item.detail.summary.kind;
  const value =
    kind === "login"
      ? item.secrets.password
      : kind === "wifi"
        ? item.secrets.wifiPassword
        : undefined;
  return value === undefined || value === "" ? null : value;
}

function hostOf(url: string): string | null {
  try {
    return new URL(url).hostname.toLowerCase();
  } catch {
    return null;
  }
}

function totpSiteOf(host: string): TotpSiteInfo | null {
  for (const [domain, site] of SAMPLE_TOTP_SITES) {
    if (host === domain || host.endsWith(`.${domain}`)) {
      return site;
    }
  }
  return null;
}

function hasOtp(item: MockItem): boolean {
  return item.detail.otp !== null || item.detail.customFields.some((f) => f.kind === "otp");
}

function isLoopback(host: string): boolean {
  return host === "localhost" || host.startsWith("127.") || host === "[::1]";
}

function isLocalNetwork(host: string): boolean {
  const octets = host.split(".").map(Number);
  if (octets.length === 4 && octets.every((o) => Number.isInteger(o) && o >= 0 && o <= 255)) {
    const [a = 0, b = 0] = octets;
    return a === 10 || (a === 172 && b >= 16 && b <= 31) || (a === 192 && b === 168) || (a === 169 && b === 254);
  }
  return !host.includes(".") || host === "fritz.box" || LOCAL_SUFFIXES.some((s) => host.endsWith(s));
}

/** Code-unit order, as Rust compares the IDs (not the locale's collation). */
function cmp(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}

function maxSeverity(issues: readonly InsightIssue[]): number {
  return Math.max(-1, ...issues.map((i) => SEVERITY_RANK[i.severity]));
}

export function mockSecurityReport(state: MockState): SecurityReport {
  const now = state.now();
  const checked = state.items.filter(
    (item) => item.detail.summary.trashedAt === null && item.detail.review.conflictCopyOf === null,
  );
  const entries = new Map<string, InsightFinding>();
  const entry = (item: MockItem): InsightFinding => {
    const id = item.detail.summary.itemId;
    const existing = entries.get(id);
    if (existing !== undefined) {
      return existing;
    }
    const created: InsightFinding = {
      vaultId: item.detail.summary.vaultId,
      itemId: id,
      issues: [],
      reusedWith: [],
      totpSite: null,
      insecureHost: null,
    };
    entries.set(id, created);
    return created;
  };
  const mark = (item: MockItem, kind: InsightKind, severity: InsightSeverity): InsightFinding => {
    const e = entry(item);
    if (!e.issues.some((i) => i.kind === kind)) {
      e.issues.push({ kind, severity, action: ACTIONS[kind] });
    }
    return e;
  };

  const byPassword = new Map<string, MockItem[]>();
  for (const item of checked) {
    const secret = secretOf(item);
    if (secret === null) {
      continue;
    }
    if (secret.length < 12) {
      mark(item, "weak", secret.length < 8 ? "high" : "medium");
    }
    byPassword.set(secret, [...(byPassword.get(secret) ?? []), item]);
  }
  let reusedGroups = 0;
  for (const group of byPassword.values()) {
    if (group.length < 2) {
      continue;
    }
    reusedGroups += 1;
    for (const item of group) {
      mark(item, "reused", "high").reusedWith = group
        .filter((other) => other !== item)
        .map((other) => ({ vaultId: other.detail.summary.vaultId, itemId: other.detail.summary.itemId }))
        .sort((a, b) => cmp(a.vaultId, b.vaultId) || cmp(a.itemId, b.itemId));
    }
  }

  for (const item of checked) {
    if (item.detail.summary.kind !== "login") {
      continue;
    }
    const modifiedAt = item.detail.summary.modifiedAt;
    if (secretOf(item) !== null && modifiedAt !== null && now - modifiedAt > MAX_AGE_MS) {
      mark(item, "old", "low");
    }
    const urls = item.detail.urls.map((u) => u.url.trim());
    if (!hasOtp(item)) {
      const site = urls
        .map(hostOf)
        .map((h) => (h === null ? null : totpSiteOf(h)))
        .find((s) => s !== null);
      if (site !== undefined) {
        mark(item, "missingTotp", "medium").totpSite = site;
      }
    }
    const httpHosts = urls
      .filter((url) => url.toLowerCase().startsWith("http://"))
      .map(hostOf)
      .filter((h): h is string => h !== null && !isLoopback(h));
    if (httpHosts.length > 0) {
      const local = httpHosts.every(isLocalNetwork);
      mark(item, "insecureUrl", local ? "low" : "medium").insecureHost =
        httpHosts.find((h) => !isLocalNetwork(h)) ?? httpHosts[0] ?? null;
    }
  }

  const counts: InsightCounts = {
    weak: 0,
    reused: 0,
    reusedGroups,
    old: 0,
    missingTotp: 0,
    insecureUrl: 0,
    high: 0,
    medium: 0,
    low: 0,
  };
  const findings = [...entries.values()];
  for (const finding of findings) {
    finding.issues.sort(
      (a, b) =>
        SEVERITY_RANK[b.severity] - SEVERITY_RANK[a.severity] ||
        KIND_ORDER.indexOf(a.kind) - KIND_ORDER.indexOf(b.kind),
    );
    for (const issue of finding.issues) {
      counts[issue.kind] += 1;
    }
    const top = finding.issues[0];
    if (top !== undefined) {
      counts[top.severity] += 1;
    }
  }
  findings.sort(
    (a, b) =>
      maxSeverity(b.issues) - maxSeverity(a.issues) ||
      b.issues.length - a.issues.length ||
      cmp(a.vaultId, b.vaultId) ||
      cmp(a.itemId, b.itemId),
  );

  return { generatedAt: now, itemsChecked: checked.length, counts, findings };
}

export const insightHandlers: MockHandlers = {
  security_report: (state) => {
    requireUnlocked(state);
    return mockSecurityReport(state);
  },
};
