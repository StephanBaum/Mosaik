/** Helpers of the password and key tests: the mock with a call record, and Settings opened on it. */
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "../../../App.tsx";
import type { MosaicIpc } from "../../../ipc/client.ts";
import type { CommandName, RequestOf } from "../../../ipc/contract/index.ts";
import { createMockIpc, type MockOptions } from "../../../ipc/mock.ts";

/** The mock with a record of every call. */
export function recording(options: MockOptions) {
  const inner = createMockIpc(options);
  const calls: { command: string; request: unknown }[] = [];
  const ipc: MosaicIpc = {
    subscribe: (listener) => inner.subscribe(listener),
    call: <K extends CommandName>(command: K, request: RequestOf<K>) => {
      calls.push({ command, request });
      return inner.call(command, request);
    },
  };
  return { ipc, calls };
}

/** Renders the app on the unlocked mock and opens Settings. */
export async function openSettings(options: MockOptions, language = "en") {
  const { ipc, calls } = recording({ start: "unlocked", rotation: { stepMs: 0 }, ...options });
  const user = userEvent.setup();
  render(<App ipc={ipc} languages={[language]} />);
  await screen.findByText("GitHub");
  await user.click(screen.getByRole("button", { name: language === "de" ? "Einstellungen" : "Settings" }));
  await screen.findByRole("heading", { name: language === "de" ? "Passwort und Schlüssel" : "Password and keys" });
  return { user, calls, ipc };
}

