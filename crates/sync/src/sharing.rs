//! The sharing and emergency-access calls of the hosted backend (M6b spec
//! §6.4, CRYPTO_SPEC §8): [`SharingTransport`], one method per route of
//! `docs/SERVER.md`, blocking, with the types they exchange.
//!
//! Only the hosted transport implements it ([`crate::Transport::sharing`]);
//! the folder has none (CRYPTO_SPEC §8.6). Every method returns
//! [`TransportError`], whose variants carry no IDs or bodies. Nothing here
//! opens a seal or a ciphertext: the engine hands them to the session.

use mosaic_core::model::Role;
use mosaic_core::{
    AccountId, ContactsCiphertext, IdentityPublicKey, InstallId, KeyGeneration, ShareSeal, VaultId,
    VaultMetaCiphertext, VaultRosterCiphertext, WrappedKey,
};
use mosaic_vault::session::{IncomingInvite, InviteId};

use crate::transport::TransportError;

/// An account's identity key as the directory serves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectoryEntry {
    /// The account.
    pub account_id: AccountId,
    /// Its identity public key as the server serves it (the caller pins and
    /// checks it; the server can lie, CRYPTO_SPEC §8.2).
    pub ipk: IdentityPublicKey,
}

/// What `PUT /v1/account/identity` did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentityPut {
    /// Stored, or the same pair was stored already.
    Stored,
    /// Another pair is stored: the server's.
    Exists {
        /// The stored public key.
        ipk: IdentityPublicKey,
        /// The stored wrapped private key.
        wrapped: WrappedKey,
    },
}

/// What a compare-and-swap upload did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CasResult {
    /// Stored under this sequence number.
    Stored {
        /// The new sequence number.
        seq: u64,
    },
    /// The base was not the stored one.
    Conflict {
        /// The sequence the server holds.
        current_seq: u64,
    },
}

/// An incoming invite with its expiry.
#[derive(Clone, Debug)]
pub struct ListedInvite {
    /// The invite as the session reads it.
    pub invite: IncomingInvite,
    /// When it ends (ms since the Unix epoch).
    pub expires_at: u64,
}

/// What became of an invite the account sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InviteState {
    /// Waiting for the recipient.
    Pending,
    /// Accepted.
    Accepted,
    /// Declined by the recipient.
    Declined,
    /// Cancelled by the sender or a manager, or by a rotation.
    Cancelled,
    /// Ran out.
    Expired,
}

/// An invite the account sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutgoingInvite {
    /// The invite.
    pub invite_id: InviteId,
    /// The vault.
    pub vault_id: VaultId,
    /// The invited account.
    pub recipient: AccountId,
    /// The role offered.
    pub role: Role,
    /// Its state.
    pub state: InviteState,
    /// The recipient fetched it (the seal left the server).
    pub delivered: bool,
    /// When it ends (ms since the Unix epoch).
    pub expires_at: u64,
}

/// How a membership came about.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberOrigin {
    /// The vault's owner.
    Owner,
    /// Accepted an invite.
    Invite,
    /// An emergency release.
    Emergency,
}

/// A member as the server lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberInfo {
    /// The account.
    pub account_id: AccountId,
    /// Its email.
    pub email: String,
    /// Its role on the server.
    pub role: Role,
    /// How it became a member.
    pub origin: MemberOrigin,
    /// When (ms since the Unix epoch).
    pub added_at: u64,
}

/// An open invite as the server lists it for a vault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingInvite {
    /// The invite.
    pub invite_id: InviteId,
    /// The invited account.
    pub recipient: AccountId,
    /// The role offered.
    pub role: Role,
    /// The recipient fetched it.
    pub delivered: bool,
}

/// `GET /v1/vaults/{v}/members`.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct MembersView {
    /// The members.
    pub members: Vec<MemberInfo>,
    /// The open invites.
    pub invites: Vec<PendingInvite>,
}

