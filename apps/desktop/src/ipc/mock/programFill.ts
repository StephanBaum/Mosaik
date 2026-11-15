/**
 * Mock program fill (ADR-0164): the status and the block list of the settings screen.
 * `?programFill=blocked` starts with two blocked programs; `notWindows`, `mosaicElevated` and
 * `hostFailed` start with program fill unavailable for that reason. The picker windows' own
 * mock is `mock/picker.ts`.
 */
import type { BlockedProgram, ProgramFillStatus, ProgramFillUnavailable } from "../contract/index.ts";
import type { MockHandlers, MockState } from "./state.ts";

export type ProgramFillVariant = "blocked" | ProgramFillUnavailable;

export interface MockProgramFill {
  blocked: BlockedProgram[];
  reason: ProgramFillUnavailable | null;
}

export function defaultProgramFill(): MockProgramFill {
  return { blocked: [], reason: null };
}

export function seedProgramFill(variant: ProgramFillVariant | undefined): MockProgramFill {
  if (variant === undefined) {
    return defaultProgramFill();
  }
  if (variant === "blocked") {
    return {
      blocked: [
        { key: "signed:notepad.exe:AAAA", label: "Notepad" },
        { key: "path:c:/tools/legacy.exe", label: "legacy.exe" },
      ],
      reason: null,
    };
  }
  return { blocked: [], reason: variant };
}

export function programFillStatusOf(state: MockState): ProgramFillStatus {
  const { reason, blocked } = state.programFill;
  return {
    available: reason === null,
    reason,
    running: reason === null && state.unlocked && state.settings.programFill,
    blocked: [...blocked],
  };
}

export const programFillHandlers: MockHandlers = {
  program_fill_status: (state) => programFillStatusOf(state),

  program_fill_unblock: (state, request) => {
    state.programFill.blocked = state.programFill.blocked.filter((entry) => entry.key !== request.key);
    return programFillStatusOf(state);
  },
};
