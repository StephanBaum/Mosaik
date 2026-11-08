//! Sharing and emergency access (M6b, `docs/DESKTOP_IPC.md` §3 "Sharing"):
//! contacts and safety codes, inviting, members and roles, the invites
//! inbox, and emergency access. Safety codes are public values and travel
//! as plain text; passwords and second-factor codes are [`SecretIn`]. No key
//! material of any kind appears in these types.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireAccountId, WireInviteId, WireVaultId};
use super::macros::wire_enum;
use super::secret::SecretIn;

wire_enum!(
    /// A role in a shared vault.
    pub enum ShareRole {
        /// Created the vault; cannot be removed.
        Owner,
        /// Invites, changes roles, removes members.
        Manage,
        /// Adds and changes items.
        Write,
        /// Reads only.
        Read,
    }
);

wire_enum!(
    /// What the contact book says about a looked-up key.
    pub enum ContactStatus {
        /// Never seen before.
        New,
        /// Pinned, safety code not compared yet.
        Pinned,
        /// Pinned and compared.
        Verified,
        /// The server serves another key than the pinned one.
        Changed,
        /// Devices pinned different keys.
        Conflicting,
    }
);

wire_enum!(
    /// How a member shows in the members list.
    pub enum MemberRowState {
        /// In the roster and on the server.
        Member,
        /// An invite that waits for the recipient.
        Invited,
        /// On the server but not in the roster.
        Unconfirmed,
        /// In the roster but not on the server.
        Dropped,
        /// A removal is queued.
        Removing,
    }
);

wire_enum!(
    /// How a membership came about.
    pub enum MemberOriginKind {
        /// The vault's owner.
        Owner,
        /// Accepted an invite.
        Invite,
        /// An emergency release.
        Emergency,
    }
);

wire_enum!(
    /// Where an emergency grant stands.
    pub enum EmergencyState {
        /// No request.
        Idle,
        /// Requested; the wait runs.
        Waiting,
        /// The grantor approved early.
        Approved,
        /// Released to the grantee.
        Released,
    }
);

wire_enum!(
    /// A grantor's answer to a request.
    pub enum EmergencyDecision {
        /// Refuse.
        Decline,
        /// Approve before the wait ends.
        Approve,
    }
);

/// The wait choices of an emergency grant, in days (CRYPTO_SPEC §8.4).
pub const WAIT_DAYS: [u8; 5] = [1, 2, 7, 14, 30];

/// The most vaults one emergency grant names.
pub const MAX_GRANT_VAULTS: usize = 64;

/// The `sharing_identity` response: this account's own safety code.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharingIdentity {
    /// The code (`null` until the first sync run made the identity key).
    pub safety_code: Option<String>,
    /// Whether the server holds the key.
    pub published: bool,
}

/// `sharing_lookup`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LookupRequest {
    /// The address to look up.
    pub email: String,
}

/// The `sharing_lookup` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LookupResult {
    /// The account the directory names.
    pub account_id: WireAccountId,
    /// The address looked up.
    pub email: String,
    /// The safety code to compare.
    pub safety_code: String,
    /// What the contact book says.
    pub status: ContactStatus,
}

/// `sharing_verify_contact` and `sharing_accept_key_change`: the code as shown.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContactCodeRequest {
    /// The contact.
    pub account_id: WireAccountId,
    /// The safety code the user compared.
    pub safety_code: String,
}

/// One contact.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactView {
    /// The account.
    pub account_id: WireAccountId,
    /// The address the contact was looked up by.
    pub email: String,
    /// The pinned safety code.
    pub safety_code: String,
    /// The user compared codes.
    pub verified: bool,
    /// Devices pinned different keys: settle it before sharing.
    pub changed: bool,
}

/// The `sharing_contacts` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactList {
    /// The contacts.
    pub contacts: Vec<ContactView>,
}

/// `vault_share`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShareVaultRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The verified contact.
    pub account_id: WireAccountId,
    /// The role to offer (never `owner`).
    pub role: ShareRole,
}

/// The `vault_share` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteSent {
    /// The invite.
    pub invite_id: WireInviteId,
    /// When it ends.
    pub expires_at: UnixMillis,
}

/// A request that names a vault and a member (`vault_member_remove`).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MemberRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The member.
    pub account_id: WireAccountId,
}

/// `vault_member_set_role`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetRoleRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The member.
    pub account_id: WireAccountId,
    /// The new role (never `owner`).
    pub role: ShareRole,
}

