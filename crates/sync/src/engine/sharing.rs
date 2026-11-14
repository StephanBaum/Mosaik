//! Sharing in the sync run (M6b spec §6.4, CRYPTO_SPEC §8): the identity
//! key, the contact book, the key shares and rosters of a pulled vault (the
//! claim rule), invites, role changes, revocation rotations by the vault
//! owner and the emergency seals.
//!
//! Order inside [`SyncEngine::sync_once`](super::SyncEngine::sync_once):
//! identity and contact book before the pull (the pull needs the identity key
//! and the pins), the per-vault sharing step at the end of every vault pull,
//! and the account-level steps (invites, queued roster uploads, revocations,
//! emergency seals) between the pull and the push.
//!
//! Only the vault owner rotates a shared vault (ADR-0050): a revocation that
//! a manager asked for waits for the owner's device. A rotation is offered to
//! the server as one atomic request and committed locally only when the
//! server accepted it; a shared vault never adopts a key (CRYPTO_SPEC §10.2).

use std::collections::{BTreeMap, BTreeSet};

use mosaic_core::model::{ContactState, Role, RosterMember};
use mosaic_core::{AccountId, Fingerprint, IdentityPublicKey, KeyGeneration, ShareSeal, VaultId};
use mosaic_vault::session::{
    Error, InviteId, PulledShareView, RevocationReason, RotationPlan, SharingFinding,
    UnlockedSession,
};
use mosaic_vault::storage::VaultFlag;

use super::consts::ATTACHMENT_DOWNLOAD_BUDGET;
use super::pull::RunStats;
use super::{SessionAccess, SyncEngine, SyncError, SyncReport, SyncWarning};
use crate::sharing::{
    CasResult, GrantInfo, GrantScope, GrantState, IdentityPut, InviteState, MembersView,
    OutgoingInvite, RotateRequest, RotateResult, SharingTransport,
};
use crate::transport::{
    PullPage, PulledRoster, PulledShare, Rejection, Transport, TransportError, TransportKind,
};

/// Attempts at one revocation rotation per run (a head that moves, a member
/// list that changes under it).
const ROTATION_ATTEMPTS: usize = 3;
/// Attempts at one compare-and-swap upload per run.
const CAS_ATTEMPTS: usize = 3;

/// What the pages of one vault pull held for the sharing step.
#[derive(Default)]
pub(super) struct Collected {
    pub(super) shares: Vec<PulledShare>,
    pub(super) rosters: Vec<PulledRoster>,
    pub(super) role: Option<Role>,
    /// The owner's page said a member that re-keyed asked for a rotation (M7).
    pub(super) rotation_requested: bool,
}

impl Collected {
    pub(super) fn absorb(&mut self, page: &PullPage) {
        for share in &page.shares {
            if !self.shares.contains(share) {
                self.shares.push(share.clone());
            }
        }
        for roster in &page.rosters {
            if !self.rosters.contains(roster) {
                self.rosters.push(roster.clone());
            }
        }
        if page.role.is_some() {
            self.role = page.role;
        }
        self.rotation_requested |= page.rotation_requested;
    }
}

/// The server sequence a cursor holds (empty is 0).
fn cursor_head(cursor: Option<&[u8]>) -> u64 {
    cursor
        .and_then(|c| <[u8; 8]>::try_from(c).ok())
        .map_or(0, u64::from_be_bytes)
}

fn is_lost(e: &TransportError) -> bool {
    matches!(e, TransportError::Forbidden | TransportError::NotFound)
}

