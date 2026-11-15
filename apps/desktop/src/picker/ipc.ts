/**
 * The IPC of a picker window (ADR-0164): the shared command call and one `Channel` of `changed`
 * events through `picker_subscribe`. The picker windows have their own capability, so only the
 * picker commands work from here; none of them carries a secret.
 */
import { Channel, invoke } from "@tauri-apps/api/core";
import type { MosaicIpc } from "../ipc/client.ts";
import type { PickerEvent } from "../ipc/contract/index.ts";
import { call } from "../ipc/tauri.ts";

export interface PickerIpc {
  call: MosaicIpc["call"];
  /** Calls `onChanged` with the new generation whenever the state changed; resolves to an unsubscribe. */
  subscribe(onChanged: (generation: number) => void): Promise<() => void>;
}

export function createPickerIpc(): PickerIpc {
  return {
    call,
    async subscribe(onChanged) {
      let live = true;
      const channel = new Channel<PickerEvent>();
      channel.onmessage = (event) => {
        if (live) {
          onChanged(event.generation);
        }
      };
      // The latest subscription of a window wins in Rust, so a new one replaces this one.
      await invoke("picker_subscribe", { channel });
      return () => {
        live = false;
      };
    },
  };
}

/** The real picker IPC, or in `vite --mode mock` the mock one (`?mockPicker=list|chip|link|notice`). */
export async function loadPickerIpc(): Promise<PickerIpc> {
  if (import.meta.env.MODE === "mock") {
    const { createMockPickerIpc, mockPickerFromUrl } = await import("../ipc/mock.ts");
    return createMockPickerIpc(mockPickerFromUrl(window.location.search));
  }
  return createPickerIpc();
}