/// One row of the members list.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemberRow {
    /// The account.
    pub account_id: WireAccountId,
    /// The address (`null` if neither the server nor the roster has one).
    pub email: Option<String>,
    /// The role.
    pub role: ShareRole,
    /// The user compared safety codes with this member.
    pub verified: bool,
    /// This row is the signed-in account itself.
    #[serde(rename = "self")]
    pub is_self: bool,
    /// How the row shows.
    pub state: MemberRowState,
    /// How the membership came about (`null` for an invite or a dropped member).
    pub origin: Option<MemberOriginKind>,
}

/// The `vault_members` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultMembersView {
    /// This account's role.
    pub own_role: ShareRole,
    /// The rows.
    pub members: Vec<MemberRow>,
    /// A removal waits for the owner's device to change the vault key.
    pub pending_rotation: bool,
    /// Attachments the last key change could not protect (they sit on another device).
    pub unprotected_attachments: u32,
}

/// The `vault_member_remove` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveOutcome {
    /// Attachments the key change could not protect.
    pub unreachable_attachments: u32,
}

/// A request that names an invite (`vault_invite_cancel`, `invite_preview`).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteRequest {
    /// The invite.
    pub invite_id: WireInviteId,
}

/// An invite addressed to this account.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingInviteView {
    /// The invite.
    pub invite_id: WireInviteId,
    /// Who sent it.
    pub sender_email: String,
    /// The role offered.
    pub role: ShareRole,
    /// When it ends.
    pub expires_at: UnixMillis,
}

/// An invite this account sent and that still waits.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutgoingInviteView {
    /// The invite.
    pub invite_id: WireInviteId,
    /// The vault.
    pub vault_id: WireVaultId,
    /// The vault's name (`null` if unknown).
    pub vault_name: Option<String>,
    /// The invited address (`null` if the contact book has none).
    pub recipient_email: Option<String>,
    /// The role offered.
    pub role: ShareRole,
    /// When it ends.
    pub expires_at: UnixMillis,
}

/// The `invites_list` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InviteList {
    /// Invites for this account.
    pub incoming: Vec<IncomingInviteView>,
    /// Invites this account sent.
    pub outgoing: Vec<OutgoingInviteView>,
}

/// The `invite_preview` response: what the invite says, after the checks.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvitePreviewView {
    /// The vault's name (user content; `null` if unreadable).
    pub vault_name: Option<String>,
    /// The role offered.
    pub role: ShareRole,
    /// Who sent it.
    pub sender_account_id: WireAccountId,
    /// The sender's address.
    pub sender_email: String,
    /// The sender's safety code.
    pub safety_code: String,
    /// The state of the sender's pin.
    pub sender_status: ContactStatus,
    /// The vault's owner (the sender, or someone else when a manager invites).
    pub owner_account_id: WireAccountId,
    /// The owner's address.
    pub owner_email: String,
    /// The owner's safety code.
    pub owner_code: String,
    /// The state of the owner's pin: it must be verified before accepting.
    pub owner_status: ContactStatus,
}

/// `invite_accept`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteAcceptRequest {
    /// The invite.
    pub invite_id: WireInviteId,
    /// The sender's safety code the user compared (`null` when the sender is
    /// verified already). The owner of the vault is verified with
    /// `sharing_verify_contact` first.
    pub safety_code: Option<String>,
}

/// `invite_decline`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InviteDeclineRequest {
    /// The invite.
    pub invite_id: WireInviteId,
    /// Also refuse every later invite from this sender.
    pub block: bool,
}

/// What an emergency grant covers.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum EmergencyScopeView {
    /// Every vault the grantor owns, new ones included.
    All,
    /// These vaults.
    Vaults {
        /// The vaults.
        vault_ids: Vec<WireVaultId>,
    },
}

/// A grant this account gave.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantedView {
    /// The grantee.
    pub grantee_account_id: WireAccountId,
    /// The grantee's address.
    pub email: String,
    /// The wait in days.
    pub wait_days: u8,
    /// What it covers.
    pub scope: EmergencyScopeView,
    /// Where it stands.
    pub state: EmergencyState,
    /// When the grantee asked.
    pub requested_at: Option<UnixMillis>,
    /// When the wait ends (the server keeps the time).
    pub release_at: Option<UnixMillis>,
}

