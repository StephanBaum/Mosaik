import { act, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { App } from "../App.tsx";
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
