/** Data the vault shell needs besides the open list: vaults, category counts, findings. */
import { useCallback, useEffect, useState } from "react";
import { useIpc } from "../../ipc/IpcProvider.tsx";
import type { InsightFinding, ItemSummary, VaultId, VaultSummary } from "../../ipc/contract/index.ts";

export type Category =
  | "all"
  | "favorites"
  | "logins"
  | "passkeys"
  | "codes"
  | "cards"
  | "notes"
  | "ssh";

export const CATEGORIES: readonly Category[] = [
  "all",
  "favorites",
  "logins",
  "passkeys",
  "codes",
  "cards",
  "notes",
  "ssh",
];

/** Whether `item` belongs to `category` (pure; the sidebar counts and the list filter use it). */
export function matchesCategory(item: ItemSummary, category: Category): boolean {
  switch (category) {
    case "all":
      return true;
    case "favorites":
      return item.favorite;
    case "logins":
      return item.kind === "login";
    case "passkeys":
      return item.kind === "passkey";
    case "codes":
      return item.hasOtp || item.kind === "totp";
    case "cards":
      return item.kind === "creditCard";
    case "notes":
      return item.kind === "secureNote";
    case "ssh":
      return item.kind === "sshKey";
  }
}

export function categoryCounts(items: readonly ItemSummary[]): Record<Category, number> {
  const counts = Object.fromEntries(CATEGORIES.map((category) => [category, 0])) as Record<
    Category,
    number
  >;
  for (const item of items) {
    for (const category of CATEGORIES) {
      if (matchesCategory(item, category)) {
        counts[category] += 1;
      }
    }
  }
  return counts;
}

const NO_COUNTS = categoryCounts([]);

export function useVaults(reloadKey: number): VaultSummary[] {
  const ipc = useIpc();
  const [vaults, setVaults] = useState<VaultSummary[]>([]);
  useEffect(() => {
    void ipc.call("list_vaults", null).then((result) => {
      if (result.ok) {
        setVaults(result.value);
      }
    });
  }, [ipc, reloadKey]);
  return vaults;
}

/** An item that was saved: the list takes its row in place instead of loading again. */
export interface SavedItem {
  /** Counts the saves; a new value is a new save, also of the same item. */
  seq: number;
  summary: ItemSummary;
}

/** The list as the backend returned it (all categories), and what it was asked for. */
export interface FetchedList {
  items: ItemSummary[];
  vaultId: VaultId | null;
  /** No search and not the trash: every item of the vault, so the sidebar counts come from it. */
  unfiltered: boolean;
}

function before(a: ItemSummary, b: ItemSummary): boolean {
  // The backend lists by vault ID, then item ID (both UUIDs: the hyphenated text sorts like the bytes).
  return a.vaultId === b.vaultId ? a.itemId < b.itemId : a.vaultId < b.vaultId;
}

/**
 * The list after a save, without asking the backend: the row is replaced, removed (it left this
 * list) or, for a new item, inserted at its place. `null` when that cannot be done exactly (a
 * search or the trash is shown, or no list of this view is loaded): the caller loads again.
 */
export function applySaved(
  current: FetchedList | null,
  summary: ItemSummary,
  view: { vaultId: VaultId | null; trash: boolean; query: string },
): FetchedList | null {
  const unfiltered = view.query.trim() === "" && !view.trash;
  if (current === null || !unfiltered || !current.unfiltered || current.vaultId !== view.vaultId) {
    return null;
  }
  const items = current.items.slice();
  // A move keeps the item ID but changes the vault, so the old row is found by its ID alone.
  const index = items.findIndex((item) => item.itemId === summary.itemId);
  const belongs = summary.trashedAt === null && (view.vaultId === null || view.vaultId === summary.vaultId);
  if (!belongs) {
    if (index >= 0) {
      items.splice(index, 1);
    }
  } else if (index >= 0) {
    items[index] = summary;
  } else {
    const at = items.findIndex((item) => before(summary, item));
    items.splice(at < 0 ? items.length : at, 0, summary);
  }
  return { ...current, items };
}

/**
 * Item counts per category for the selected vault (trash excluded). While `fromList` is set the
 * item list feeds them through `feed` (it loads the same unfiltered list); otherwise they are
 * requested here.
 */
export function useCategoryCounts(vaultId: VaultId | null, reloadKey: number, fromList: boolean) {
  const ipc = useIpc();
  const [counts, setCounts] = useState(NO_COUNTS);
  const feed = useCallback((items: readonly ItemSummary[]) => {
    setCounts(categoryCounts(items));
  }, []);
  useEffect(() => {
    if (fromList) {
      return undefined;
    }
    let current = true;
    void ipc
      .call("list_items", { vaultId, trash: "exclude", favoritesOnly: false, query: null, limit: null })
      .then((result) => {
        if (current && result.ok) {
          setCounts(categoryCounts(result.value));
        }
      });
    return () => {
      current = false;
    };
  }, [ipc, vaultId, reloadKey, fromList]);
  return { counts, feed };
}

/**
 * The security findings: loaded once the item list has loaded (`ready`: the report is slower
 * than the list and must not delay the first paint) and again when the user leaves the
 * security view, not on every reload.
 */
export function useFindings(inSecurityView: boolean, ready: boolean): readonly InsightFinding[] {
  const ipc = useIpc();
  const [findings, setFindings] = useState<readonly InsightFinding[]>([]);
  useEffect(() => {
    if (inSecurityView || !ready) {
      return undefined;
    }
    let current = true;
    void ipc.call("security_report", null).then((result) => {
      if (current && result.ok) {
        setFindings(result.value.findings);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc, inSecurityView, ready]);
  return findings;
}

/** Whether the backend can use security keys (hides the hardware-vault option otherwise). */
export function useHardwareKeysSupported(): boolean {
  const ipc = useIpc();
  const [supported, setSupported] = useState(false);
  useEffect(() => {
    let current = true;
    void ipc.call("quick_unlock_status", null).then((result) => {
      if (current && result.ok) {
        setSupported(result.value.hardwareKeys.supported);
      }
    });
    return () => {
      current = false;
    };
  }, [ipc]);
  return supported;
}
