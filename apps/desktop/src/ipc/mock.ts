/**
 * The in-memory mock IPC for browser development (`pnpm dev:mock`), component tests and
 * Playwright. Handlers live per contract area in `mock/`; a command without a handler
 * fails with `notImplemented`, like its Rust stub.
 *
 * URL options in mock mode: `?mock=noAccount|locked|unlocked` picks the start state
 * (default `noAccount`; the sample account's master password is `mosaic`), and
 * `&latency=<ms>` delays every response to show loading states, and `&sshRequest=1` makes
 * the mock agent ask for an approval 500 ms after unlock.
 */
import type { EventListener, MosaicIpc } from "./client.ts";
import type { CommandName, IpcResult, RequestOf, ResponseOf } from "./contract/index.ts";
import { appHandlers } from "./mock/app.ts";
import { clipboardHandlers } from "./mock/clipboard.ts";
import { sampleAccount, sampleItems, sampleVaults } from "./mock/data.ts";
import { generatorHandlers } from "./mock/generator.ts";
import { importHandlers } from "./mock/import.ts";
import { insightHandlers } from "./mock/insights.ts";
import { itemHandlers } from "./mock/items.ts";
import { sessionHandlers } from "./mock/session.ts";
import { defaultSettings, settingsHandlers } from "./mock/settings.ts";
import { sshHandlers, startAgent } from "./mock/ssh.ts";
import { MockFailure, type MockHandlers, type MockState } from "./mock/state.ts";
import { totpHandlers } from "./mock/totp.ts";
import { vaultHandlers } from "./mock/vaults.ts";

export type MockStart = "noAccount" | "locked" | "unlocked";

export interface MockOptions {
  start?: MockStart;
  latencyMs?: number;
  /** Emit an SSH approval request 500 ms after unlock. */
  sshRequest?: boolean;
  /** Start with an unconfirmed Emergency Kit (`?kit=unverified`). */
  kitUnverified?: boolean;
  /** Start with vaults but no items (first run without an import). */
  emptyVault?: boolean;
  now?: () => number;
}

/** One handler set per contract area. */
const handlers: MockHandlers = {
  ...sessionHandlers,
  ...appHandlers,
  ...importHandlers,
  ...vaultHandlers,
  ...itemHandlers,
  ...clipboardHandlers,
  ...generatorHandlers,
  ...totpHandlers,
  ...settingsHandlers,
  ...insightHandlers,
  ...sshHandlers,
};

export function mockOptionsFromUrl(search: string): MockOptions {
  const params = new URLSearchParams(search);
  const start = params.get("mock");
  const latency = Number(params.get("latency") ?? "0");
  return {
    ...(start === "locked" || start === "unlocked" || start === "noAccount" ? { start } : {}),
    ...(Number.isFinite(latency) && latency > 0 ? { latencyMs: latency } : {}),
    ...(params.get("sshRequest") === "1" ? { sshRequest: true } : {}),
    ...(params.get("kit") === "unverified" ? { kitUnverified: true } : {}),
  };
}

export function createMockIpc(options: MockOptions = {}): MosaicIpc {
  const listeners = new Set<EventListener>();
  let counter = 0;
  const start = options.start ?? "noAccount";
  const state: MockState = {
    account: start === "noAccount" ? null : { ...sampleAccount },
    secretKeyStored: start !== "noAccount",
    unlocked: start === "unlocked",
    kitVerified: start !== "noAccount" && options.kitUnverified !== true,
    sshEnabled: false,
    heldImport: null,
    vaults: start === "noAccount" ? [] : sampleVaults(),
    items: start === "noAccount" || options.emptyVault === true ? [] : sampleItems(),
    settings: { ...defaultSettings },
    lastGenerated: null,
    emit: (event) => {
      for (const listener of listeners) {
        listener(event);
      }
    },
    sshRequest: options.sshRequest === true,
    now: options.now ?? (() => Date.now()),
    nextId: () => {
      counter += 1;
      return `019a2b3c-0000-7000-8000-${counter.toString(16).padStart(12, "0")}`;
    },
  };

  if (start === "unlocked") {
    startAgent(state);
  }

  async function call<K extends CommandName>(
    command: K,
    request: RequestOf<K>,
  ): Promise<IpcResult<ResponseOf<K>>> {
    if (options.latencyMs !== undefined) {
      await new Promise((resolve) => setTimeout(resolve, options.latencyMs));
    } else {
      await Promise.resolve();
    }
    const handler = handlers[command];
    if (handler === undefined) {
      return { ok: false, error: { code: "notImplemented" } };
    }
    try {
      // Responses are copied so the UI cannot mutate mock state by reference.
      return { ok: true, value: structuredClone(handler(state, request)) };
    } catch (error) {
      if (error instanceof MockFailure) {
        return { ok: false, error: { code: error.code } };
      }
      return { ok: false, error: { code: "internal" } };
    }
  }

  return {
    call,
    subscribe(listener) {
      listeners.add(listener);
      return Promise.resolve(() => {
        listeners.delete(listener);
      });
    },
  };
}