/// A grant this account received.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedView {
    /// The grantor.
    pub grantor_account_id: WireAccountId,
    /// The grantor's address.
    pub email: String,
    /// The wait in days.
    pub wait_days: u8,
    /// Where it stands.
    pub state: EmergencyState,
    /// When this account asked.
    pub requested_at: Option<UnixMillis>,
    /// When the wait ends.
    pub release_at: Option<UnixMillis>,
    /// When asking is allowed again after a refusal.
    pub cooldown_until: Option<UnixMillis>,
}

/// The `emergency_overview` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyOverview {
    /// People who can request access to this account's vaults.
    pub granted: Vec<GrantedView>,
    /// People who named this account.
    pub received: Vec<ReceivedView>,
}

/// `emergency_grant`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyGrantRequest {
    /// The verified contact.
    pub account_id: WireAccountId,
    /// What to cover.
    pub scope: EmergencyScopeView,
    /// The wait in days: 1, 2, 7, 14 or 30.
    pub wait_days: u8,
    /// The master password (a fresh login).
    pub password: SecretIn,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

/// `emergency_revoke`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyRevokeRequest {
    /// The grantee.
    pub account_id: WireAccountId,
}

/// `emergency_request`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyRequestRequest {
    /// The grantor.
    pub grantor_account_id: WireAccountId,
}

/// The `emergency_request` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyRequested {
    /// When the wait ends.
    pub release_at: UnixMillis,
}

/// `emergency_respond`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyRespondRequest {
    /// The grantee who asked.
    pub grantee_account_id: WireAccountId,
    /// Refuse or approve.
    pub decision: EmergencyDecision,
    /// The master password (approving needs a fresh login).
    pub password: Option<SecretIn>,
    /// A current second-factor code, when the sync account has one.
    pub code: Option<SecretIn>,
}

/// `emergency_claim`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmergencyClaimRequest {
    /// The grantor.
    pub grantor_account_id: WireAccountId,
}

/// The `emergency_claim` response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyClaimed {
    /// How many vaults arrived (read-only).
    pub vaults: u32,
}

