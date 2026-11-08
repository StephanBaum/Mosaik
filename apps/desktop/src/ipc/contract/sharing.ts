/**
 * Sharing contract: mirrors `src-tauri/src/contract/sharing.rs` (docs/DESKTOP_IPC.md §3
 * "Sharing"). Safety codes are public values; passwords and codes are plain strings here and
 * travel as `SecretIn` on the Rust side. No key material appears in any of these types.
 */
import type { AccountId, UnixMillis, VaultId } from "./common.ts";

/** A sharing invite ID (a canonical UUID). */
export type InviteId = string;

/** A role in a shared vault. */
export const shareRoles = ["owner", "manage", "write", "read"] as const;
export type ShareRole = (typeof shareRoles)[number];

/** What the contact book says about a looked-up key. */
export const contactStatuses = ["new", "pinned", "verified", "changed", "conflicting"] as const;
export type ContactStatus = (typeof contactStatuses)[number];

/** How a member shows in the members list. */
export const memberRowStates = [
  "member",
  "invited",
  "unconfirmed",
  "dropped",
  "removing",
] as const;
export type MemberRowState = (typeof memberRowStates)[number];

/** How a membership came about. */
export const memberOriginKinds = ["owner", "invite", "emergency"] as const;
export type MemberOriginKind = (typeof memberOriginKinds)[number];

/** Where an emergency grant stands. */
export const emergencyStates = ["idle", "waiting", "approved", "released"] as const;
export type EmergencyState = (typeof emergencyStates)[number];

/** A grantor's answer to a request. */
export const emergencyDecisions = ["decline", "approve"] as const;
export type EmergencyDecision = (typeof emergencyDecisions)[number];

/** The wait choices of an emergency grant, in days (seven by default). */
export const waitDays = [1, 2, 7, 14, 30] as const;
export const defaultWaitDays = 7;

/** The `sharing_identity` response: this account's own safety code. */
export interface SharingIdentity {
  /** `null` until the first sync run made the identity key. */
  safetyCode: string | null;
  published: boolean;
}

export interface LookupRequest {
  email: string;
}

export interface LookupResult {
  accountId: AccountId;
  email: string;
  safetyCode: string;
  status: ContactStatus;
}

/** `sharing_verify_contact` and `sharing_accept_key_change`: the code as shown. */
export interface ContactCodeRequest {
  accountId: AccountId;
  safetyCode: string;
}

export interface ContactView {
  accountId: AccountId;
  email: string;
  safetyCode: string;
  verified: boolean;
  /** Devices pinned different keys: settle it before sharing. */
  changed: boolean;
}

export interface ContactList {
  contacts: ContactView[];
}

export interface ShareVaultRequest {
  vaultId: VaultId;
  accountId: AccountId;
  /** Never `owner`. */
  role: ShareRole;
}

export interface InviteSent {
  inviteId: InviteId;
  expiresAt: UnixMillis;
}

/** A request naming a vault and a member (`vault_member_remove`). */
export interface MemberRequest {
  vaultId: VaultId;
  accountId: AccountId;
}

export interface SetRoleRequest {
  vaultId: VaultId;
  accountId: AccountId;
  /** Never `owner`. */
  role: ShareRole;
}

export interface MemberRow {
  accountId: AccountId;
  email: string | null;
  role: ShareRole;
  verified: boolean;
  state: MemberRowState;
  origin: MemberOriginKind | null;
}

export interface VaultMembersView {
  ownRole: ShareRole;
  members: MemberRow[];
  /** A removal waits for the owner's device to change the vault key. */
  pendingRotation: boolean;
  /** Attachments the last key change could not protect (they sit on another device). */
  unprotectedAttachments: number;
}

export interface RemoveOutcome {
  unreachableAttachments: number;
}

/** A request naming an invite (`vault_invite_cancel`, `invite_preview`). */
export interface InviteRequest {
  inviteId: InviteId;
}

export interface IncomingInviteView {
  inviteId: InviteId;
  senderEmail: string;
  role: ShareRole;
  expiresAt: UnixMillis;
}

export interface OutgoingInviteView {
  inviteId: InviteId;
  vaultId: VaultId;
  vaultName: string | null;
  recipientEmail: string | null;
  role: ShareRole;
  expiresAt: UnixMillis;
}

export interface InviteList {
  incoming: IncomingInviteView[];
  outgoing: OutgoingInviteView[];
}

export interface InvitePreviewView {
  /** User content; `null` if unreadable. */
  vaultName: string | null;
  role: ShareRole;
  senderAccountId: AccountId;
  senderEmail: string;
  safetyCode: string;
  senderStatus: ContactStatus;
  ownerAccountId: AccountId;
  ownerEmail: string;
  ownerCode: string;
  /** The owner must be verified before accepting. */
  ownerStatus: ContactStatus;
}

export interface InviteAcceptRequest {
  inviteId: InviteId;
  /** The sender's code the user compared; `null` when the sender is verified already. */
  safetyCode: string | null;
}

export interface InviteDeclineRequest {
  inviteId: InviteId;
  /** Also refuse every later invite from this sender. */
  block: boolean;
}

export type EmergencyScopeView = { kind: "all" } | { kind: "vaults"; vaultIds: VaultId[] };

export interface GrantedView {
  granteeAccountId: AccountId;
  email: string;
  waitDays: number;
  scope: EmergencyScopeView;
  state: EmergencyState;
  requestedAt: UnixMillis | null;
  /** The server keeps this time. */
  releaseAt: UnixMillis | null;
}

export interface ReceivedView {
  grantorAccountId: AccountId;
  email: string;
  waitDays: number;
  state: EmergencyState;
  requestedAt: UnixMillis | null;
  releaseAt: UnixMillis | null;
  cooldownUntil: UnixMillis | null;
}

export interface EmergencyOverview {
  granted: GrantedView[];
  received: ReceivedView[];
}

export interface EmergencyGrantRequest {
  accountId: AccountId;
  scope: EmergencyScopeView;
  waitDays: number;
  /** A fresh login. */
  password: string;
  code: string | null;
}

export interface EmergencyRevokeRequest {
  accountId: AccountId;
}

export interface EmergencyRequestRequest {
  grantorAccountId: AccountId;
}

export interface EmergencyRequested {
  releaseAt: UnixMillis;
}

export interface EmergencyRespondRequest {
  granteeAccountId: AccountId;
  decision: EmergencyDecision;
  /** Approving needs the master password. */
  password: string | null;
  code: string | null;
}

export interface EmergencyClaimRequest {
  grantorAccountId: AccountId;
}

export interface EmergencyClaimed {
  vaults: number;
}

/** What sharing wants the user to look at; the UI text says what to do. */
export type SharingFindingView =
  | { kind: "identityMismatch" }
  | { kind: "rosterInvalid"; vaultName: string | null }
  | { kind: "shareUntrusted"; vaultName: string | null; senderEmail: string | null }
  | {
      kind: "memberUnverified";
      vaultId: VaultId;
      vaultName: string | null;
      accountId: AccountId;
      email: string | null;
    }
  | {
      kind: "memberDropped";
      vaultId: VaultId;
      vaultName: string | null;
      accountId: AccountId;
      email: string | null;
    }
  | { kind: "accessLost"; vaultId: VaultId; vaultName: string | null };

export type SharingFindingKind = SharingFindingView["kind"];
