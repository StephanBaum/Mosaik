//! What a user does with sharing (M6b spec §6.4, §7): look an account up,
//! invite it, answer an invite, change a role, remove a member, leave, and
//! hand out, request or claim emergency access. Each call is one or a few
//! server calls plus the session steps around them, in the two-phase style
//! of the rest of the engine: the session prepares, the server decides, the
//! session commits.
//!
//! The desktop calls these between sync runs (one at a time, with the same
//! [`SessionAccess`] as the run). Verifying a contact
//! ([`UnlockedSession::verify_contact`](mosaic_vault::session::UnlockedSession::verify_contact))
//! and accepting a key change are session steps without a server call.

use std::collections::BTreeSet;

use mosaic_core::model::{ContactState, GrantCover, Role};
use mosaic_core::{AccountId, IdentityPublicKey, VaultId};
use mosaic_vault::session::{
    Error, InviteId, InvitePreview, RevocationReason, SharingFinding, UnlockedSession,
};

use super::pull::RunStats;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport};
use crate::sharing::{
    GrantInfo, GrantScope, ListedInvite, MemberOrigin, NewInvite, OutgoingInvite, ReceivedGrant,
};
use crate::transport::{Rejection, Transport, TransportError};

/// What a directory lookup found, pinned for the user to verify.
#[derive(Clone, Debug)]
pub struct ContactLookup {
    /// The account the directory names for the address.
    pub account_id: AccountId,
    /// The address looked up.
    pub email: String,
    /// The identity key the server serves (unchecked until the user compares
    /// the safety code).
    pub ipk: IdentityPublicKey,
    /// The safety code to compare, `XXXXX XXXXX XXXXX XXXXX XXXXXX`.
    pub safety_code: String,
    /// What the contact book says about this key.
    pub state: ContactState,
}

/// A sent invite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InviteSentInfo {
    /// The invite.
    pub invite_id: InviteId,
    /// When it ends (ms since the Unix epoch).
    pub expires_at: u64,
}

/// How a member shows in the members panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberState {
    /// In the roster and on the server.
    Member,
    /// An invite that waits for the recipient.
    Invited,
    /// On the server but not in the roster (an emergency release, an account
    /// the server added): the next rotation locks it out.
    Unconfirmed,
    /// In the roster but not on the server.
    Dropped,
    /// A revocation is queued for it.
    Removing,
}

/// One row of the members panel.
#[derive(Clone, Debug)]
pub struct MemberView {
    /// The account.
    pub account_id: AccountId,
    /// Its email (the server's, or the roster's for a dropped member).
    pub email: String,
    /// Its role.
    pub role: Role,
    /// How the server says it became a member.
    pub origin: Option<MemberOrigin>,
    /// How it shows.
    pub state: MemberState,
    /// The user compared safety codes with it.
    pub verified: bool,
}

/// The members of a vault as this device sees them.
#[derive(Clone, Debug)]
pub struct VaultMembers {
    /// This account's role.
    pub own_role: Role,
    /// The rows.
    pub members: Vec<MemberView>,
    /// A revocation waits for the owner's device.
    pub pending_rotation: bool,
}

/// The more restrictive of two roles (a higher number).
fn stricter(a: Role, b: Role) -> Role {
    match (a.number(), b.number()) {
        (Some(x), Some(y)) if y > x => b,
        _ => a,
    }
}

