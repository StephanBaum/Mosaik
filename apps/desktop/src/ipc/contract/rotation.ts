/** Account keys contract: mirrors `src-tauri/src/contract/rotation.rs` (docs/DESKTOP_IPC.md §3 "Account keys"). */
import type { AccountId, VaultId } from "./common.ts";

/** What a rotation job renews. */
export const rotationKinds = ["secretKey", "rekey", "profile", "vault"] as const;
export type RotationKind = (typeof rotationKinds)[number];

/** Where a rotation job stands (CRYPTO_SPEC §6.6, phases 1 to 7). */
export const rotationPhaseNames = [
  "prepared",
  "backendStarted",
  "backendAccepted",
  "folderWritten",
  "localCommitted",
  "attachments",
  "uploading",
] as const;
export type RotationPhaseName = (typeof rotationPhaseNames)[number];

/** What the user must type to continue a job. */
export const rotationNeeds = ["none", "password", "kitGroup"] as const;
export type RotationNeed = (typeof rotationNeeds)[number];

/** The profiles `profile_upgrade` offers, weakest first. */
export const rotationProfiles = ["standard", "strong", "max"] as const;
export type RotationProfile = (typeof rotationProfiles)[number];

export const rekeyBackends = ["local", "hosted", "folder"] as const;
export type RekeyBackend = (typeof rekeyBackends)[number];

export interface RotationJobView {
  kind: RotationKind;
  phase: RotationPhaseName;
  done: number;
  total: number;
  needs: RotationNeed;
}

/** The `rotation_status` response. */
export interface RotationStatus {
  job: RotationJobView | null;
}

/** An attachment the re-seal could not read: it keeps its old key. */
export interface UnreachableAttachmentView {
  /** `null` if the item has no title or is gone. */
  itemTitle: string | null;
  /** `null` if the reference is gone. */
  fileName: string | null;
}

/** A re-key notice in the sync folder that this device could not confirm. */
export interface RekeyMarkerView {
  name: string;
  /** 64 hex digits. */
  sha256: string;
  toEpoch: number;
}

/** What a finished (or paused) rotation did. */
export interface RotationDone {
  quickUnlockRemoved: boolean;
  hardwareKeysRemoved: number;
  /** The total; `unreachable` lists at most the first 50. */
  unreachableAttachments: number;
  unreachable: UnreachableAttachmentView[];
  /** `false`: the job continues from "Continue key rotation". */
  finished: boolean;
}

export interface RotatePrepareRequest {
  password: string;
}

export interface RotateCommitRequest {
  password: string;
  code: string | null;
}

export interface ProfileUpgradeRequest {
  password: string;
  profile: RotationProfile;
  code: string | null;
  /** Go on although this device needs more than two seconds per unlock. */
  force: boolean;
}

export interface SharedOwnedView {
  vaultId: VaultId;
  name: string | null;
  members: number;
}

export interface MemberVaultView {
  vaultId: VaultId;
  name: string | null;
}

/** The `rekey_review` response: what a re-key touches. */
export interface RekeyReviewView {
  ownedVaults: number;
  sharedOwned: SharedOwnedView[];
  memberVaults: MemberVaultView[];
  hardwareVaults: number;
  grants: number;
  hardwareKeys: number;
  quickUnlock: boolean;
  unreachableAttachments: number;
  backend: RekeyBackend;
  accountId: AccountId;
}

export interface RekeyPrepareRequest {
  password: string;
  newPassword: string;
}

export interface RekeyCommitRequest {
  password: string;
  newPassword: string;
  code: string | null;
}

export interface RotationResumeRequest {
  /** The password the account has after the job; `null` if `needs` is `none`. */
  password: string | null;
  /** The last group of the new Secret Key, from the new kit (`needs` `kitGroup`). */
  kitGroup: string | null;
  code: string | null;
}

export interface DismissFileRequest {
  name: string;
  sha256: string;
}