/// What sharing wants the user to look at, with what to do in the UI text.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SharingFindingView {
    /// The server serves this account another identity key than its own.
    IdentityMismatch,
    /// A vault's member list is invalid.
    RosterInvalid {
        /// The vault.
        vault_name: Option<String>,
    },
    /// A new vault key was refused: its sender may not have been allowed to send it.
    ShareUntrusted {
        /// The vault.
        vault_name: Option<String>,
        /// The sender's address, if known.
        sender_email: Option<String>,
    },
    /// A member this account has not verified: no key is sealed to them and a
    /// key change locks them out.
    MemberUnverified {
        /// The vault.
        vault_id: WireVaultId,
        /// The vault's name.
        vault_name: Option<String>,
        /// The member.
        account_id: WireAccountId,
        /// The member's address, if known.
        email: Option<String>,
    },
    /// The server served a key for a member or invitee that does not match
    /// the verified one (possibly a man in the middle): they were removed or
    /// the invite cancelled.
    MemberKeyMismatch {
        /// The vault.
        vault_id: WireVaultId,
        /// The vault's name.
        vault_name: Option<String>,
        /// The member.
        account_id: WireAccountId,
        /// The member's address, if known.
        email: Option<String>,
    },
    /// Nothing here confirms who owns this vault: it is held until the
    /// account's other devices confirm it.
    VaultHeld {
        /// The vault.
        vault_id: WireVaultId,
        /// The vault's name.
        vault_name: Option<String>,
    },
    /// A member the server no longer lists.
    MemberDropped {
        /// The vault.
        vault_id: WireVaultId,
        /// The vault's name.
        vault_name: Option<String>,
        /// The member.
        account_id: WireAccountId,
        /// The member's address, if known.
        email: Option<String>,
    },
    /// The server refused this device a shared vault: its copy stays readable, nothing new arrives.
    AccessLost {
        /// The vault.
        vault_id: WireVaultId,
        /// The vault's name.
        vault_name: Option<String>,
    },
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    const T: UnixMillis = 1_761_480_000_000;
    const CODE: &str = "7KQ4X 29HDM 3PRT6 WZB04 NCFG8V";

    pub(crate) fn identities() -> Vec<SharingIdentity> {
        vec![
            SharingIdentity {
                safety_code: Some(CODE.into()),
                published: true,
            },
            SharingIdentity {
                safety_code: None,
                published: false,
            },
        ]
    }

    pub(crate) fn lookups() -> Vec<LookupResult> {
        ContactStatus::ALL
            .iter()
            .map(|&status| LookupResult {
                account_id: ids::account_n(2),
                email: "sam@example.com".into(),
                safety_code: CODE.into(),
                status,
            })
            .collect()
    }

    pub(crate) fn contacts() -> Vec<ContactList> {
        vec![
            ContactList {
                contacts: vec![
                    ContactView {
                        account_id: ids::account_n(2),
                        email: "sam@example.com".into(),
                        safety_code: CODE.into(),
                        verified: true,
                        changed: false,
                    },
                    ContactView {
                        account_id: ids::account_n(3),
                        email: "kim@example.com".into(),
                        safety_code: CODE.into(),
                        verified: false,
                        changed: true,
                    },
                ],
            },
            ContactList { contacts: vec![] },
        ]
    }

    pub(crate) fn invite_sent() -> Vec<InviteSent> {
        vec![InviteSent {
            invite_id: ids::invite(1),
            expires_at: T + 7 * 86_400_000,
        }]
    }

    pub(crate) fn members() -> Vec<VaultMembersView> {
        let row = |n: u8, role, state, origin, verified, email: Option<&str>| MemberRow {
            account_id: ids::account_n(n),
            email: email.map(Into::into),
            role,
            verified,
            is_self: n == 1,
            state,
            origin,
        };
        let mut rows = vec![
            row(
                1,
                ShareRole::Owner,
                MemberRowState::Member,
                Some(MemberOriginKind::Owner),
                true,
                Some("alex@example.com"),
            ),
            row(
                2,
                ShareRole::Write,
                MemberRowState::Member,
                Some(MemberOriginKind::Invite),
                true,
                Some("sam@example.com"),
            ),
            row(
                3,
                ShareRole::Read,
                MemberRowState::Invited,
                None,
                false,
                Some("kim@example.com"),
            ),
            row(
                4,
                ShareRole::Manage,
                MemberRowState::Unconfirmed,
                Some(MemberOriginKind::Emergency),
                false,
                None,
            ),
            row(
                5,
                ShareRole::Read,
                MemberRowState::Dropped,
                None,
                true,
                Some("lee@example.com"),
            ),
            row(
                6,
                ShareRole::Read,
                MemberRowState::Removing,
                Some(MemberOriginKind::Invite),
                true,
                Some("max@example.com"),
            ),
        ];
        let all = VaultMembersView {
            own_role: ShareRole::Owner,
            members: rows.clone(),
            pending_rotation: true,
            unprotected_attachments: 2,
        };
        rows.truncate(1);
        let alone = VaultMembersView {
            own_role: ShareRole::Manage,
            members: rows,
            pending_rotation: false,
            unprotected_attachments: 0,
        };
        vec![all, alone]
    }

    pub(crate) fn removed() -> Vec<RemoveOutcome> {
        vec![
            RemoveOutcome {
                unreachable_attachments: 0,
            },
            RemoveOutcome {
                unreachable_attachments: 3,
            },
        ]
    }

    pub(crate) fn invites() -> Vec<InviteList> {
        vec![
            InviteList {
                incoming: vec![IncomingInviteView {
                    invite_id: ids::invite(2),
                    sender_email: "sam@example.com".into(),
                    role: ShareRole::Write,
                    expires_at: T + 86_400_000,
                }],
                outgoing: vec![OutgoingInviteView {
                    invite_id: ids::invite(1),
                    vault_id: ids::vault(1),
                    vault_name: Some("Family".into()),
                    recipient_email: Some("kim@example.com".into()),
                    role: ShareRole::Read,
                    expires_at: T + 86_400_000,
                }],
            },
            InviteList {
                incoming: vec![],
                outgoing: vec![OutgoingInviteView {
                    invite_id: ids::invite(3),
                    vault_id: ids::vault(2),
                    vault_name: None,
                    recipient_email: None,
                    role: ShareRole::Manage,
                    expires_at: T,
                }],
            },
        ]
    }

    pub(crate) fn previews() -> Vec<InvitePreviewView> {
        vec![
            InvitePreviewView {
                vault_name: Some("Family".into()),
                role: ShareRole::Write,
                sender_account_id: ids::account_n(2),
                sender_email: "sam@example.com".into(),
                safety_code: CODE.into(),
                sender_status: ContactStatus::Pinned,
                owner_account_id: ids::account_n(2),
                owner_email: "sam@example.com".into(),
                owner_code: CODE.into(),
                owner_status: ContactStatus::Pinned,
            },
            InvitePreviewView {
                vault_name: None,
                role: ShareRole::Read,
                sender_account_id: ids::account_n(3),
                sender_email: "kim@example.com".into(),
                safety_code: CODE.into(),
                sender_status: ContactStatus::Verified,
                owner_account_id: ids::account_n(2),
                owner_email: "sam@example.com".into(),
                owner_code: "2B9MQ 7XK4D RT6W0 ZNCF8 V3HJ4P".into(),
                owner_status: ContactStatus::New,
            },
        ]
    }

    pub(crate) fn emergency() -> Vec<EmergencyOverview> {
        vec![
            EmergencyOverview {
                granted: vec![
                    GrantedView {
                        grantee_account_id: ids::account_n(2),
                        email: "sam@example.com".into(),
                        wait_days: 7,
                        scope: EmergencyScopeView::All,
                        state: EmergencyState::Waiting,
                        requested_at: Some(T),
                        release_at: Some(T + 7 * 86_400_000),
                    },
                    GrantedView {
                        grantee_account_id: ids::account_n(3),
                        email: "kim@example.com".into(),
                        wait_days: 30,
                        scope: EmergencyScopeView::Vaults {
                            vault_ids: vec![ids::vault(1), ids::vault(2)],
                        },
                        state: EmergencyState::Idle,
                        requested_at: None,
                        release_at: None,
                    },
                ],
                received: vec![
                    ReceivedView {
                        grantor_account_id: ids::account_n(4),
                        email: "lee@example.com".into(),
                        wait_days: 14,
                        state: EmergencyState::Released,
                        requested_at: Some(T),
                        release_at: Some(T),
                        cooldown_until: None,
                    },
                    ReceivedView {
                        grantor_account_id: ids::account_n(5),
                        email: "max@example.com".into(),
                        wait_days: 2,
                        state: EmergencyState::Idle,
                        requested_at: None,
                        release_at: None,
                        cooldown_until: Some(T + 86_400_000),
                    },
                ],
            },
            EmergencyOverview {
                granted: vec![],
                received: vec![],
            },
        ]
    }

    pub(crate) fn requested() -> Vec<EmergencyRequested> {
        vec![EmergencyRequested {
            release_at: T + 7 * 86_400_000,
        }]
    }

    pub(crate) fn claimed() -> Vec<EmergencyClaimed> {
        vec![EmergencyClaimed { vaults: 2 }]
    }

    pub(crate) fn findings() -> Vec<SharingFindingView> {
        let all = vec![
            SharingFindingView::IdentityMismatch,
            SharingFindingView::RosterInvalid {
                vault_name: Some("Family".into()),
            },
            SharingFindingView::ShareUntrusted {
                vault_name: None,
                sender_email: Some("sam@example.com".into()),
            },
            SharingFindingView::MemberUnverified {
                vault_id: ids::vault(1),
                vault_name: Some("Family".into()),
                account_id: ids::account_n(3),
                email: Some("kim@example.com".into()),
            },
            SharingFindingView::MemberKeyMismatch {
                vault_id: ids::vault(1),
                vault_name: Some("Family".into()),
                account_id: ids::account_n(4),
                email: Some("lee@example.com".into()),
            },
            SharingFindingView::VaultHeld {
                vault_id: ids::vault(2),
                vault_name: None,
            },
            SharingFindingView::MemberDropped {
                vault_id: ids::vault(1),
                vault_name: Some("Family".into()),
                account_id: ids::account_n(5),
                email: None,
            },
            SharingFindingView::AccessLost {
                vault_id: ids::vault(2),
                vault_name: Some("Work".into()),
            },
        ];
        for finding in &all {
            // Adding a variant fails to compile here until it has a sample.
            match finding {
                SharingFindingView::IdentityMismatch
                | SharingFindingView::RosterInvalid { .. }
                | SharingFindingView::ShareUntrusted { .. }
                | SharingFindingView::MemberUnverified { .. }
                | SharingFindingView::MemberKeyMismatch { .. }
                | SharingFindingView::VaultHeld { .. }
                | SharingFindingView::MemberDropped { .. }
                | SharingFindingView::AccessLost { .. } => {}
            }
        }
        all
    }
}
