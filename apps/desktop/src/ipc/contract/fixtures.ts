/**
 * The shape of `../contract.fixtures.ts`, which Rust generates from its contract types
 * (ADR-0018). `satisfies ContractFixtures` there makes `tsc` reject any sample whose
 * fields, variants or command names differ from these TypeScript types.
 */
import type { CommandName, RequestOf, ResponseOf } from "./commands.ts";
import type { IpcError } from "./common.ts";
import type { AppEvent } from "./events.ts";

/** String enums whose value lists the parity test compares in both directions. */
export interface EnumFixtures {
  errorCode: string[];
  quickUnlockKind: string[];
  itemKind: string[];
  textFieldKey: string[];
  secretFieldKey: string[];
  trashFilter: string[];
  urlMatchMode: string[];
  customFieldKind: string[];
  otpScheme: string[];
  strength: string[];
  theme: string[];
  language: string[];
  insightKind: string[];
  importFormat: string[];
  importWarning: string[];
  sshDecision: string[];
  sshCloseReason: string[];
  lockReason: string[];
  syncStatus: string[];
}

export interface ContractFixtures {
  /** Every command Rust registers, including channel commands. */
  commands: string[];
  enums: EnumFixtures;
  errors: IpcError[];
  events: AppEvent[];
  requests: { [K in CommandName]: RequestOf<K>[] };
  responses: { [K in CommandName]: ResponseOf<K>[] };
}
