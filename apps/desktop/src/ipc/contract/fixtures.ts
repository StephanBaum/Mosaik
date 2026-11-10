/**
 * The shape of `../contract.fixtures.ts`, which Rust generates from its contract types
 * (ADR-0018). `satisfies ContractFixtures` there makes `tsc` reject any sample whose
 * fields, variants or command names differ from these TypeScript types.
 */
import type { BinaryCommandName, CommandName, RequestOf, ResponseOf } from "./commands.ts";
import type { IpcError } from "./common.ts";
import type { AppEvent } from "./events.ts";

/** String enums whose value lists the parity test compares in both directions. */
export interface EnumFixtures {
  biometricPlatformKind: string[];
  errorCode: string[];
  quickUnlockKind: string[];
  itemKind: string[];
  textFieldKey: string[];
  secretFieldKey: string[];
  trashFilter: string[];
  urlMatchMode: string[];
  customFieldKind: string[];
  otpScheme: string[];
  otpScanSource: string[];
  otpAlgorithm: string[];
  otpSecretStrength: string[];
  strength: string[];
  theme: string[];
  language: string[];
  insightKind: string[];
  insightSeverity: string[];
  insightAction: string[];
  breachMode: string[];
  queueKind: string[];
  queueReason: string[];
  attemptStage: string[];
  rulesSourceKey: string[];
  changeUrlSource: string[];
  rotationOutcomeKey: string[];
  historyReasonKey: string[];
  breachScanError: string[];
  importFormat: string[];
  importWarning: string[];
  importEntryWarning: string[];
  importSkipReason: string[];
  shortcutState: string[];
  sshSkipReason: string[];
  sshSignFailReason: string[];
  sshDecision: string[];
  sshCloseReason: string[];
  browserEndpointState: string[];
  browserRegistryState: string[];
  browserFamily: string[];
  browserHint: string[];
  pairingDecision: string[];
  browserCloseReason: string[];
  browserSaveKind: string[];
  browserPasskeyKind: string[];
  browserRotationKind: string[];
  passkeyVerification: string[];
  passkeyAlgorithm: string[];
  lockReason: string[];
  syncStatus: string[];
  vaultProtection: string[];
  rotationKind: string[];
  rotationPhaseName: string[];
  rotationNeeds: string[];
  rotationProfile: string[];
  rekeyBackend: string[];
  registrationMode: string[];
  hostedMode: string[];
  handoffState: string[];
  handoffStep: string[];
  folderState: string[];
  devicePlatform: string[];
  shareRole: string[];
  contactStatus: string[];
  memberRowState: string[];
  memberOriginKind: string[];
  emergencyState: string[];
  emergencyDecision: string[];
}

export interface ContractFixtures {
  /** Every command Rust registers, including channel commands. */
  commands: string[];
  enums: EnumFixtures;
  errors: IpcError[];
  events: AppEvent[];
  requests: { [K in CommandName]: RequestOf<K>[] };
  /** JSON samples of every command except those with a binary response. */
  responses: { [K in Exclude<CommandName, BinaryCommandName>]: ResponseOf<K>[] };
  /** The commands without a JSON response sample (`attachment_preview`). */
  binaryResponses: BinaryCommandName[];
}
