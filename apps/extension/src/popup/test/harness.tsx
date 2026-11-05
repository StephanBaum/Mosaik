/**
 * Test harness of the popup components: a fake port that answers like the background (a table
 * of replies per op, every request recorded) and a render helper with a fake environment.
 */
import { render } from "@testing-library/react";
import { vi } from "vitest";
import type { ErrorCode } from "../../shared/errors.ts";
import type { PopupRequest, PopupState } from "../../shared/messages.ts";
import { App } from "../App.tsx";
import { createPopupClient, type PortLike } from "../client.ts";
import type { PopupEnv } from "../env.ts";

type Reply = ((request: PopupRequest) => unknown) | object | string | number | boolean | null;

export interface FakePort extends PortLike {
  /** Every request the popup sent, in order. */
  requests: PopupRequest[];
  /** What to answer per op; throw an `Error` whose message is an `ErrorCode` to fail. */
  replies: Partial<Record<PopupRequest["op"], Reply>>;
  /** Pushes an event as the background does. */
  push: (event: unknown) => void;
  disconnect: () => void;
}

export function fakePort(state: PopupState): FakePort {
  const messageListeners: ((message: unknown) => void)[] = [];
  const disconnectListeners: (() => void)[] = [];
  const port: FakePort = {
    requests: [],
    replies: { getState: () => state },
    postMessage: (message) => {
      const { id, request } = message as { id: number; request: PopupRequest };
      port.requests.push(request);
      const reply = port.replies[request.op];
      queueMicrotask(() => {
        try {
          const result = typeof reply === "function" ? (reply as (r: PopupRequest) => unknown)(request) : (reply ?? {});
          for (const listener of messageListeners) {
            listener({ id, ok: true, result });
          }
        } catch (error) {
          for (const listener of messageListeners) {
            listener({ id, ok: false, error: (error as Error).message as ErrorCode });
          }
        }
      });
    },
    onMessage: {
      addListener: (listener) => {
        messageListeners.push(listener);
      },
    },
    onDisconnect: {
      addListener: (listener) => {
        disconnectListeners.push(listener);
      },
    },
    push: (event) => {
      for (const listener of messageListeners) {
        listener(event);
      }
    },
    disconnect: () => {
      for (const listener of disconnectListeners) {
        listener();
      }
    },
  };
  return port;
}

export const baseState = (over: Partial<PopupState> = {}): PopupState => ({
  connection: "unlocked",
  pairCode: null,
  tab: { origin: "https://github.com", host: "github.com", unicodeHost: null, secure: true, supported: true },
  frame: "top",
  frameTarget: { frameId: 0, documentId: "doc-0" },
  matches: [],
  warnings: [],
  pendingSave: null,
  nextOtp: null,
  lastError: null,
  saveOffers: false,
  generatorFill: false,
  ...over,
});

export const match = (over: Partial<PopupState["matches"][number]> = {}): PopupState["matches"][number] => ({
  vaultId: "v1",
  itemId: "i1",
  title: "GitHub",
  username: "alex-dev",
  host: "github.com",
  hasOtp: false,
  favorite: false,
  matchedBy: "exactOrigin",
  blocked: null,
  ...over,
});

export type FakeEnv = PopupEnv & { closed: ReturnType<typeof vi.fn> };

export function fakeEnv(over: Partial<PopupEnv> = {}): FakeEnv {
  const closed = vi.fn();
  return {
    requestSiteAccess: () => Promise.resolve(true),
    removeSiteAccess: () => Promise.resolve(),
    fillShortcut: () => Promise.resolve("Ctrl+Shift+L"),
    closePopup: () => {
      closed();
      return Promise.resolve();
    },
    hostInstallCommand: "pnpm host:install",
    closed,
    ...over,
  };
}

export function renderPopup(
  state: PopupState,
  options: { languages?: readonly string[]; env?: PopupEnv; replies?: FakePort["replies"] } = {},
) {
  const port = fakePort(state);
  Object.assign(port.replies, options.replies ?? {});
  const env = (options.env ?? fakeEnv()) as FakeEnv;
  const utils = render(<App api={createPopupClient(port)} env={env} languages={options.languages ?? ["en"]} />);
  return { port, env, ...utils };
}

export const requestsOf = <Op extends PopupRequest["op"]>(port: FakePort, op: Op): Extract<PopupRequest, { op: Op }>[] =>
  port.requests.filter((request): request is Extract<PopupRequest, { op: Op }> => request.op === op);