impl MembersView {
    /// The member with the role `owner`, if the server names one.
    #[must_use]
    pub fn owner(&self) -> Option<AccountId> {
        self.members
            .iter()
            .find(|m| m.role == Role::Owner)
            .map(|m| m.account_id)
    }
}

/// What `POST /v1/invites/{id}/accept` answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AcceptResult {
    /// The vault.
    pub vault_id: VaultId,
    /// The generation of the key the invite carried.
    pub generation: KeyGeneration,
    /// The role the server gave.
    pub role: Role,
}

/// The body of `POST /v1/vaults/{v}/rotate`.
#[derive(Debug)]
pub struct RotateRequest<'a> {
    /// The new generation.
    pub generation: KeyGeneration,
    /// The new key wrapped under the account key.
    pub wrapped: &'a WrappedKey,
    /// The install that makes the request.
    pub device: InstallId,
    /// The vault's head this rotation is based on.
    pub expected_head: u64,
    /// The roster at the new generation.
    pub roster: &'a VaultRosterCiphertext,
    /// A share for every remaining member.
    pub seals: Vec<(AccountId, ShareSeal)>,
    /// A refreshed seal for every open invite.
    pub invite_seals: Vec<(InviteId, ShareSeal)>,
    /// The members to lock out.
    pub revoke: Vec<AccountId>,
}

/// What a rotation request did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RotateResult {
    /// Stored.
    Accepted {
        /// The vault's new head.
        seq: u64,
    },
    /// The generation exists already.
    GenerationTaken {
        /// The vault's generation now.
        current: KeyGeneration,
    },
    /// The head moved: pull and prepare again.
    HeadMoved {
        /// The vault's head now.
        head: u64,
    },
    /// The request did not cover exactly the members and open invites.
    MembersMismatch {
        /// The members without the caller, as the server counts them.
        members: Vec<AccountId>,
        /// The open invites.
        invites: Vec<InviteId>,
    },
}

/// A new invite (`POST /v1/vaults/{v}/invites`).
#[derive(Debug)]
pub struct NewInvite<'a> {
    /// The invite ID.
    pub invite_id: InviteId,
    /// The invited account.
    pub recipient: AccountId,
    /// The role offered.
    pub role: Role,
    /// The generation of the sealed key.
    pub generation: KeyGeneration,
    /// The vault key sealed to the recipient.
    pub seal: &'a ShareSeal,
}

/// Who an emergency grant covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantScope {
    /// Every vault the grantor owns, new ones included.
    All,
    /// The listed vaults.
    Selected,
}

/// Where an emergency grant stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrantState {
    /// No request.
    Idle,
    /// Requested; the wait runs.
    Waiting,
    /// The grantor approved.
    Approved,
    /// Released to the grantee.
    Released,
}

/// A grant the account gave.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrantInfo {
    /// The grantee.
    pub grantee: AccountId,
    /// The grantee's email.
    pub email: String,
    /// The waiting days.
    pub wait_days: u8,
    /// The scope.
    pub scope: GrantScope,
    /// The vaults with seals and the generation of each seal.
    pub vaults: Vec<(VaultId, KeyGeneration)>,
    /// The state.
    pub state: GrantState,
    /// When the grantee asked (ms since the Unix epoch).
    pub requested_at: Option<u64>,
    /// When the wait ends (ms since the Unix epoch).
    pub release_at: Option<u64>,
}

/// A grant the account received.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceivedGrant {
    /// The grantor.
    pub grantor: AccountId,
    /// The grantor's email.
    pub email: String,
    /// The waiting days.
    pub wait_days: u8,
    /// The state.
    pub state: GrantState,
    /// When the account asked (ms since the Unix epoch).
    pub requested_at: Option<u64>,
    /// When the wait ends (ms since the Unix epoch).
    pub release_at: Option<u64>,
    /// When asking is allowed again after a refusal.
    pub cooldown_until: Option<u64>,
}

