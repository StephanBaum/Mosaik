/**
 * Mock picker windows (ADR-0164), for `vite --mode mock` and the UX review. Open `picker.html` or
 * `chip.html` with `?mock&mockPicker=list|listMany|chip|chipLocked|link|linkUnsigned|none|notice`:
 * the page shows that state and answers its commands the way Rust does (generations, the guard
 * times, two confirmations for an unsigned program). No password exists here, like in the real
 * contract.
 */
import type { PickerIpc } from "../../picker/ipc.ts";
import type {
  CommandName,
  IpcResult,
  LinkStep,
  PickerLogin,
  PickerNotice,
  PickerState,
  ProgramView,
  RequestOf,
  ResponseOf,
} from "../contract/index.ts";

export type PickerVariant =
  | "list"
  | "listMany"
  | "chip"
  | "chipLocked"
  | "link"
  | "linkUnsigned"
  | "none"
  | "notice";

const VARIANTS: readonly PickerVariant[] = [
  "list",
  "listMany",
  "chip",
  "chipLocked",
  "link",
  "linkUnsigned",
  "none",
  "notice",
];

const LINK_GUARD_MS = 1_000;

const SLACK: ProgramView = {
  kind: "signed",
  exe: "slack.exe",
  name: "Slack",
  signer: "Slack Technologies",
  path: null,
};

const TOOL: ProgramView = {
  kind: "unsigned",
  exe: "tool.exe",
  name: "tool.exe",
  signer: null,
  path: "C:\\Users\\alex\\AppData\\Local\\Tool\\tool.exe",
};

function login(n: number, title: string, username: string | null): PickerLogin {
  return {
    vaultId: "019a2b3c-4d5e-7001-8000-000000000001",
    itemId: `019a2b3c-4d5e-7002-8000-${String(n).padStart(12, "0")}`,
    title,
    username,
    vaultName: "Personal",
    host: null,
  };
}

const SAMPLE_LOGINS = [
  login(1, "Slack (work)", "alex@example.com"),
  login(2, "Slack (side project)", "alex@example.org"),
  login(3, "Bank", "alex"),
  login(4, "Tool licence", null),
];

export function mockPickerFromUrl(search: string): PickerVariant | null {
  const wanted = new URLSearchParams(search).get("mockPicker");
  return VARIANTS.find((variant) => variant === wanted) ?? null;
}

function base(): PickerState {
  return {
    generation: 1,
    mode: "list",
    locked: false,
    language: "system",
    theme: "system",
    program: SLACK,
    matches: [],
    preselected: null,
    suggestions: [],
    linkAgain: null,
    link: null,
    notice: null,
  };
}

function linkStep(program: ProgramView, target: PickerLogin): LinkStep {
  return {
    login: target,
    program,
    confirmations: program.kind === "unsigned" ? 2 : 1,
    confirmed: 0,
    lookalikeOf: program.kind === "unsigned" ? "toolz.exe" : null,
    hashChange: null,
    submit: true,
  };
}

function initial(variant: PickerVariant): PickerState {
  const state = base();
  switch (variant) {
    case "list":
      return { ...state, matches: [SAMPLE_LOGINS[0] as PickerLogin], preselected: 0 };
    case "listMany":
      return { ...state, matches: SAMPLE_LOGINS.slice(0, 3) };
    case "chip":
      return { ...state, mode: "chip" };
    case "chipLocked":
      return { ...state, mode: "chip", locked: true };
    case "none":
      return { ...state, suggestions: SAMPLE_LOGINS.slice(0, 2), linkAgain: "fileChanged", program: TOOL };
    case "link":
      return { ...state, link: linkStep(SLACK, SAMPLE_LOGINS[2] as PickerLogin) };
    case "linkUnsigned":
      return { ...state, program: TOOL, link: linkStep(TOOL, SAMPLE_LOGINS[3] as PickerLogin) };
    case "notice":
      return { ...state, mode: "notice", notice: "modifiersHeld" satisfies PickerNotice };
  }
}

type Handler<K extends CommandName> = (request: RequestOf<K>) => IpcResult<ResponseOf<K>>;

