/** The state of a picker window: read on mount and again on every `changed` event. */
import { useEffect, useState } from "react";
import type { PickerState } from "../ipc/contract/index.ts";
import type { PickerIpc } from "./ipc.ts";

export function usePickerState(ipc: PickerIpc): PickerState | null {
  const [state, setState] = useState<PickerState | null>(null);

  useEffect(() => {
    let live = true;
    // A slow answer must not overwrite a newer one.
    let latest = 0;
    const read = async (): Promise<void> => {
      latest += 1;
      const mine = latest;
      const result = await ipc.call("picker_state", null);
      if (live && mine === latest && result.ok) {
        setState(result.value);
      }
    };
    let unsubscribe: (() => void) | null = null;
    void ipc.subscribe(() => void read()).then((off) => {
      if (live) {
        unsubscribe = off;
      } else {
        off();
      }
    });
    void read();
    return () => {
      live = false;
      unsubscribe?.();
    };
  }, [ipc]);

  return state;
}
