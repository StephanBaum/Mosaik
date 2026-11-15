/** Test helpers of the picker windows: a recording IPC and sample states (no password anywhere). */
import { vi } from "vitest";
import type { PickerIpc } from "../picker/ipc.ts";
import type {
  CommandName,
  IpcResult,
  LinkStep,
  PickerLogin,
  PickerState,
  ProgramView,
  RequestOf,
  ResponseOf,
} from "../ipc/contract/index.ts";

export const SLACK: ProgramView = {
  kind: "signed",
  exe: "slack.exe",
  name: "Slack",
  signer: "Slack Technologies",
  path: null,
};

export const TOOL: ProgramView = {
  kind: "unsigned",
  exe: "tool.exe",
  name: "tool.exe",
  signer: null,
  path: "C:\\Tools\\tool.exe",
};

export function sampleLogin(n: number, title: string, username: string | null = null): PickerLogin {
  return {
    vaultId: "019a2b3c-4d5e-7001-8000-000000000001",
    itemId: `019a2b3c-4d5e-7002-8000-${String(n).padStart(12, "0")}`,
    title,
    username,
    vaultName: "Personal",
    host: null,
  };
}

export function sampleState(overrides: Partial<PickerState> = {}): PickerState {
  return {
    generation: 7,
    mode: "list",
    locked: false,
    language: "en",
    theme: "system",
    program: SLACK,
    matches: [],
    preselected: null,
    suggestions: [],
    linkAgain: null,
    link: null,
    notice: null,
    ...overrides,
  };
}

export function sampleStep(overrides: Partial<LinkStep> = {}): LinkStep {
  return {
    login: sampleLogin(3, "Bank", "alex"),
    program: SLACK,
    confirmations: 1,
    confirmed: 0,
    lookalikeOf: null,
    hashChange: null,
    submit: true,
    ...overrides,
  };
}

/** The answers by command; loosely typed on purpose (a union over every command is too large for tsc). */
type Answers = Partial<Record<CommandName, IpcResult<unknown>>>;

export interface RecordedIpc extends PickerIpc {
  calls: { command: string; request: unknown }[];
  /** What `picker_subscribe` was given; call it to simulate a `changed` event. */
  emit: (generation: number) => void;
}

/** An IPC that records its calls and answers from `answers` (`ok: null` by default). */
export function recordingIpc(answers: Answers = {}): RecordedIpc {
  const calls: { command: string; request: unknown }[] = [];
  const listeners = new Set<(generation: number) => void>();
  return {
    calls,
    emit: (generation) => {
      for (const listener of listeners) {
        listener(generation);
      }
    },
    call: vi.fn(<K extends CommandName>(command: K, request: RequestOf<K>) => {
      calls.push({ command, request });
      const answer: IpcResult<unknown> = answers[command] ?? { ok: true, value: null };
      return Promise.resolve(answer as IpcResult<ResponseOf<K>>);
    }),
    subscribe: (onChanged) => {
      listeners.add(onChanged);
      return Promise.resolve(() => {
        listeners.delete(onChanged);
      });
    },
  };
}
