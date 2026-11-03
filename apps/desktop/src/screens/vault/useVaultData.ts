/** Data the vault shell needs besides the open list: vaults, category counts, findings. */
import { useEffect, useState } from "react";
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

/** Item counts per category for the selected vault (trash excluded). */
export function useCategoryCounts(vaultId: VaultId | null, reloadKey: number) {
  const ipc = useIpc();
  const [counts, setCounts] = useState(NO_COUNTS);
  useEffect(() => {
    let current = true;
    void ipc
      .call("list_items", { vaultId, trash: "exclude", favoritesOnly: false, query: null })
      .then((result) => {
        if (current && result.ok) {
          setCounts(categoryCounts(result.value));
        }
      });
    return () => {
      current = false;
    };
  }, [ipc, vaultId, reloadKey]);
  return counts;
}

/**
 * The security findings: loaded once on mount and again when the user leaves the
 * security view, not on every reload.
 */
export function useFindings(inSecurityView: boolean): readonly InsightFinding[] {
  const ipc = useIpc();
  const [findings, setFindings] = useState<readonly InsightFinding[]>([]);
  useEffect(() => {
    if (inSecurityView) {
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
  }, [ipc, inSecurityView]);
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