impl<T: Transport> SyncEngine<T> {
    /// Uploads the contact book and requires that nothing is left pending:
    /// the ownership entries and grant covers must be on the server before
    /// a seal or a grant goes out. `sync_contacts` can end with an upload
    /// still pending after compare-and-swap conflicts; that is a retryable
    /// error here, nothing is sealed.
    pub(super) fn publish_contacts(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<(), SyncError> {
        self.sync_contacts(access, &mut SyncReport::default())?;
        if access
            .with(UnlockedSession::pending_contacts_upload)??
            .is_some()
        {
            return Err(SyncError::Transport(TransportError::Rejected(
                Rejection::Conflict { current_seq: None },
            )));
        }
        Ok(())
    }

    /// Looks an email address up in the server's directory and pins the key
    /// it serves (trust on first use, unverified). The user compares the
    /// safety code with the person before any seal is made.
    ///
    /// # Errors
    /// [`SyncError`]; [`SyncError::SharingUnavailable`] on a folder.
    pub fn lookup_contact(
        &mut self,
        access: &mut impl SessionAccess,
        email: &str,
    ) -> Result<ContactLookup, SyncError> {
        let entry = self.sharing_tx()?.lookup(email)?;
        let state = access.with(|s| s.pin_contact(&entry.account_id, &entry.ipk, email))??;
        let safety_code = mosaic_core::Fingerprint::of(&entry.account_id, &entry.ipk).safety_code();
        Ok(ContactLookup {
            account_id: entry.account_id,
            email: email.to_owned(),
            ipk: entry.ipk,
            safety_code,
            state,
        })
    }

    /// Moves the cursor of `vault` back to just before `seq`, so the next
    /// pull serves the roster stored under it again.
    pub(super) fn rewind_cursor(
        access: &mut impl SessionAccess,
        vault: VaultId,
        seq: u64,
    ) -> Result<(), SyncError> {
        let back = seq.saturating_sub(1);
        access.with(|s| -> Result<(), Error> {
            let now = s
                .sync_cursor(vault)?
                .and_then(|c| <[u8; 8]>::try_from(c.as_slice()).ok())
                .map_or(0, u64::from_be_bytes);
            if now > back {
                s.set_sync_cursor(vault, &back.to_be_bytes())?;
            }
            Ok(())
        })??;
        Ok(())
    }

    /// Invites `account` (looked up as `email`, and verified by the user) to
    /// the vault with `role` (CRYPTO_SPEC §8.2): the roster with the
    /// invitee goes up first, then the invite with the vault key sealed to
    /// the invitee's key.
    ///
    /// # Errors
    /// [`SyncError::Session`] with `ContactUnverified`, `ContactKeyChanged`,
    /// `RoleForbidden`, `AlreadyMember` or `ReinviteBlocked`;
    /// [`TransportError::Rejected`] with `InviteExists`, `InviteLimit` and
    /// the like.
    pub fn invite_member(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        account: &AccountId,
        email: &str,
        role: Role,
    ) -> Result<InviteSentInfo, SyncError> {
        // The key as the server serves it now: the session compares it with
        // the verified pin.
        let entry = self.sharing_tx()?.lookup(email)?;
        if entry.account_id != *account {
            return Err(SyncError::Session(Error::ContactKeyChanged));
        }
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        // A vault of this account that is about to be shared is in the
        // account's set of owned vaults, and the set is on the server,
        // before the roster goes up (M6b verify H3).
        if access.with(|s| s.ensure_vault_owned(vault))?? {
            self.publish_contacts(access)?;
        }
        for _ in 0..3 {
            let prepared =
                access.with(|s| s.prepare_invite(vault, account, &entry.ipk, email, role))??;
            let base = access.with(|s| s.roster_seq(vault))??;
            match self.sharing_tx()?.put_roster(
                vault,
                prepared.generation(),
                prepared.roster(),
                base,
            )? {
                crate::sharing::CasResult::Stored { seq } => {
                    let expires_at = self.sharing_tx()?.create_invite(
                        vault,
                        &NewInvite {
                            invite_id: prepared.invite_id(),
                            recipient: *account,
                            role,
                            generation: prepared.generation(),
                            seal: prepared.seal(),
                        },
                    )?;
                    let invite_id = prepared.invite_id();
                    let roster = prepared.roster().clone();
                    access.with(|s| -> Result<(), Error> {
                        s.commit_invite_sent(prepared, seq)?;
                        super::served::note_roster(s, vault, seq, roster.as_bytes())
                    })??;
                    return Ok(InviteSentInfo {
                        invite_id,
                        expires_at,
                    });
                }
                crate::sharing::CasResult::Conflict { current_seq } => {
                    // Another manager changed the roster: read it, merge,
                    // prepare again.
                    Self::rewind_cursor(access, vault, current_seq)?;
                    self.pull_vault(access, vault, &mut report, &mut stats)?;
                }
            }
        }
        Err(SyncError::Transport(TransportError::Rejected(
            Rejection::Conflict { current_seq: None },
        )))
    }

    /// The invites addressed to this account and the ones it sent.
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn list_invites(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<(Vec<ListedInvite>, Vec<OutgoingInvite>), SyncError> {
        let _ = access;
        Ok(self.sharing_tx()?.list_invites()?)
    }

    /// Previews an incoming invite (CRYPTO_SPEC §8.2): the sender's key from
    /// the directory, the vault's metadata and roster, and the checks of
    /// [`UnlockedSession::open_invite`](mosaic_vault::session::UnlockedSession::open_invite).
    /// The opened key waits in the session for the user's accept.
    ///
    /// # Errors
    /// [`SyncError::Session`] with `InviteUnreadable`, `InviteNotInRoster` or
    /// `ContactKeyChanged`; [`TransportError::NotFound`] for an invite the
    /// server does not list.
    pub fn preview_invite(
        &mut self,
        access: &mut impl SessionAccess,
        invite: &InviteId,
    ) -> Result<InvitePreview, SyncError> {
        let (incoming, _) = self.sharing_tx()?.list_invites()?;
        let listed = incoming
            .into_iter()
            .find(|l| l.invite.invite_id == *invite)
            .ok_or(SyncError::Transport(TransportError::NotFound))?;
        let ipk = self
            .sharing_tx()?
            .directory(&listed.invite.sender)?
            .ok_or(SyncError::Transport(TransportError::NotFound))?;
        let (meta, roster) = self.sharing_tx()?.invite_vault(invite)?;
        let roster = roster.ok_or(SyncError::Session(Error::InviteNotInRoster))?;
        Ok(access.with(|s| s.open_invite(&listed.invite, &ipk, &meta, &roster))??)
    }

    /// Accepts a previewed invite (the user's gesture; the sender's pin must
    /// be verified): the key goes to the server wrapped under the account
    /// key, then the vault is stored here and the next run pulls it.
    ///
    /// # Errors
    /// [`SyncError::Session`] with `InviteNotOpened`, `ContactUnverified` or
    /// `OwnerMismatch`; [`TransportError::Rejected`] with `InviteStale` or
    /// `InviteExpired`.
    pub fn accept_invite(
        &mut self,
        access: &mut impl SessionAccess,
        invite: &InviteId,
    ) -> Result<VaultId, SyncError> {
        let install = access.with(|s| s.install_id())?;
        let accepted = access.with(|s| s.accept_invite(invite))??;
        // The invite carries the generation of the key that was previewed:
        // a rotation since then replaced the seal (CRYPTO_SPEC §8.3), and the
        // key in the session is the old one. Ask again.
        let (incoming, _) = self.sharing_tx()?.list_invites()?;
        let listed = incoming
            .iter()
            .find(|l| l.invite.invite_id == *invite)
            .ok_or(SyncError::Transport(TransportError::NotFound))?;
        if listed.invite.generation != accepted.generation() {
            return Err(SyncError::Transport(TransportError::Rejected(
                Rejection::InviteStale {
                    current_generation: listed.invite.generation.get(),
                },
            )));
        }
        let result = self.sharing_tx()?.accept_invite(
            invite,
            accepted.wrapped(),
            accepted.generation(),
            install,
        )?;
        if result.vault_id != accepted.vault_id() || result.generation != accepted.generation() {
            return Err(SyncError::Transport(TransportError::Protocol));
        }
        let owner = self
            .sharing_tx()?
            .members(result.vault_id)?
            .owner()
            .ok_or(SyncError::Transport(TransportError::Protocol))?;
        // The role the invite carried and the one the server gave: the
        // stricter holds.
        let role = stricter(accepted.role(), result.role);
        let vault = accepted.vault_id();
        access.with(|s| s.commit_invite(accepted, role, &owner))??;
        Ok(vault)
    }

    /// Declines an invite, optionally blocking the sender.
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn decline_invite(
        &mut self,
        access: &mut impl SessionAccess,
        invite: &InviteId,
        block: bool,
    ) -> Result<(), SyncError> {
        let _ = access;
        Ok(self.sharing_tx()?.decline_invite(invite, block)?)
    }

    /// Cancels an invite this account sent. If the seal was already
    /// delivered the recipient may hold the key: a revocation is queued
    /// (CRYPTO_SPEC §8.3).
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn cancel_invite(
        &mut self,
        access: &mut impl SessionAccess,
        invite: &InviteId,
    ) -> Result<(), SyncError> {
        let (_, outgoing) = self.sharing_tx()?.list_invites()?;
        let sent = outgoing.into_iter().find(|o| o.invite_id == *invite);
        if let Some(o) = sent
            && o.delivered
        {
            let queued = access.with(|s| {
                s.request_revocation(o.vault_id, &o.recipient, RevocationReason::InviteCancelled)
            })?;
            // Not a vault of this device, or not one this account manages:
            // nothing to queue.
            if let Err(e @ Error::Store(_)) = queued {
                return Err(e.into());
            }
        }
        Ok(self.sharing_tx()?.cancel_invite(invite)?)
    }

    /// The members panel of a vault: the server's list against the roster.
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn vault_members(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<VaultMembers, SyncError> {
        let view = self.sharing_tx()?.members(vault)?;
        let (own_role, roster, pending, contacts) = access.with(|s| -> Result<_, Error> {
            Ok((
                s.vault_own_role(vault)?,
                s.vault_roster(vault)?.unwrap_or_default(),
                s.pending_revocations()?,
                s.contacts()?,
            ))
        })??;
        let verified = |a: &AccountId| {
            contacts
                .iter()
                .any(|c| c.account_id == *a && c.verified && !c.conflicting)
        };
        let removing: BTreeSet<AccountId> = pending
            .iter()
            .filter(|(v, _, _)| *v == vault)
            .map(|(_, a, _)| *a)
            .collect();
        let active: BTreeSet<AccountId> = roster
            .iter()
            .filter(|m| m.role != Role::Removed)
            .map(|m| m.account_id)
            .collect();
        let mut rows = Vec::new();
        for m in &view.members {
            let state = if removing.contains(&m.account_id) {
                MemberState::Removing
            } else if active.contains(&m.account_id) {
                MemberState::Member
            } else {
                MemberState::Unconfirmed
            };
            let role = roster
                .iter()
                .find(|r| r.account_id == m.account_id)
                .map_or(m.role, |r| r.role);
            rows.push(MemberView {
                account_id: m.account_id,
                email: m.email.clone(),
                role,
                origin: Some(m.origin),
                state,
                verified: verified(&m.account_id),
            });
        }
        for i in &view.invites {
            let email = roster
                .iter()
                .find(|r| r.account_id == i.recipient)
                .map(|r| r.email.clone())
                .unwrap_or_default();
            rows.push(MemberView {
                account_id: i.recipient,
                email,
                role: i.role,
                origin: None,
                state: MemberState::Invited,
                verified: verified(&i.recipient),
            });
        }
        for r in roster.iter().filter(|r| r.role != Role::Removed) {
            let known = view.members.iter().any(|m| m.account_id == r.account_id)
                || view.invites.iter().any(|i| i.recipient == r.account_id);
            if !known {
                rows.push(MemberView {
                    account_id: r.account_id,
                    email: r.email.clone(),
                    role: r.role,
                    origin: None,
                    state: MemberState::Dropped,
                    verified: verified(&r.account_id),
                });
            }
        }
        Ok(VaultMembers {
            own_role,
            members: rows,
            pending_rotation: !removing.is_empty(),
        })
    }

    /// Changes a member's role: the roster edit (uploaded by the next run)
    /// and the server's role. Never to or from owner.
    ///
    /// # Errors
    /// [`SyncError::Session`] with `RoleForbidden`, `NoRoster`,
    /// `NotInRoster`; [`TransportError::Rejected`] with `RoleForbidden`.
    pub fn set_member_role(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        account: &AccountId,
        role: Role,
    ) -> Result<(), SyncError> {
        access.with(|s| s.set_member_role(vault, account, role))??;
        Ok(self.sharing_tx()?.set_role(vault, account, role)?)
    }

    /// Removes a member: the revocation is queued and, on the owner's
    /// device, carried out at once by rotating the vault key (only the owner
    /// rotates; a manager's request waits for the owner, CRYPTO_SPEC §8.3).
    /// Returns the attachments the rotation could not re-seal.
    ///
    /// # Errors
    /// [`SyncError::Session`] with `RoleForbidden` or `OwnerCannotBeRevoked`.
    pub fn remove_member(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        account: &AccountId,
    ) -> Result<Vec<mosaic_core::AttachmentId>, SyncError> {
        let (is_owner, pending) = access.with(|s| -> Result<_, Error> {
            let own = s.vault_own_role(vault)?;
            if !own.can_manage() || *account == s.account_id() {
                return Err(Error::RoleForbidden);
            }
            if *account == s.vault_owner(vault)? {
                return Err(Error::OwnerCannotBeRevoked);
            }
            let is_owner = own == Role::Owner && s.owns_vault(vault)?;
            if is_owner {
                s.request_revocation(vault, account, RevocationReason::Removed)?;
            }
            Ok((is_owner, s.pending_revocations()?))
        })??;
        if !is_owner {
            // A manager: the server drops the membership (the target's role
            // must be at most the manager's); the owner's next run sees the
            // roster member missing from the member list and rotates (H4).
            self.sharing_tx()?.remove_member(vault, account)?;
            return Ok(Vec::new());
        }
        let explicit: BTreeSet<AccountId> = pending
            .iter()
            .filter(|(v, _, _)| *v == vault)
            .map(|(_, a, _)| *a)
            .collect();
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        self.rotate_shared(access, vault, &explicit, &mut report, &mut stats)?;
        Ok(report
            .unreachable_attachments
            .into_iter()
            .map(|(_, a)| a)
            .collect())
    }

    /// Leaves a shared vault: the server drops the membership and this
    /// device stops syncing the vault (its copy stays readable). The owner
    /// cannot leave.
    ///
    /// # Errors
    /// [`TransportError::Rejected`] with `OwnerCannotLeave`.
    pub fn leave_vault(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<(), SyncError> {
        self.sharing_tx()?.leave(vault)?;
        access.with(|s| s.mark_access_lost(vault))??;
        Ok(())
    }

    // ---- emergency access ---------------------------------------------------

    /// The grants this account gave and received.
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn emergency_overview(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<(Vec<GrantInfo>, Vec<ReceivedGrant>), SyncError> {
        let _ = access;
        Ok(self.sharing_tx()?.emergency()?)
    }

    /// Grants emergency access (CRYPTO_SPEC §8.4): one seal of the current
    /// key per vault in scope, to a verified contact. `vaults` is used for
    /// [`GrantScope::Selected`]; [`GrantScope::All`] covers every vault this
    /// account owns. Needs a fresh login.
    ///
    /// # Errors
    /// [`SyncError::Session`] with `ContactUnverified`, `ContactKeyChanged`
    /// or `NotOwner`; [`TransportError::FreshLoginRequired`] and
    /// [`TransportError::Rejected`] with `EmergencyWaiting`.
    pub fn grant_emergency(
        &mut self,
        access: &mut impl SessionAccess,
        grantee: &AccountId,
        email: &str,
        scope: GrantScope,
        wait_days: u8,
        vaults: &[VaultId],
    ) -> Result<(), SyncError> {
        let entry = self.sharing_tx()?.lookup(email)?;
        if entry.account_id != *grantee {
            return Err(SyncError::Session(Error::ContactKeyChanged));
        }
        let chosen: Vec<VaultId> = match scope {
            GrantScope::All => Self::owned_vaults(access)?.into_keys().collect(),
            GrantScope::Selected => vaults.to_vec(),
        };
        // The covered vaults are shared vaults from now on (H3): the owner
        // roster, the sharing row and the entry in the account's set of owned
        // vaults come BEFORE the seals and the grant, so no crash leaves a
        // grant on a vault another device still takes for unshared. The
        // grant's cover is recorded the same way (H3b): deleting the grant
        // rotates exactly that, whatever the server reports.
        for vault in &chosen {
            Self::ensure_owner_roster(access, *vault)?;
        }
        let cover = match scope {
            GrantScope::All => GrantCover::All,
            GrantScope::Selected => GrantCover::Vaults(chosen.clone()),
        };
        access.with(|s| s.record_grant_cover(grantee, &cover))??;
        self.publish_contacts(access)?;
        let seals = access.with(|s| s.seal_emergency(grantee, &entry.ipk, &chosen))??;
        self.sharing_tx()?
            .put_grant(grantee, wait_days, scope, &seals)?;
        Ok(())
    }

    /// Deletes a grant (any state). If it was released, a revocation is
    /// queued for every vault of this account the grantee became a member
    /// of through the release.
    ///
    /// # Errors
    /// [`SyncError`].
    pub fn revoke_emergency(
        &mut self,
        access: &mut impl SessionAccess,
        grantee: &AccountId,
    ) -> Result<(), SyncError> {
        // Every vault the grant covered is rotated, whatever the server says
        // about the release state, the membership, the scope or the vault
        // list: the cover was recorded in the contact book before the seals
        // were made (M6b verify H3b). A grant this account has no record of
        // is taken as covering every vault it positively owns.
        let cover = access.with(|s| s.grant_cover(grantee))??;
        let covered: Vec<VaultId> = {
            let owned = Self::owned_vaults(access)?;
            match cover {
                Some(GrantCover::Vaults(vaults)) => vaults
                    .into_iter()
                    .filter(|v| owned.contains_key(v))
                    .collect(),
                Some(GrantCover::All) | None => owned.into_keys().collect(),
            }
        };
        for vault in &covered {
            Self::ensure_owner_roster(access, *vault)?;
            access.with(|s| s.request_revocation(*vault, grantee, RevocationReason::Removed))??;
        }
        self.sharing_tx()?.delete_grant(grantee)?;
        // The key the grantee may hold is replaced at once.
        let mut report = SyncReport::default();
        let mut stats = RunStats::default();
        for vault in covered {
            let explicit: BTreeSet<AccountId> = BTreeSet::from([*grantee]);
            self.rotate_shared(access, vault, &explicit, &mut report, &mut stats)?;
        }
        Ok(())
    }

    /// Asks a grantor for emergency access: when the wait ends (ms since the
    /// Unix epoch).
    ///
    /// # Errors
    /// [`TransportError::Rejected`] with `EmergencyCooldown`.
    pub fn request_emergency(
        &mut self,
        access: &mut impl SessionAccess,
        grantor: &AccountId,
    ) -> Result<u64, SyncError> {
        let _ = access;
        Ok(self.sharing_tx()?.emergency_request(grantor)?)
    }

    /// Answers a request: refuse, or approve at once (approving needs a
    /// fresh login).
    ///
    /// # Errors
    /// [`TransportError::FreshLoginRequired`] and the other
    /// [`TransportError`]s.
    pub fn respond_emergency(
        &mut self,
        access: &mut impl SessionAccess,
        grantee: &AccountId,
        approve: bool,
    ) -> Result<(), SyncError> {
        let _ = access;
        Ok(self.sharing_tx()?.emergency_decide(grantee, approve)?)
    }

    /// Claims an emergency release (CRYPTO_SPEC §8.4): the server hands over
    /// the seals once the wait is over, the session opens them with the
    /// grantor's key, and each opened key is claimed on the server and
    /// stored as a read-only vault. Returns how many vaults.
    ///
    /// # Errors
    /// [`TransportError::Rejected`] with `EmergencyWaiting`;
    /// [`SyncError::Session`] with `ContactKeyChanged` or `ShareUntrusted`.
    pub fn claim_emergency(
        &mut self,
        access: &mut impl SessionAccess,
        grantor: &AccountId,
    ) -> Result<usize, SyncError> {
        let install = access.with(|s| s.install_id())?;
        let seals = self.sharing_tx()?.emergency_release(grantor)?;
        let ipk = self
            .sharing_tx()?
            .directory(grantor)?
            .ok_or(SyncError::Transport(TransportError::NotFound))?;
        let accepted = access.with(|s| s.open_emergency_release(grantor, &ipk, &seals))??;
        let mut claimed = 0;
        for a in accepted {
            self.sharing_tx()?
                .claim_key(a.vault_id(), a.generation(), a.wrapped(), install)?;
            access.with(|s| s.commit_invite(a, Role::Read, grantor))??;
            claimed += 1;
        }
        Ok(claimed)
    }

    /// The findings about sharing, for the settings screen (the user
    /// acknowledges them with the session's `clear_sharing_findings`).
    ///
    /// # Errors
    /// [`SyncError::Locked`], [`SyncError::Session`].
    pub fn sharing_findings(
        &self,
        access: &mut impl SessionAccess,
    ) -> Result<Vec<SharingFinding>, SyncError> {
        Ok(access.with(|s| s.sharing_findings())??)
    }
}
