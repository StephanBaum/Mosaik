import { act, render, renderHook, screen, type RenderHookResult } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import { App } from "../App.tsx";
import { ToastProvider } from "../components/ToastProvider.tsx";
import { I18nProvider } from "../i18n/I18nProvider.tsx";
import { IpcProvider } from "../ipc/IpcProvider.tsx";
import { EventsProvider, useEvents } from "./EventsProvider.tsx";
import { SecretsProvider } from "./SecretsContext.tsx";
import type { MosaicIpc } from "../ipc/client.ts";
import type { AppEvent } from "../ipc/contract/index.ts";
import { createMockIpc } from "../ipc/mock.ts";

describe("settingsReset event", () => {
  it("tells the user once, in the chosen language", async () => {
    const mock = createMockIpc({ start: "unlocked" });
    const listeners: ((event: AppEvent) => void)[] = [];
    const ipc: MosaicIpc = {
      subscribe: (listener) => {
        listeners.push(listener);
        return mock.subscribe(listener);
      },
      call: (command, request) => mock.call(command, request),
    };
    render(<App ipc={ipc} languages={["de"]} />);
    await screen.findByText("GitHub");
    expect(screen.queryByText(/Sicherheitseinstellungen/)).toBeNull();
    act(() => {
      for (const listener of listeners) {
        listener({ type: "settingsReset" });
      }
    });
    expect(
      await screen.findByText(
        "Deine Sicherheitseinstellungen wurden außerhalb von Mosaic geändert und auf sichere Standardwerte zurückgesetzt.",
      ),
    ).toBeTruthy();
  });
});

describe("importFileDropped event", () => {
  it("keeps the token and the name until the import flow takes them, and drops them on lock", () => {
    const mock = createMockIpc({ start: "unlocked" });
    const listeners: ((event: AppEvent) => void)[] = [];
    const ipc: MosaicIpc = {
      subscribe: (listener) => {
        listeners.push(listener);
        return mock.subscribe(listener);
      },
      call: (command, request) => mock.call(command, request),
    };
    const wrapper = ({ children }: { children: ReactNode }) => (
      <I18nProvider locale="en">
        <IpcProvider ipc={ipc}>
          <ToastProvider>
            <SecretsProvider>
              <EventsProvider onLocked={() => undefined}>{children}</EventsProvider>
            </SecretsProvider>
          </ToastProvider>
        </IpcProvider>
      </I18nProvider>
    );
    const hook: RenderHookResult<ReturnType<typeof useEvents>, unknown> = renderHook(() => useEvents(), { wrapper });
    const emit = (event: AppEvent) => {
      act(() => {
        for (const listener of listeners) {
          listener(event);
        }
      });
    };
    return vi
      .waitFor(() => {
        expect(listeners.length).toBeGreaterThan(0);
      })
      .then(() => {
        expect(hook.result.current.importDrop).toBeNull();
        emit({ type: "importFileDropped", fileToken: "file-9", fileName: "x.csv" });
        expect(hook.result.current.importDrop).toEqual({ fileToken: "file-9", fileName: "x.csv" });
        act(() => {
          hook.result.current.clearImportDrop();
        });
        expect(hook.result.current.importDrop).toBeNull();
        emit({ type: "importFileDropped", fileToken: "file-10", fileName: "y.csv" });
        emit({ type: "locked", reason: "manual" });
        expect(hook.result.current.importDrop).toBeNull();
      });
  });
});
