/** A file dropped onto the window: Rust holds its path behind a token; the import flow takes the token. */
import { useCallback, useMemo, useState } from "react";
import type { AppEvent } from "../ipc/contract/index.ts";

export interface ImportDrop {
  fileToken: string;
  fileName: string;
}

export interface ImportDropEvents {
  /** The last dropped file until the import flow takes it (a token, never a path). */
  drop: ImportDrop | null;
  /** Takes the event if it is a drop. */
  apply: (event: AppEvent) => boolean;
  /** The flow took it, or a lock dropped it. */
  clear: () => void;
}

export function useImportDrop(): ImportDropEvents {
  const [drop, setDrop] = useState<ImportDrop | null>(null);
  const apply = useCallback((event: AppEvent): boolean => {
    if (event.type !== "importFileDropped") {
      return false;
    }
    setDrop({ fileToken: event.fileToken, fileName: event.fileName });
    return true;
  }, []);
  const clear = useCallback(() => {
    setDrop(null);
  }, []);
  return useMemo(() => ({ drop, apply, clear }), [drop, apply, clear]);
}
