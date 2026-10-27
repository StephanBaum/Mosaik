/**
 * Mock security report. It follows the shape and ordering of `mosaic-insights`, with
 * simplified rules, so the dashboard can be built against realistic data:
 * - weak: a login or Wi-Fi password shorter than 12 characters (the real check estimates
 *   entropy and patterns),
 * - reused: the same password on two or more of those items,
 * - old: a login not modified for more than 365 days,
 * - missingTotp: a login without a one-time code for a site in a short sample list (the
 *   real check uses the bundled 2FA Directory data),
 * - insecureUrl: a login URL starting with `http://`, except localhost.
 */
import type {
  InsightCounts,
  InsightFinding,
  InsightKind,
  SecurityReport,
} from "../contract/index.ts";
import { requireUnlocked, type MockHandlers, type MockItem, type MockState } from "./state.ts";

const DAY_MS = 86_400_000;
const MAX_AGE_MS = 365 * DAY_MS;
const SAMPLE_TOTP_SITES = ["github.com", "gitlab.com", "figma.com", "dropbox.com", "amazon.com"];
const KIND_ORDER: readonly InsightKind[] = ["weak", "reused", "old", "missingTotp", "insecureUrl"];

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

function supportsTotp(host: string): boolean {
  return SAMPLE_TOTP_SITES.some((site) => host === site || host.endsWith(`.${site}`));
}

export function mockSecurityReport(state: MockState): SecurityReport {
  const now = state.now();
  const checked = state.items.filter(
    (item) => item.detail.summary.trashedAt === null && item.detail.review.conflictCopyOf === null,
  );
  const kinds = new Map<string, Set<InsightKind>>();
  const reusedWith = new Map<string, string[]>();
  const mark = (item: MockItem, kind: InsightKind): void => {
    const id = item.detail.summary.itemId;
    const set = kinds.get(id) ?? new Set<InsightKind>();
    set.add(kind);
    kinds.set(id, set);
  };

  const byPassword = new Map<string, MockItem[]>();
  for (const item of checked) {
    const secret = secretOf(item);
    if (secret === null) {
      continue;
    }
    if (secret.length < 12) {
      mark(item, "weak");
    }
    byPassword.set(secret, [...(byPassword.get(secret) ?? []), item]);
  }
  for (const group of byPassword.values()) {
    if (group.length < 2) {
      continue;
    }
    for (const item of group) {
      mark(item, "reused");
      reusedWith.set(
        item.detail.summary.itemId,
        group
          .filter((other) => other !== item)
          .map((other) => other.detail.summary.itemId)
          .sort(),
      );
    }
  }

  for (const item of checked) {
    if (item.detail.summary.kind !== "login") {
      continue;
    }
    const modifiedAt = item.detail.summary.modifiedAt;
    if (secretOf(item) !== null && modifiedAt !== null && now - modifiedAt > MAX_AGE_MS) {
      mark(item, "old");
    }
    const urls = item.detail.urls.map((u) => u.url.trim());
    const hosts = urls.map(hostOf).filter((h): h is string => h !== null);
    if (item.detail.otp === null && hosts.some(supportsTotp)) {
      mark(item, "missingTotp");
    }
    const insecure = urls.some((url) => {
      const host = hostOf(url);
      return (
        url.toLowerCase().startsWith("http://") &&
        host !== null &&
        host !== "localhost" &&
        !host.startsWith("127.")
      );
    });
    if (insecure) {
      mark(item, "insecureUrl");
    }
  }

  const counts: InsightCounts = { weak: 0, reused: 0, old: 0, missingTotp: 0, insecureUrl: 0 };
  const findings: InsightFinding[] = [];
  for (const item of checked) {
    const set = kinds.get(item.detail.summary.itemId);
    if (set === undefined) {
      continue;
    }
    const ordered = KIND_ORDER.filter((kind) => set.has(kind));
    for (const kind of ordered) {
      counts[kind] += 1;
    }
    findings.push({
      vaultId: item.detail.summary.vaultId,
      itemId: item.detail.summary.itemId,
      kinds: ordered,
      reusedWith: reusedWith.get(item.detail.summary.itemId) ?? [],
    });
  }
  findings.sort((a, b) => b.kinds.length - a.kinds.length || a.itemId.localeCompare(b.itemId));

  return { generatedAt: now, itemsChecked: checked.length, counts, findings };
}

export const insightHandlers: MockHandlers = {
  security_report: (state) => {
    requireUnlocked(state);
    return mockSecurityReport(state);
  },
};
