/** Security insights contract: mirrors `src-tauri/src/contract/insights.rs`. */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";

export const insightKinds = ["weak", "reused", "old", "missingTotp", "insecureUrl"] as const;
export type InsightKind = (typeof insightKinds)[number];

/** Ordered: `low < medium < high`. */
export const insightSeverities = ["low", "medium", "high"] as const;
export type InsightSeverity = (typeof insightSeverities)[number];

export const insightActions = ["changePassword", "setUpTotp", "reviewUrl"] as const;
export type InsightAction = (typeof insightActions)[number];

export interface InsightCounts {
  weak: number;
  reused: number;
  /** Groups of items sharing one password. */
  reusedGroups: number;
  old: number;
  missingTotp: number;
  insecureUrl: number;
  /** Items by their most severe finding. */
  high: number;
  medium: number;
  low: number;
}

export interface InsightIssue {
  kind: InsightKind;
  severity: InsightSeverity;
  action: InsightAction;
}

export interface InsightItemRef {
  vaultId: VaultId;
  itemId: ItemId;
}

/** A site from the bundled 2FA Directory data. */
export interface TotpSiteInfo {
  name: string;
  /** `https://` page about setting up 2FA; open only on a user action. */
  documentation: string | null;
}

export interface InsightFinding {
  vaultId: VaultId;
  itemId: ItemId;
  /** Most severe first, at most one per kind. */
  issues: InsightIssue[];
  /** For `reused`; empty otherwise. */
  reusedWith: InsightItemRef[];
  /** For `missingTotp`; `null` otherwise. */
  totpSite: TotpSiteInfo | null;
  /** For `insecureUrl`; `null` otherwise. */
  insecureHost: string | null;
}

export interface SecurityReport {
  generatedAt: UnixMillis;
  itemsChecked: number;
  counts: InsightCounts;
  /** Most severe first, then more findings first, then by vault and item ID. */
  findings: InsightFinding[];
}