/** The picker's mock IPC. `variant` null (no `mockPicker` in the URL) shows the list. */
// eslint-disable-next-line max-lines-per-function -- one handler per picker command: the list is the mock
export function createMockPickerIpc(variant: PickerVariant | null): PickerIpc {
  let state = initial(variant ?? "list");
  const listeners = new Set<(generation: number) => void>();
  let shownAt = Date.now();
  let stepAt = Date.now();
  let firstAt = 0;

  const changed = (next: PickerState): void => {
    state = { ...next, generation: state.generation + 1 };
    for (const listener of listeners) {
      listener(state.generation);
    }
  };
  const stale = <T>(generation: number): IpcResult<T> | null =>
    generation === state.generation ? null : { ok: false, error: { code: "pickerStale" } };

  const handlers: { [K in CommandName]?: Handler<K> } = {
    picker_state: () => ({ ok: true, value: state }),
    picker_chip_click: (request) => {
      const refused = stale<null>(request.generation);
      if (refused !== null) {
        return refused;
      }
      shownAt = Date.now();
      changed({ ...state, mode: "list", matches: [SAMPLE_LOGINS[0] as PickerLogin], preselected: 0 });
      return { ok: true, value: null };
    },
    picker_search: (request) => {
      const needle = request.query.trim().toLowerCase();
      const hits = SAMPLE_LOGINS.filter(
        (item) =>
          needle !== "" &&
          (item.title.toLowerCase().includes(needle) || (item.username ?? "").toLowerCase().includes(needle)),
      );
      return { ok: true, value: hits };
    },
    picker_choose: (request) => {
      const refused = stale<null>(request.generation);
      if (refused !== null) {
        return refused;
      }
      if (Date.now() - shownAt < 300) {
        return { ok: false, error: { code: "pickerTooSoon" } };
      }
      changed({ ...state, mode: "hidden", matches: [], preselected: null });
      return { ok: true, value: null };
    },
    picker_link_begin: (request) => {
      const refused = stale<LinkStep>(request.generation);
      if (refused !== null) {
        return refused;
      }
      const target = SAMPLE_LOGINS.find((item) => item.itemId === request.itemId);
      if (target === undefined) {
        return { ok: false, error: { code: "invalidInput" } };
      }
      const step = linkStep(state.program ?? SLACK, target);
      stepAt = Date.now();
      firstAt = 0;
      changed({ ...state, link: step });
      return { ok: true, value: step };
    },
    picker_link_confirm: (request) => {
      const refused = stale<{ kind: "needAnother" | "tooSoon" | "linked" }>(request.generation);
      const step = state.link;
      if (refused !== null) {
        return refused;
      }
      if (step === null) {
        return { ok: false, error: { code: "invalidInput" } };
      }
      const now = Date.now();
      if (now - stepAt < LINK_GUARD_MS) {
        return { ok: false, error: { code: "pickerTooSoon" } };
      }
      if (step.confirmations === 1) {
        changed({ ...state, mode: "hidden", link: null });
        return { ok: true, value: { kind: "linked" } };
      }
      if (firstAt === 0) {
        firstAt = now;
        changed({ ...state, link: { ...step, confirmed: 1 } });
        return { ok: true, value: { kind: "needAnother" } };
      }
      if (now - firstAt < LINK_GUARD_MS) {
        return { ok: true, value: { kind: "tooSoon" } };
      }
      changed({ ...state, mode: "hidden", link: null });
      return { ok: true, value: { kind: "linked" } };
    },
    picker_block: (request) => {
      const refused = stale<null>(request.generation);
      if (refused !== null) {
        return refused;
      }
      changed({ ...state, mode: "hidden", matches: [], link: null });
      return { ok: true, value: null };
    },
    picker_dismiss: (request) => {
      const refused = stale<null>(request.generation);
      if (refused !== null) {
        return refused;
      }
      changed({ ...state, mode: "hidden", matches: [], link: null, notice: null });
      return { ok: true, value: null };
    },
  };

  return {
    call: <K extends CommandName>(command: K, request: RequestOf<K>) => {
      const handler: Handler<K> | undefined = handlers[command];
      return Promise.resolve(
        handler === undefined ? ({ ok: false, error: { code: "notImplemented" } } as const) : handler(request),
      );
    },
    subscribe: (onChanged) => {
      listeners.add(onChanged);
      return Promise.resolve(() => {
        listeners.delete(onChanged);
      });
    },
  };
}
