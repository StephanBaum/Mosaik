/** Security insights contract: mirrors `src-tauri/src/contract/insights.rs`. */
import type { ItemId, UnixMillis, VaultId } from "./common.ts";

export const insightKinds = ["weak", "reused", "old", "missingTotp", "insecureUrl"] as const;
export type InsightKind = (typeof insightKinds)[number];

export interface InsightCounts {
  weak: number;
  reused: number;
  old: number;
  missingTotp: number;
  insecureUrl: number;
}

export interface InsightFinding {
  vaultId: VaultId;
  itemId: ItemId;
  kinds: InsightKind[];
  reusedWith: ItemId[];
}

export interface SecurityReport {
  generatedAt: UnixMillis;
  itemsChecked: number;
  counts: InsightCounts;
  findings: InsightFinding[];
}