/// The sharing and emergency-access calls of the hosted backend. See the
/// [module documentation](self).
pub trait SharingTransport {
    /// `GET /v1/account?identity=1`: the identity pair the server holds.
    ///
    /// # Errors
    /// [`TransportError`].
    fn account_identity(
        &mut self,
    ) -> Result<Option<(IdentityPublicKey, WrappedKey)>, TransportError>;

    /// `PUT /v1/account/identity`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_identity(
        &mut self,
        ipk: &IdentityPublicKey,
        wrapped: &WrappedKey,
    ) -> Result<IdentityPut, TransportError>;

    /// `POST /v1/directory/lookup`: the key the directory serves for an
    /// email address (a fake one for an unknown address).
    ///
    /// # Errors
    /// [`TransportError`].
    fn lookup(&mut self, email: &str) -> Result<DirectoryEntry, TransportError>;

    /// `GET /v1/directory/{id}`: `None` if the account shares nothing with
    /// the caller.
    ///
    /// # Errors
    /// [`TransportError`].
    fn directory(
        &mut self,
        account: &AccountId,
    ) -> Result<Option<IdentityPublicKey>, TransportError>;

    /// `GET /v1/account/contacts`: the encrypted contact book and its
    /// sequence number (0 if there is none).
    ///
    /// # Errors
    /// [`TransportError`].
    fn get_contacts(&mut self) -> Result<(Option<ContactsCiphertext>, u64), TransportError>;

    /// `PUT /v1/account/contacts`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_contacts(
        &mut self,
        ciphertext: &ContactsCiphertext,
        base_seq: u64,
    ) -> Result<CasResult, TransportError>;

    /// `POST /v1/vaults/{v}/invites`: the invite's expiry.
    ///
    /// # Errors
    /// [`TransportError`] (`Rejected(InviteExists | AlreadyMember |
    /// ReinviteBlocked | InviteLimit | RoleForbidden | ...)`).
    fn create_invite(
        &mut self,
        vault: VaultId,
        invite: &NewInvite<'_>,
    ) -> Result<u64, TransportError>;

    /// `GET /v1/invites`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn list_invites(&mut self) -> Result<(Vec<ListedInvite>, Vec<OutgoingInvite>), TransportError>;

    /// `GET /v1/invites/{id}/vault`: the vault's metadata and current
    /// roster, for the recipient of an open invite.
    ///
    /// # Errors
    /// [`TransportError`].
    fn invite_vault(
        &mut self,
        invite: &InviteId,
    ) -> Result<(VaultMetaCiphertext, Option<VaultRosterCiphertext>), TransportError>;

    /// `POST /v1/invites/{id}/accept`.
    ///
    /// # Errors
    /// [`TransportError`] (`Rejected(InviteStale | InviteExpired)`).
    fn accept_invite(
        &mut self,
        invite: &InviteId,
        wrapped: &WrappedKey,
        generation: KeyGeneration,
        device: InstallId,
    ) -> Result<AcceptResult, TransportError>;

    /// `POST /v1/invites/{id}/decline`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn decline_invite(&mut self, invite: &InviteId, block: bool) -> Result<(), TransportError>;

    /// `DELETE /v1/invites/{id}`: cancels an open invite, or removes a
    /// finished one from the sender's list.
    ///
    /// # Errors
    /// [`TransportError`].
    fn cancel_invite(&mut self, invite: &InviteId) -> Result<(), TransportError>;

    /// `GET /v1/vaults/{v}/members`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn members(&mut self, vault: VaultId) -> Result<MembersView, TransportError>;

    /// `PUT /v1/vaults/{v}/members/{a}`: the server's role of a member.
    ///
    /// # Errors
    /// [`TransportError`].
    fn set_role(
        &mut self,
        vault: VaultId,
        account: &AccountId,
        role: Role,
    ) -> Result<(), TransportError>;

    /// `DELETE /v1/vaults/{v}/members/{a}`: an owner or manager removes a
    /// member whose role is at most their own (never the owner). The
    /// owner's next run sees the roster member missing from the member list
    /// and rotates the vault key.
    ///
    /// # Errors
    /// [`TransportError`] (`Forbidden`, `NotFound`, `Rejected(RoleForbidden)`).
    fn remove_member(&mut self, vault: VaultId, account: &AccountId) -> Result<(), TransportError>;

    /// `PUT /v1/vaults/{v}/roster`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_roster(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        roster: &VaultRosterCiphertext,
        base_seq: Option<u64>,
    ) -> Result<CasResult, TransportError>;

    /// `POST /v1/vaults/{v}/rotate`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn rotate(
        &mut self,
        vault: VaultId,
        request: &RotateRequest<'_>,
    ) -> Result<RotateResult, TransportError>;

    /// `PUT /v1/vaults/{v}/keys/{g}/claim`: the vault's new head.
    ///
    /// # Errors
    /// [`TransportError`] (`Rejected(NoShare)`).
    fn claim_key(
        &mut self,
        vault: VaultId,
        generation: KeyGeneration,
        wrapped: &WrappedKey,
        device: InstallId,
    ) -> Result<u64, TransportError>;

    /// `POST /v1/vaults/{v}/leave`.
    ///
    /// # Errors
    /// [`TransportError`] (`Rejected(OwnerCannotLeave)`).
    fn leave(&mut self, vault: VaultId) -> Result<(), TransportError>;

    /// `POST /v1/blocks/{id}`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn block(&mut self, account: &AccountId) -> Result<(), TransportError>;

    /// `DELETE /v1/blocks/{id}`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn unblock(&mut self, account: &AccountId) -> Result<(), TransportError>;

    /// `GET /v1/emergency`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn emergency(&mut self) -> Result<(Vec<GrantInfo>, Vec<ReceivedGrant>), TransportError>;

    /// `PUT /v1/emergency/grants/{grantee}` (needs a fresh login).
    ///
    /// # Errors
    /// [`TransportError`] (`FreshLoginRequired`, `Rejected(EmergencyWaiting |
    /// EmergencyReleased | EmergencyOwnerOnly)`).
    fn put_grant(
        &mut self,
        grantee: &AccountId,
        wait_days: u8,
        scope: GrantScope,
        seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError>;

    /// `PUT /v1/emergency/grants/{grantee}/seals`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn put_grant_seals(
        &mut self,
        grantee: &AccountId,
        seals: &[(VaultId, KeyGeneration, ShareSeal)],
    ) -> Result<(), TransportError>;

    /// `DELETE /v1/emergency/grants/{grantee}`.
    ///
    /// # Errors
    /// [`TransportError`].
    fn delete_grant(&mut self, grantee: &AccountId) -> Result<(), TransportError>;

    /// `POST /v1/emergency/grants/{grantor}/request`: when the wait ends.
    ///
    /// # Errors
    /// [`TransportError`] (`Rejected(EmergencyCooldown)`).
    fn emergency_request(&mut self, grantor: &AccountId) -> Result<u64, TransportError>;

    /// `POST /v1/emergency/grants/{grantee}/decline` or `/approve` (approve
    /// needs a fresh login).
    ///
    /// # Errors
    /// [`TransportError`].
    fn emergency_decide(
        &mut self,
        grantee: &AccountId,
        approve: bool,
    ) -> Result<(), TransportError>;

    /// `POST /v1/emergency/grants/{grantor}/release`: the seals.
    ///
    /// # Errors
    /// [`TransportError`] (`Rejected(EmergencyWaiting)`).
    fn emergency_release(
        &mut self,
        grantor: &AccountId,
    ) -> Result<Vec<(VaultId, KeyGeneration, ShareSeal)>, TransportError>;
}