/// A refusal of an upload that the next run makes again is not an error.
fn tolerate(result: Result<(), TransportError>) -> Result<(), SyncError> {
    match result {
        Ok(())
        | Err(TransportError::Rejected(_) | TransportError::Forbidden | TransportError::NotFound) => {
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// The server accepted `ct` as the roster of `vault` under `seq`.
fn roster_uploaded(
    s: &mut UnlockedSession,
    vault: VaultId,
    ct: &mosaic_core::VaultRosterCiphertext,
    seq: u64,
) -> Result<(), Error> {
    s.mark_roster_uploaded(vault, ct, seq)?;
    super::served::note_roster(s, vault, seq, ct.as_bytes())
}

impl<T: Transport> SyncEngine<T> {
    /// The sharing calls of the backend.
    pub(super) fn sharing_tx(&mut self) -> Result<&mut dyn SharingTransport, SyncError> {
        self.transport
            .sharing()
            .ok_or(SyncError::SharingUnavailable)
    }

    fn sharing_offered(&mut self) -> bool {
        self.config.transport_kind == TransportKind::Hosted && self.transport.sharing().is_some()
    }

    /// Whether a finding stops sharing until the user acknowledges it (the
    /// server serves this account another identity key).
    fn sharing_blocked(access: &mut impl SessionAccess) -> Result<bool, SyncError> {
        Ok(access
            .with(|s| s.sharing_findings())??
            .contains(&SharingFinding::IdentityMismatch))
    }

    /// The server answered 403 or 404 for a shared vault. A member loses the
    /// vault (it is no longer served; CRYPTO_SPEC §8.3). The owner never is:
    /// an honest server cannot refuse the owner, who cannot be removed or
    /// leave, so a refusal is the server lying. It is recorded as a finding
    /// against the server (persisted until acknowledged) and the vault stays
    /// in the normal retry path, so a revocation is never dropped (SYNC-1,
    /// R154). One closure: the ownership check and the write cannot race.
    ///
    /// Returns whether the vault is one this account owns.
    pub(super) fn refused_vault(
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<bool, SyncError> {
        Ok(access.with(|s| -> Result<bool, Error> {
            if !s.vault_shared(vault)? {
                return Ok(false);
            }
            if s.owns_vault(vault)? {
                s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
                Ok(true)
            } else {
                s.mark_access_lost(vault)?;
                Ok(false)
            }
        })??)
    }

    /// The rotation of an owned vault cannot go on because the server
    /// refused a call (403, 404 or `role_forbidden`, none of which an honest
    /// server answers the owner). Never silent: a finding against the server
    /// and [`SyncWarning::RotationBlocked`]; the revocations stay queued and
    /// the next run tries again. A vault this account does not own just
    /// returns (only the owner's device rotates).
    fn rotation_refused(
        access: &mut impl SessionAccess,
        vault: VaultId,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let owned = access.with(|s| -> Result<bool, Error> {
            if !s.vault_shared(vault)? || !s.owns_vault(vault)? {
                return Ok(false);
            }
            s.set_vault_flag(vault, VaultFlag::ServerInconsistent)?;
            Ok(true)
        })??;
        if owned {
            Self::warn_rotation_blocked(report, vault);
        }
        Ok(())
    }

    fn warn_rotation_blocked(report: &mut SyncReport, vault: VaultId) {
        let warning = SyncWarning::RotationBlocked { vault };
        if !report.warnings.contains(&warning) {
            report.warnings.push(warning);
        }
    }

    /// Records a finding unless the same one is stored.
    pub(super) fn note_finding(
        access: &mut impl SessionAccess,
        finding: SharingFinding,
    ) -> Result<(), SyncError> {
        access.with(|s| -> Result<(), Error> {
            if !s.sharing_findings()?.contains(&finding) {
                s.record_sharing_finding(finding)?;
            }
            Ok(())
        })??;
        Ok(())
    }

    /// Whether the account-level sharing steps run: a hosted backend, an
    /// identity key and no finding that blocks sharing.
    fn sharing_ready(&mut self, access: &mut impl SessionAccess) -> Result<bool, SyncError> {
        if !self.sharing_offered() {
            return Ok(false);
        }
        if access.with(|s| s.identity())??.is_none() {
            return Ok(false);
        }
        Ok(!Self::sharing_blocked(access)?)
    }

    // ---- identity and contact book (before the pull) ---------------------

    /// Makes the identity key if there is none, publishes it or adopts the
    /// one another device published (CRYPTO_SPEC §8.1), and syncs the contact
    /// book. A key the server serves for this account that is not the local
    /// one, after publication, is the finding `IdentityMismatch`.
    pub(super) fn sharing_identity_phase(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        // An opened invite key does not outlive its five minutes (L4).
        access.with(UnlockedSession::sweep_pending_invites)?;
        if !self.sharing_offered() {
            return Ok(());
        }
        match access.with(UnlockedSession::ensure_identity_key)? {
            Ok(_) => {}
            Err(Error::IdentityMismatch) => {
                return Self::note_finding(access, SharingFinding::IdentityMismatch);
            }
            Err(e) => return Err(e.into()),
        }
        let Some((ipk, wrapped, published)) = access.with(|s| s.identity())?? else {
            return Ok(());
        };
        match self.sharing_tx()?.account_identity()? {
            Some((served, _)) if served == ipk => {
                if !published {
                    access.with(UnlockedSession::mark_identity_published)??;
                }
            }
            Some((served, served_wrapped)) => {
                Self::take_served_identity(access, published, &served, &served_wrapped)?;
            }
            None => match self.sharing_tx()?.put_identity(&ipk, &wrapped)? {
                IdentityPut::Stored => access.with(UnlockedSession::mark_identity_published)??,
                IdentityPut::Exists {
                    ipk: served,
                    wrapped: served_wrapped,
                } => {
                    if served == ipk {
                        access.with(UnlockedSession::mark_identity_published)??;
                    } else {
                        Self::take_served_identity(access, published, &served, &served_wrapped)?;
                    }
                }
            },
        }
        if Self::sharing_blocked(access)? {
            return Ok(());
        }
        // Vaults this device created before the contact book held ownership
        // (positive authorization, M6b verify round 3).
        access.with(UnlockedSession::record_local_ownership)??;
        self.sync_contacts(access, report)
    }

    /// The server holds another identity pair than this device's: adopt it
    /// if this device's own was never published and the served one opens
    /// under the account key to a matching `isk`; otherwise it is a finding.
    fn take_served_identity(
        access: &mut impl SessionAccess,
        published: bool,
        served: &IdentityPublicKey,
        wrapped: &mosaic_core::WrappedKey,
    ) -> Result<(), SyncError> {
        if published {
            return Self::note_finding(access, SharingFinding::IdentityMismatch);
        }
        match access.with(|s| s.adopt_remote_identity(wrapped, served))? {
            Ok(()) => Ok(()),
            Err(Error::IdentityMismatch) => {
                Self::note_finding(access, SharingFinding::IdentityMismatch)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Merges the server's contact book into the local one and uploads a
    /// local change (compare-and-swap, merge on conflict).
    pub(super) fn sync_contacts(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        self.pull_contacts(access, report)?;
        for _ in 0..CAS_ATTEMPTS {
            let Some((ct, base)) = access.with(UnlockedSession::pending_contacts_upload)?? else {
                break;
            };
            match self.sharing_tx()?.put_contacts(&ct, base.unwrap_or(0))? {
                CasResult::Stored { seq } => {
                    access.with(|s| s.mark_contacts_uploaded(seq))??;
                    break;
                }
                CasResult::Conflict { .. } => self.pull_contacts(access, report)?,
            }
        }
        Ok(())
    }

    fn pull_contacts(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let (ct, seq) = self.sharing_tx()?.get_contacts()?;
        if let Some(ct) = ct {
            match access.with(|s| s.apply_remote_contacts(&ct, seq))? {
                Ok(()) => {}
                // Does not authenticate under this account's key.
                Err(Error::Core(_)) => report.undecryptable += 1,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    // ---- the sharing step of a vault pull --------------------------------

    /// After a vault was pulled: applies the rosters of the held generation,
    /// claims the key shares that pass the claim rule and notes the members
    /// the server lists. If the step fails, the cursor goes back before the
    /// rosters it did not apply (a roster is served only newer than the
    /// cursor).
    pub(super) fn after_pull_sharing(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        collected: &Collected,
        report: &mut SyncReport,
        stats: &mut RunStats,
        follow: bool,
    ) -> Result<(), SyncError> {
        if collected.rotation_requested {
            stats.rotation_requested.insert(vault);
        }
        if !self.sharing_offered() {
            return Ok(());
        }
        let shared = access.with(|s| s.vault_shared(vault))??;
        let relevant = shared
            || !collected.shares.is_empty()
            || !collected.rosters.is_empty()
            || collected.role.is_some_and(|r| r != Role::Owner);
        if !relevant || Self::sharing_blocked(access)? {
            return Ok(());
        }
        let first_roster = collected.rosters.iter().filter_map(|r| r.seq).min();
        // Nothing says who owns the vault (no sharing row, no `owned:` or
        // `member:` entry): held, nothing applied, retried on later syncs
        // once the contact book brings an entry (M6b verify round 3).
        if !access.with(|s| s.vault_owner_known(vault))?? {
            if let Some(seq) = first_roster {
                Self::rewind_cursor(access, vault, seq)?;
            }
            return Self::note_finding(access, SharingFinding::VaultHeld { vault });
        }
        let result = self.sharing_step(access, vault, collected, report, stats, follow);
        if result.is_err()
            && let Some(seq) = first_roster
        {
            let back = seq.saturating_sub(1);
            // Best effort: the original error is the one to report.
            let _ = access.with(|s| -> Result<(), Error> {
                if cursor_head(s.sync_cursor(vault)?.as_deref()) > back {
                    s.set_sync_cursor(vault, &back.to_be_bytes())?;
                }
                Ok(())
            });
        }
        result
    }

    fn sharing_step(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        collected: &Collected,
        report: &mut SyncReport,
        stats: &mut RunStats,
        follow: bool,
    ) -> Result<(), SyncError> {
        let members = match self.sharing_tx()?.members(vault) {
            Ok(m) => m,
            Err(e) if is_lost(&e) => {
                if access.with(|s| s.vault_shared(vault))?? {
                    Self::refused_vault(access, vault)?;
                }
                return Ok(());
            }
            Err(e) => return Err(e.into()),
        };
        let Some(owner) = members.owner() else {
            return Ok(());
        };
        // A vault this device has never seen shared, whose owner the server
        // names as an account this account gave emergency access to: that
        // account holds the key after a release and could have forged the
        // roster (H3). Nothing of it is applied.
        if !access.with(|s| s.vault_shared(vault))??
            && access.with(|s| s.account_id())? != owner
            && self
                .sharing_tx()?
                .emergency()?
                .0
                .iter()
                .any(|g| g.grantee == owner)
        {
            return Self::note_finding(access, SharingFinding::RosterInvalid { vault });
        }
        stats.members.insert(vault, members);
        let held = Self::current_generation(access, vault)?;
        let mut current: Vec<&PulledRoster> = collected
            .rosters
            .iter()
            .filter(|r| r.generation == held)
            .collect();
        current.sort_by_key(|r| r.seq);
        for roster in current {
            Self::apply_roster(access, vault, roster, &owner, report)?;
        }
        let newer: Vec<PulledShare> = collected
            .shares
            .iter()
            .filter(|s| s.generation > held)
            .cloned()
            .collect();
        if newer.is_empty() {
            return Ok(());
        }
        if self.claim_shares(access, vault, collected, &newer, &owner, report)? && follow {
            // The records that waited for the key.
            self.pull_vault_inner(access, vault, report, stats, false)?;
        }
        Ok(())
    }

    /// Applies one pulled roster of the held generation.
    fn apply_roster(
        access: &mut impl SessionAccess,
        vault: VaultId,
        roster: &PulledRoster,
        owner: &AccountId,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let Some(seq) = roster.seq else {
            return Ok(());
        };
        let applied = access.with(|s| {
            s.apply_remote_roster(vault, roster.generation, &roster.ciphertext, seq, owner)
        })?;
        match applied {
            // Applied; or not a member of that roster (an emergency release,
            // an account the server injected); or another generation than
            // the held one, which waits for the key.
            Ok(())
            | Err(
                Error::NotInRoster
                | Error::KeyGenerationRollback { .. }
                | Error::KeyGenerationNotAccepted { .. },
            ) => {}
            // The session recorded `RosterInvalid` itself for an invalid
            // roster; an owner the server changed is the same finding.
            Err(Error::OwnerMismatch) => {
                Self::note_finding(access, SharingFinding::RosterInvalid { vault })?;
            }
            Err(Error::OwnerUnknown) => {
                Self::rewind_cursor(access, vault, seq)?;
                Self::note_finding(access, SharingFinding::VaultHeld { vault })?;
            }
            Err(Error::Core(_)) => report.undecryptable += 1,
            Err(e) => return Err(e.into()),
        }
        Ok(())
    }

    /// The claim rule (CRYPTO_SPEC §8.3): opens the chain of shares above the
    /// held generation, claims the final key on the server and stores it.
    /// Returns whether a key was claimed.
    fn claim_shares(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        collected: &Collected,
        newer: &[PulledShare],
        owner: &AccountId,
        report: &mut SyncReport,
    ) -> Result<bool, SyncError> {
        let install = access.with(|s| s.install_id())?;
        let mut ipks: BTreeMap<AccountId, IdentityPublicKey> = BTreeMap::new();
        for sender in newer.iter().map(|s| s.sender).collect::<BTreeSet<_>>() {
            if let Some(ipk) = self.sharing_tx()?.directory(&sender)? {
                ipks.insert(sender, ipk);
            }
        }
        let views: Vec<PulledShareView> = newer
            .iter()
            .map(|s| PulledShareView {
                generation: s.generation,
                sender: s.sender,
                seal: s.seal.clone(),
            })
            .collect();
        let history: Vec<_> = collected
            .rosters
            .iter()
            .map(|r| (r.generation, r.ciphertext.clone()))
            .collect();
        let opened = access
            .with(|s| s.open_share_chain(vault, &views, &history, &|a| ipks.get(a).copied()))?;
        let chain = match opened {
            Ok(chain) => chain,
            Err(Error::ShareUntrusted { generation }) => {
                let generation =
                    KeyGeneration::new(generation).map_err(|e| SyncError::Session(e.into()))?;
                let sender = newer
                    .iter()
                    .find(|s| s.generation == generation)
                    .or_else(|| newer.first())
                    .map(|s| s.sender);
                if let Some(sender) = sender {
                    Self::note_finding(
                        access,
                        SharingFinding::ShareUntrusted {
                            vault,
                            generation,
                            sender,
                        },
                    )?;
                }
                return Ok(false);
            }
            Err(Error::NothingToClaim | Error::NoIdentity | Error::NoRoster) => return Ok(false),
            Err(e) => return Err(e.into()),
        };
        let wrapped = access.with(|s| s.claim_wrap(&chain))??;
        let generation = chain.generation();
        match self
            .sharing_tx()?
            .claim_key(vault, generation, &wrapped, install)
        {
            Ok(_) => {}
            // Claimed by another device of the account meanwhile, or the
            // vault moved on: the key arrives as a key row.
            Err(
                TransportError::Rejected(Rejection::NoShare)
                | TransportError::GenerationMismatch { .. },
            ) => return Ok(false),
            Err(e) => return Err(e.into()),
        }
        access.with(|s| s.commit_claimed_key(chain, &wrapped))??;
        if let Some(roster) = collected
            .rosters
            .iter()
            .find(|r| r.generation == generation)
        {
            Self::apply_roster(access, vault, roster, owner, report)?;
        }
        if !report.claimed_vaults.contains(&vault) {
            report.claimed_vaults.push(vault);
        }
        if !report.changed_vaults.contains(&vault) {
            report.changed_vaults.push(vault);
        }
        Ok(true)
    }

    // ---- account-level steps between pull and push ----------------------

    /// Invites, queued roster uploads, revocation rotations and emergency
    /// seals.
    pub(super) fn sharing_phase(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<(), SyncError> {
        if !self.sharing_ready(access)? {
            return Ok(());
        }
        let outgoing = self.sync_invites(access, report)?;
        self.push_pending_rosters(access, report, stats)?;
        self.process_revocations(access, report, stats, &outgoing)?;
        self.process_rotation_requests(access, report, stats)?;
        self.refresh_emergency(access, report)
    }

    /// Rotates the shared vaults this account owns whose pull said a member
    /// re-keyed and asked for a rotation (M7, CRYPTO_SPEC §6.6): the old
    /// account key of that member still opens the vault until the owner
    /// rotates. Nobody is locked out by this: the plan is strict, so a member
    /// whose new identity key nobody here has verified yet (or whose key does
    /// not match the roster) keeps the rotation waiting
    /// ([`SyncWarning::RotationBlocked`]) instead of being revoked. The
    /// request is advisory (THREAT_MODEL R86): the server cannot force it.
    fn process_rotation_requests(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<(), SyncError> {
        for vault in core::mem::take(&mut stats.rotation_requested) {
            let owned = access.with(|s| -> Result<bool, Error> {
                Ok(s.vault_shared(vault)? && s.owns_vault(vault)?)
            })??;
            if !owned || report.rotated_vaults.contains(&vault) {
                continue;
            }
            self.rotate_shared(access, vault, &BTreeSet::new(), true, report, stats)?;
        }
        Ok(())
    }

    /// Counts the invites waiting for this account and turns a delivered
    /// invite that was declined or cancelled into a queued revocation (the
    /// recipient may hold the key; CRYPTO_SPEC §8.3), then removes it from
    /// the sender's list.
    fn sync_invites(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<Vec<OutgoingInvite>, SyncError> {
        let (incoming, outgoing) = self.sharing_tx()?.list_invites()?;
        report.invites_incoming = u32::try_from(incoming.len()).unwrap_or(u32::MAX);
        for o in &outgoing {
            let reason = match o.state {
                InviteState::Declined => RevocationReason::InviteDeclined,
                InviteState::Cancelled => RevocationReason::InviteCancelled,
                _ => continue,
            };
            if !o.delivered {
                continue;
            }
            match access.with(|s| s.request_revocation(o.vault_id, &o.recipient, reason))? {
                Ok(()) => {}
                Err(e @ Error::Store(_)) => return Err(e.into()),
                // Not a vault of this device, or not one this account manages.
                Err(_) => continue,
            }
            match self.sharing_tx()?.cancel_invite(&o.invite_id) {
                Ok(()) => {}
                Err(e) if is_lost(&e) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(outgoing)
    }

    /// Uploads the rosters that wait for it (a role change, a merge the
    /// server lacks); a conflict pulls the vault, which merges.
    fn push_pending_rosters(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<(), SyncError> {
        let vaults = access.with(|s| -> Result<Vec<VaultId>, Error> {
            let mut out = Vec::new();
            for v in s.synced_vaults() {
                if s.vault_shared(v)? && !s.vault_access_lost(v)? {
                    out.push(v);
                }
            }
            Ok(out)
        })??;
        for vault in vaults {
            for _ in 0..CAS_ATTEMPTS {
                let Some((ct, base)) = access.with(|s| s.pending_roster_upload(vault))?? else {
                    break;
                };
                match self
                    .sharing_tx()?
                    .put_roster(vault, ct.key_generation(), &ct, base)
                {
                    Ok(CasResult::Stored { seq }) => {
                        access.with(|s| roster_uploaded(s, vault, &ct, seq))??;
                        break;
                    }
                    Ok(CasResult::Conflict { current_seq }) => {
                        // The roster the server holds is served only above
                        // the cursor: step back to it, read it and merge.
                        Self::rewind_cursor(access, vault, current_seq)?;
                        self.pull_vault(access, vault, report, stats)?;
                    }
                    // Not allowed to write it (a role change on the server
                    // side): it stays queued.
                    Err(TransportError::Rejected(Rejection::RoleForbidden)) => {
                        Self::rotation_refused(access, vault, report)?;
                        break;
                    }
                    Err(e) if is_lost(&e) => {
                        Self::rotation_refused(access, vault, report)?;
                        break;
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }
        Ok(())
    }

    /// Roster members that count: not removed.
    fn active_members(roster: Vec<RosterMember>) -> BTreeMap<AccountId, RosterMember> {
        roster
            .into_iter()
            .filter(|m| m.role != Role::Removed)
            .map(|m| (m.account_id, m))
            .collect()
    }

    /// Roster members (not the owner) the server neither lists as a member
    /// nor as an open invite: they left, or their invite ended. With
    /// `skip_ghosts` the ones whose delivered-never invite ended are left
    /// out (nothing was handed to them, so nothing triggers a rotation).
    fn left_members(
        own: &AccountId,
        roster: Vec<RosterMember>,
        members: &MembersView,
        outgoing: &[OutgoingInvite],
        vault: VaultId,
        skip_ghosts: bool,
    ) -> BTreeSet<AccountId> {
        let ghosts: BTreeSet<AccountId> = outgoing
            .iter()
            .filter(|o| {
                o.vault_id == vault
                    && !o.delivered
                    && matches!(o.state, InviteState::Declined | InviteState::Cancelled)
            })
            .map(|o| o.recipient)
            .collect();
        let on_server: BTreeSet<AccountId> = members.members.iter().map(|m| m.account_id).collect();
        let invited: BTreeSet<AccountId> = members.invites.iter().map(|i| i.recipient).collect();
        Self::active_members(roster)
            .into_keys()
            .filter(|a| {
                let ghost = skip_ghosts && ghosts.contains(a);
                a != own && !on_server.contains(a) && !invited.contains(a) && !ghost
            })
            .collect()
    }

    /// Rotates the vaults this account owns whose revocations are queued or
    /// whose roster lists a member the server no longer has.
    pub(super) fn process_revocations(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
        stats: &mut RunStats,
        outgoing: &[OutgoingInvite],
    ) -> Result<(), SyncError> {
        let (own, pending, vaults) = access.with(|s| -> Result<_, Error> {
            let own = s.account_id();
            let pending = s.pending_revocations()?;
            let mut owned = Vec::new();
            for v in s.synced_vaults() {
                if s.vault_shared(v)? && s.owns_vault(v)? {
                    owned.push(v);
                }
            }
            Ok((own, pending, owned))
        })??;
        for vault in vaults {
            let explicit: BTreeSet<AccountId> = pending
                .iter()
                .filter(|(v, _, _)| *v == vault)
                .map(|(_, a, _)| *a)
                .collect();
            let members = match stats.members.get(&vault) {
                Some(m) => m.clone(),
                None => match self.sharing_tx()?.members(vault) {
                    Ok(m) => m,
                    // A revocation is waiting and the server refuses the
                    // member list: never skipped silently (SYNC-1).
                    Err(e) if is_lost(&e) => {
                        Self::rotation_refused(access, vault, report)?;
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                },
            };
            let roster = access.with(|s| s.vault_roster(vault))??.unwrap_or_default();
            let left = Self::left_members(&own, roster, &members, outgoing, vault, true);
            // A roster member the server dropped is shown as a finding (L2).
            // The rotation below leaves it out of the new key: a leave, a
            // manager's removal and a server's drop look the same, and none
            // may keep the key.
            // A drop a queued revocation explains (a declined or cancelled
            // invite) is no finding.
            for account in left.difference(&explicit) {
                Self::note_finding(
                    access,
                    SharingFinding::MemberDropped {
                        vault,
                        account: *account,
                    },
                )?;
            }
            if explicit.is_empty() && left.is_empty() {
                continue;
            }
            // A rotation that did not happen leaves the revocation queued:
            // that is visible, never a silent skip (SYNC-1).
            if self
                .rotate_shared(access, vault, &explicit, false, report, stats)?
                .is_none()
            {
                Self::warn_rotation_blocked(report, vault);
            }
        }
        Ok(())
    }

    /// Downloads the attachments of `vault` this device does not hold, so
    /// the rotation re-seals them (CRYPTO_SPEC §8.3). What cannot be fetched
    /// is listed by the session and keeps its old ID.
    fn fetch_vault_attachments(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<(), SyncError> {
        let missing = access.with(|s| -> Result<Vec<_>, Error> {
            let mut out = Vec::new();
            for r in s.attachment_references() {
                if r.vault_id == vault
                    && s.attachment_file_state(r.attachment_id)?
                        == mosaic_vault::session::AttachmentFileState::Missing
                {
                    out.push(r);
                }
            }
            Ok(out)
        })??;
        let mut budget = ATTACHMENT_DOWNLOAD_BUDGET;
        for r in missing {
            if budget == 0 {
                break;
            }
            let Some(ct) = self.transport.get_attachment(vault, r.attachment_id)? else {
                continue;
            };
            budget = budget.saturating_sub(u64::try_from(ct.len()).unwrap_or(u64::MAX));
            match access
                .with(|s| s.store_remote_attachment(vault, r.item_id, r.attachment_id, &ct))?
            {
                Err(Error::Store(mosaic_vault::storage::Error::AttachmentsUnavailable)) => break,
                Ok(())
                | Err(Error::Core(_) | Error::UnknownAttachmentReference | Error::Store(_)) => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(())
    }

    /// Cancels an invite that cannot be sealed under the new key.
    fn drop_invite(&mut self, invite: &InviteId) -> Result<(), SyncError> {
        match self.sharing_tx()?.cancel_invite(invite) {
            Ok(()) => Ok(()),
            Err(e) if is_lost(&e) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }

    /// The rotation plan of `vault` from the roster, the server's members
    /// and the emergency grants; `None` if a key the plan needs is not
    /// available (the rotation waits). A `strict` plan (the rotation a
    /// member's re-key asked for, and the re-key itself) locks nobody out and
    /// cancels no invite: where the normal plan would revoke a member or drop
    /// an invite because a key is unverified or does not match, it returns
    /// `None` instead.
    #[expect(
        clippy::too_many_lines,
        reason = "the plan of one rotation: members, invitees, grants and their checks belong together"
    )]
    pub(super) fn build_plan(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        explicit: &BTreeSet<AccountId>,
        members: &MembersView,
        strict: bool,
    ) -> Result<Option<RotationPlan>, SyncError> {
        let own = access.with(|s| s.account_id())?;
        let Some(roster) = access.with(|s| s.vault_roster(vault))?? else {
            return Ok(None);
        };
        let active = Self::active_members(roster.clone());
        let on_server: BTreeSet<AccountId> = members
            .members
            .iter()
            .map(|m| m.account_id)
            .filter(|a| *a != own)
            .collect();
        let mut revoked: BTreeSet<AccountId> = explicit.clone();
        // Left, or a declined invite: in the roster, nowhere on the server.
        revoked.extend(Self::left_members(&own, roster, members, &[], vault, false));
        // Listed by the server but not in the roster (an emergency release,
        // an account the server added): unconfirmed, revoked with the next
        // rotation.
        revoked.extend(
            on_server
                .iter()
                .filter(|a| !active.contains_key(a))
                .copied(),
        );
        revoked.remove(&own);

        let mut plan = RotationPlan::default();
        let mut invites: Vec<(InviteId, AccountId)> = Vec::new();
        for inv in &members.invites {
            if revoked.contains(&inv.recipient) || !active.contains_key(&inv.recipient) {
                if strict {
                    return Ok(None);
                }
                // An invite that cannot be sealed under the new key.
                self.drop_invite(&inv.invite_id)?;
            } else {
                invites.push((inv.invite_id, inv.recipient));
            }
        }
        for account in active.keys() {
            if *account == own || revoked.contains(account) || !on_server.contains(account) {
                continue;
            }
            // A member the directory does not serve is handled like one whose
            // key does not match: revoked, a finding, the rotation goes ahead.
            let Some(ipk) = self.sharing_tx()?.directory(account)? else {
                // A strict plan (the owner's own re-key) records no finding:
                // it would block the retry through the clean-sync rule, and
                // the refusal already tells the owner (review L2).
                if strict {
                    return Ok(None);
                }
                Self::note_finding(
                    access,
                    SharingFinding::MemberKeyMismatch {
                        vault,
                        account: *account,
                    },
                )?;
                revoked.insert(*account);
                continue;
            };
            // Only a contact this account verified gets the new key (H2).
            // A member nobody here compared codes with (a roster entry a
            // manager, or a removed member, wrote) is unconfirmed: no key
            // is sealed to them, the rotation locks them out, and the
            // finding tells the user. A served key that is not the roster's
            // fingerprint, or a pin that is Changed or Conflicting, is the
            // same: the member is revoked and the finding recorded, and the
            // rotation goes ahead (a key mismatch never stops it).
            let state = access.with(|s| s.contact_state(account, &ipk))??;
            let roster_key = active
                .get(account)
                .is_some_and(|m| m.fingerprint == Fingerprint::of(account, &ipk));
            let mismatch =
                !roster_key || matches!(state, ContactState::Changed | ContactState::Conflicting);
            if mismatch
                || matches!(
                    state,
                    ContactState::Unknown | ContactState::Pinned { verified: false }
                )
            {
                let account = *account;
                let finding = if mismatch {
                    SharingFinding::MemberKeyMismatch { vault, account }
                } else {
                    SharingFinding::MemberUnverified { vault, account }
                };
                if strict {
                    return Ok(None);
                }
                Self::note_finding(access, finding)?;
                revoked.insert(account);
                continue;
            }
            plan.remaining.push((*account, ipk));
        }
        for (invite, account) in invites {
            // The directory does not serve by ID the key of an account this
            // account invited (it would tell a real recipient from a fake
            // one): the key comes from the email lookup, with the address
            // the roster holds, and must be for the same account. An invitee
            // that cannot be resolved that way, or is not verified here,
            // loses the invite instead of blocking the revocation (L1).
            let email = active.get(&account).map(|m| m.email.clone());
            let entry = match email {
                Some(email) => Some(self.sharing_tx()?.lookup(&email)?),
                None => None,
            };
            let usable = match &entry {
                Some(e) if e.account_id == account => {
                    let state = access.with(|s| s.contact_state(&account, &e.ipk))??;
                    let roster_key = active
                        .get(&account)
                        .is_some_and(|m| m.fingerprint == Fingerprint::of(&account, &e.ipk));
                    // A changed or conflicting pin, or a key that is not the
                    // roster's, cancels the invite and is a key-mismatch
                    // finding; an unverified one is cancelled like a
                    // member nobody compared codes with.
                    let finding = invitee_finding(roster_key, state, vault, account);
                    if let Some(f) = &finding {
                        Self::note_finding(access, *f)?;
                    }
                    finding.is_none()
                }
                // The address no longer resolves to this account: handled
                // like a wrong key.
                Some(_) => {
                    Self::note_finding(
                        access,
                        SharingFinding::MemberKeyMismatch { vault, account },
                    )?;
                    false
                }
                None => false,
            };
            match entry {
                Some(e) if usable => plan.invites.push((invite, account, e.ipk)),
                _ if strict => return Ok(None),
                _ => {
                    self.drop_invite(&invite)?;
                    revoked.insert(account);
                }
            }
        }
        plan.revoked = revoked.into_iter().collect();

        let (granted, _) = self.sharing_tx()?.emergency()?;
        for g in granted.iter().filter(|g| covers(g, vault)) {
            let Some(ipk) = self.sharing_tx()?.directory(&g.grantee)? else {
                continue;
            };
            let verified = access.with(|s| s.contact_state(&g.grantee, &ipk))??
                == ContactState::Pinned { verified: true };
            if verified {
                plan.grants.push((g.grantee, ipk));
            }
        }
        Ok(Some(plan))
    }

    /// The owner's revocation rotation of a shared vault (CRYPTO_SPEC §8.3).
    /// `explicit` are the accounts a user or a declined invite queued; the
    /// plan adds members that left and members the roster does not list.
    /// Returns the new generation, or `None` if the rotation could not be
    /// done now (not the owner, a key the plan needs is missing, the server
    /// kept refusing): the revocations stay queued.
    ///
    /// # Errors
    /// [`SyncError`].
    #[expect(
        clippy::too_many_lines,
        clippy::cognitive_complexity,
        reason = "one rotation: prepare, offer, commit and the follow-up belong together"
    )]
    pub(super) fn rotate_shared(
        &mut self,
        access: &mut impl SessionAccess,
        vault: VaultId,
        explicit: &BTreeSet<AccountId>,
        strict: bool,
        report: &mut SyncReport,
        stats: &mut RunStats,
    ) -> Result<Option<KeyGeneration>, SyncError> {
        let install = access.with(|s| s.install_id())?;
        for _ in 0..ROTATION_ATTEMPTS {
            self.pull_vault(access, vault, report, stats)?;
            let members = match self.sharing_tx()?.members(vault) {
                Ok(m) => m,
                Err(e) if is_lost(&e) => {
                    Self::rotation_refused(access, vault, report)?;
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            };
            self.fetch_vault_attachments(access, vault)?;
            let Some(plan) = self.build_plan(access, vault, explicit, &members, strict)? else {
                report.warnings.push(SyncWarning::RotationBlocked { vault });
                return Ok(None);
            };
            let prepared = match access.with(|s| s.prepare_shared_rotation(vault, &plan))? {
                Ok(p) => p,
                Err(Error::PreparedStale) => continue,
                // A served key that does not match the roster or the pin, an
                // unverified grantee, a plan the roster does not support: the
                // rotation waits for the user.
                Err(
                    Error::MemberKeyMismatch
                    | Error::ContactUnverified
                    | Error::ContactKeyChanged
                    | Error::PlanMismatch
                    | Error::NoRoster,
                ) => {
                    report.warnings.push(SyncWarning::RotationBlocked { vault });
                    return Ok(None);
                }
                // Not the owner: only the owner's device rotates.
                Err(Error::NotOwner | Error::OwnerCannotBeRevoked) => return Ok(None),
                Err(e) => return Err(e.into()),
            };
            let head = cursor_head(access.with(|s| s.sync_cursor(vault))??.as_deref());
            let on_server: BTreeSet<AccountId> =
                members.members.iter().map(|m| m.account_id).collect();
            let emergency: Vec<(AccountId, ShareSeal)> = prepared.emergency_seals().to_vec();
            let generation = prepared.generation();
            let result = {
                let request = RotateRequest {
                    generation,
                    wrapped: prepared.wrapped(),
                    device: install,
                    expected_head: head,
                    roster: prepared.roster(),
                    seals: prepared.seals().to_vec(),
                    invite_seals: prepared.invite_seals().to_vec(),
                    revoke: prepared
                        .revoke()
                        .iter()
                        .filter(|a| on_server.contains(a))
                        .copied()
                        .collect(),
                };
                self.sharing_tx()?.rotate(vault, &request)
            };
            match result {
                Ok(RotateResult::Accepted { .. }) => {}
                // Another install of this account rotated first, the head or
                // the member list moved: read the vault and try again. Never
                // adopt (CRYPTO_SPEC §10.2).
                Ok(RotateResult::GenerationTaken { .. }) => {
                    drop(prepared);
                    self.pull_vault(access, vault, report, stats)?;
                    return Ok(None);
                }
                Ok(RotateResult::HeadMoved { .. } | RotateResult::MembersMismatch { .. }) => {
                    continue;
                }
                Err(TransportError::Rejected(Rejection::RoleForbidden)) => {
                    Self::rotation_refused(access, vault, report)?;
                    return Ok(None);
                }
                Err(e) if is_lost(&e) => {
                    Self::rotation_refused(access, vault, report)?;
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            }
            let outcome = match access.with(|s| s.commit_shared_rotation(prepared))? {
                Ok(o) => o,
                Err(Error::PreparedStale | Error::KeyGenerationRollback { .. }) => {
                    self.pull_vault(access, vault, report, stats)?;
                    return Ok(None);
                }
                Err(e) => return Err(e.into()),
            };
            report.rotated_vaults.push(vault);
            if !report.changed_vaults.contains(&vault) {
                report.changed_vaults.push(vault);
            }
            report.unreachable_attachments.extend(
                outcome
                    .unreachable_attachments
                    .iter()
                    .map(|(_, a)| (vault, *a)),
            );
            for (grantee, seal) in emergency {
                // The next run's refresh puts a seal again that was refused.
                tolerate(
                    self.sharing_tx()?
                        .put_grant_seals(&grantee, &[(vault, outcome.generation, seal)]),
                )?;
            }
            // The re-sealed records go up before anything else, and the pull
            // brings back the roster with the server's sequence number.
            self.push_phase(access, report, stats, Some(vault))?;
            // The metadata too: an invitee previews it with the key of the
            // new generation as soon as the server has rotated.
            self.push_metas(access, stats, report)?;
            self.pull_vault(access, vault, report, stats)?;
            return Ok(Some(outcome.generation));
        }
        report.warnings.push(SyncWarning::RotationBlocked { vault });
        Ok(None)
    }

    // ---- emergency seals --------------------------------------------------

    /// The vaults this account owns and that sync.
    pub(super) fn owned_vaults(
        access: &mut impl SessionAccess,
    ) -> Result<BTreeMap<VaultId, KeyGeneration>, SyncError> {
        Ok(access.with(|s| -> Result<_, Error> {
            let mut out = BTreeMap::new();
            for summary in s.vaults() {
                let v = summary.vault_id;
                if s.synced_vaults().contains(&v) && s.owns_vault(v)? {
                    out.insert(v, summary.key_generation);
                }
            }
            Ok(out)
        })??)
    }

    /// Makes sure a vault this account owns has its owner-only roster and
    /// sharing row.
    ///
    /// Returns whether the account's set of owned vaults changed (and so
    /// has to be uploaded).
    pub(super) fn ensure_owner_roster(
        access: &mut impl SessionAccess,
        vault: VaultId,
    ) -> Result<bool, SyncError> {
        let before = access.with(|s| s.vault_recorded_owned(vault))??;
        match access.with(|s| s.ensure_owner_roster(vault))? {
            // Done; or not an account-key vault of this account's, or no
            // identity key yet: nothing a grant could cover.
            Ok(()) => Ok(!before),
            Err(Error::WrongVaultProtection | Error::NotOwner | Error::NoIdentity) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }

    /// Counts the open emergency requests and keeps the seals of every grant
    /// fresh: every vault in scope whose seal is older than the local key
    /// (or missing under scope `all`) is sealed again (CRYPTO_SPEC §8.4).
    fn refresh_emergency(
        &mut self,
        access: &mut impl SessionAccess,
        report: &mut SyncReport,
    ) -> Result<(), SyncError> {
        let (granted, _) = self.sharing_tx()?.emergency()?;
        report.emergency_requests = u32::try_from(
            granted
                .iter()
                .filter(|g| g.state == GrantState::Waiting)
                .count(),
        )
        .unwrap_or(u32::MAX);
        if granted.is_empty() {
            return Ok(());
        }
        let owned = Self::owned_vaults(access)?;
        // A vault a grant covers gets the owner-only roster and the sharing
        // row, so a release (which adds a member without any invite) is
        // followed by the revocation rotation and the claim rule (H3).
        for v in owned.keys() {
            if granted.iter().any(|g| covers(g, *v)) {
                Self::ensure_owner_roster(access, *v)?;
            }
        }
        for g in &granted {
            if matches!(g.state, GrantState::Released) {
                continue;
            }
            let stale: Vec<VaultId> = owned
                .iter()
                .filter_map(|(v, current)| {
                    let sealed = g.vaults.iter().find(|(gv, _)| gv == v).map(|(_, n)| *n);
                    let in_scope = g.scope == GrantScope::All || sealed.is_some();
                    (in_scope && sealed.is_none_or(|n| n < *current)).then_some(*v)
                })
                .collect();
            if stale.is_empty() {
                continue;
            }
            let Some(ipk) = self.sharing_tx()?.directory(&g.grantee)? else {
                continue;
            };
            let sealed = access.with(|s| s.seal_emergency(&g.grantee, &ipk, &stale))?;
            let seals = match sealed {
                Ok(seals) => seals,
                // Not verified, or the key changed: the grantor decides in
                // the app, no seal is made.
                Err(
                    Error::ContactUnverified
                    | Error::ContactKeyChanged
                    | Error::WrongVaultProtection
                    | Error::NotOwner
                    | Error::VaultUnavailable,
                ) => continue,
                Err(e) => return Err(e.into()),
            };
            tolerate(self.sharing_tx()?.put_grant_seals(&g.grantee, &seals))?;
        }
        Ok(())
    }
}

/// Why an invitee gets no seal, if it does not: `None` for a verified pin and
/// the roster's key.
fn invitee_finding(
    roster_key: bool,
    state: ContactState,
    vault: VaultId,
    account: AccountId,
) -> Option<SharingFinding> {
    match state {
        ContactState::Pinned { verified: true } if roster_key => None,
        ContactState::Changed | ContactState::Conflicting => {
            Some(SharingFinding::MemberKeyMismatch { vault, account })
        }
        _ if !roster_key => Some(SharingFinding::MemberKeyMismatch { vault, account }),
        _ => Some(SharingFinding::MemberUnverified { vault, account }),
    }
}

/// Whether an emergency grant covers `vault`.
pub(super) fn covers(g: &GrantInfo, vault: VaultId) -> bool {
    match g.scope {
        GrantScope::All => true,
        GrantScope::Selected => g.vaults.iter().any(|(v, _)| *v == vault),
    }
}
