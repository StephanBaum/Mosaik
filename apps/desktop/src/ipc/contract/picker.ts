/**
 * The picker windows' contract: mirrors `src-tauri/src/contract/picker.rs` (ADR-0164). The chip and
 * the list of Windows program fill call these commands; no secret is in any of the types.
 */
import type { ItemId, VaultId } from "./common.ts";
import type { Language, Theme } from "./settings.ts";

export const pickerModes = ["hidden", "chip", "list", "notice"] as const;
export type PickerMode = (typeof pickerModes)[number];

export const programKinds = ["package", "signed", "unsigned"] as const;
export type ProgramKind = (typeof programKinds)[number];

export const linkAgainReasons = ["signerChanged", "fileChanged"] as const;
export type LinkAgainReason = (typeof linkAgainReasons)[number];

export const pickerNotices = [
  "elevated",
  "noField",
  "aborted",
  "abortedCleared",
  "modifiersHeld",
  "notResponding",
] as const;
export type PickerNotice = (typeof pickerNotices)[number];

/** A program, for display. */
export interface ProgramView {
  kind: ProgramKind;
  exe: string;
  name: string;
  signer: string | null;
  /** The file's path, for an unsigned program. */
  path: string | null;
}

/** A login, for the list: no password, no one-time code. */
export interface PickerLogin {
  vaultId: VaultId;
  itemId: ItemId;
  title: string;
  username: string | null;
  vaultName: string | null;
  host: string | null;
}

/** 16 lowercase hex digits of each file hash. */
export interface HashChange {
  oldPrefix: string;
  newPrefix: string;
}

export interface LinkStep {
  login: PickerLogin;
  program: ProgramView;
  /** 1 or 2. */
  confirmations: number;
  confirmed: number;
  lookalikeOf: string | null;
  hashChange: HashChange | null;
  /** Whether Enter is pressed after the fill. */
  submit: boolean;
}

export interface PickerState {
  /** Rises with every change; a request with another one is `pickerStale`. */
  generation: number;
  mode: PickerMode;
  /** The vault is locked: the chip says "unlock". */
  locked: boolean;
  language: Language;
  theme: Theme;
  program: ProgramView | null;
  matches: PickerLogin[];
  /** The index in `matches` to preselect (exactly one match). */
  preselected: number | null;
  suggestions: PickerLogin[];
  linkAgain: LinkAgainReason | null;
  link: LinkStep | null;
  notice: PickerNotice | null;
}

export interface PickerGenerationRequest {
  generation: number;
}

export interface PickerSearchRequest {
  generation: number;
  /** At most 128 characters. */
  query: string;
}

export interface PickerItemRequest {
  generation: number;
  vaultId: VaultId;
  itemId: ItemId;
}

export type LinkConfirmResult = { kind: "needAnother" } | { kind: "tooSoon" } | { kind: "linked" };

/** What a picker window is told on its channel. */
export interface PickerChangedEvent {
  type: "changed";
  generation: number;
}
export type PickerEvent = PickerChangedEvent;
