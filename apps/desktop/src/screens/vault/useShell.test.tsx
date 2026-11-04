import { renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { EventsProvider } from "../../app/EventsProvider.tsx";
import { SecretsProvider } from "../../app/SecretsContext.tsx";
import { ToastProvider } from "../../components/ToastProvider.tsx";
import { I18nProvider } from "../../i18n/I18nProvider.tsx";
import type { MosaicIpc } from "../../ipc/client.ts";
import type { CommandName, IpcResult, RequestOf, ResponseOf } from "../../ipc/contract/index.ts";
import { IpcProvider } from "../../ipc/IpcProvider.tsx";
import { createMockIpc } from "../../ipc/mock.ts";
import { RevealControlContext } from "../../reveal/context.ts";
import { useShell } from "./useShell.ts";

function setup(options: { sealed: boolean; failLock?: boolean }) {
  const order: string[] = [];
  const base = createMockIpc({ start: "unlocked" });
  const ipc: MosaicIpc = {
    subscribe: (listener) => base.subscribe(listener),
    call<K extends CommandName>(command: K, request: RequestOf<K>): Promise<IpcResult<ResponseOf<K>>> {
      if (command === "lock") {
        order.push("lock");
        if (options.failLock === true) {
          return Promise.resolve({ ok: false, error: { code: "internal" } });
        }
      }
      return base.call(command, request);
    },
  };
  const abortSeal = vi.fn();
  const control = {
    sealVault: () => {
      order.push("seal");
      return options.sealed;
    },
    abortSeal,
  };
  const onLocked = vi.fn();
  const wrapper = ({ children }: { children: ReactNode }) => (
    <I18nProvider locale="en">
      <IpcProvider ipc={ipc}>
        <ToastProvider>
          <SecretsProvider>
            <EventsProvider onLocked={() => undefined}>
              <RevealControlContext value={control}>{children}</RevealControlContext>
            </EventsProvider>
          </SecretsProvider>
        </ToastProvider>
      </IpcProvider>
    </I18nProvider>
  );
  const hook = renderHook(() => useShell(onLocked), { wrapper });
  return { order, abortSeal, onLocked, hook };
}

describe("useShell.lock", () => {
  it("sends the lock command first, then seals the vault, and reports the locked session", async () => {
    const { order, onLocked, abortSeal, hook } = setup({ sealed: true });
    hook.result.current.lock();
    expect(order).toEqual(["lock", "seal"]);
    await vi.waitFor(() => {
      expect(onLocked).toHaveBeenCalledWith(expect.objectContaining({ state: "locked" }));
    });
    expect(abortSeal).not.toHaveBeenCalled();
  });

  it("puts the sealed vault back when the lock fails", async () => {
    const { abortSeal, onLocked, hook } = setup({ sealed: true, failLock: true });
    hook.result.current.lock();
    await vi.waitFor(() => {
      expect(abortSeal).toHaveBeenCalledTimes(1);
    });
    expect(onLocked).not.toHaveBeenCalled();
  });

  it("leaves a failed lock alone when nothing was sealed", async () => {
    const { order, abortSeal, onLocked, hook } = setup({ sealed: false, failLock: true });
    hook.result.current.lock();
    await Promise.resolve();
    await Promise.resolve();
    expect(order).toEqual(["lock", "seal"]);
    expect(abortSeal).not.toHaveBeenCalled();
    expect(onLocked).not.toHaveBeenCalled();
  });
});
