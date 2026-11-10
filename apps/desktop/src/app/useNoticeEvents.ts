/** Events that become notices the user dismisses: vault metadata that lost a value in a merge (M9). */
import { useCallback, useMemo, useState } from "react";
import type { AppEvent, VaultId, VaultMetaField } from "../ipc/contract/index.ts";

export interface MetaDropped {
  vaultId: VaultId;
  fields: readonly VaultMetaField[];
}

export interface NoticeEvents {
  /** Vaults whose name, icon or description lost a value, until dismissed. */
  metaDropped: readonly MetaDropped[];
  dismissMetaDropped: (vaultId: VaultId) => void;
  /** Takes the event if it is one of ours. */
  apply: (event: AppEvent) => boolean;
  /** A lock drops the notices. */
  clear: () => void;
}

export function useNoticeEvents(): NoticeEvents {
  const [metaDropped, setMetaDropped] = useState<readonly MetaDropped[]>([]);
  const dismissMetaDropped = useCallback((vaultId: VaultId) => {
    setMetaDropped((all) => all.filter((entry) => entry.vaultId !== vaultId));
  }, []);
  const apply = useCallback((event: AppEvent): boolean => {
    if (event.type !== "vaultMetaDropped") {
      return false;
    }
    setMetaDropped((all) => [
      ...all.filter((entry) => entry.vaultId !== event.vaultId),
      { vaultId: event.vaultId, fields: event.fields },
    ]);
    return true;
  }, []);
  const clear = useCallback(() => {
    setMetaDropped([]);
  }, []);
  return useMemo(
    () => ({ metaDropped, dismissMetaDropped, apply, clear }),
    [metaDropped, dismissMetaDropped, apply, clear],
  );
}
