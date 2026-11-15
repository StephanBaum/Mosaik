/** Program fill contract: mirrors `src-tauri/src/contract/program_fill.rs` (ADR-0164). */

/** Why program fill is not available on this device. */
export const programFillUnavailable = ["notWindows", "mosaicElevated", "hostFailed"] as const;
export type ProgramFillUnavailable = (typeof programFillUnavailable)[number];

/** A program the user chose not to be offered logins for. */
export interface BlockedProgram {
  /** Opaque; goes back unchanged in `program_fill_unblock`. */
  key: string;
  label: string;
}

export interface ProgramFillStatus {
  available: boolean;
  reason: ProgramFillUnavailable | null;
  /** The watcher runs only while unlocked and switched on. */
  running: boolean;
  blocked: BlockedProgram[];
}

export interface ProgramFillUnblockRequest {
  key: string;
}
