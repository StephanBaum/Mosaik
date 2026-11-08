//! The sync side of an account re-key (M7, CRYPTO_SPEC §6.6, §6.7, ADR-0057,
//! ADR-0058): the engine steps around the vault session's `prepare_rekey` and
//! `commit_rekey`, on both backends.
//!
//! - **Hosted** ([`SyncEngine::rekey_hosted`]): run to head, plan the owned
//!   shared vaults, prepare, one atomic request to the server
//!   ([`crate::http::hosted::rekey`]), commit locally. The session that made
//!   the request stays valid; every other session of the account ends. The
//!   caller connects the transport again with the returned token and calls
//!   [`SyncEngine::rekey_continue`] (attachments, uploads, close the job).
//! - **Folder** ([`SyncEngine::rekey_folder`]): run to head, prepare (the
//!   marker is built under the old keys), commit, then publish the new account
//!   file, a snapshot set of every vault and the marker.
//! - **Other devices** adopt: [`adopt_hosted_rekey`] and
//!   [`SyncEngine::adopt_folder_rekey`] with the new Secret Key and the
//!   password, and never at an equal epoch.
//!
//! Resuming a job that crashed after the server (or the typed keys) knew the
//! new account key needs the user to retype the last group of the new Secret
//! Key ([`RekeyError::KitGroupMismatch`], [`SyncEngine::resume_hosted_rekey`]):
//! the pending Secret Key sits in the OS keystore, and a retyped group proves
//! the person holds the new kit before the device adopts its own re-key.

#[cfg(feature = "http")]
use mosaic_core::KeyGeneration;
use mosaic_core::sync_auth::{self, FolderMacPurpose};
use mosaic_core::{
    Argon2Profile, AttachmentId, MasterPassword, PasswordParams, SecretKey, UnlockMethod, VaultId,
};
use mosaic_vault::session::{
    Error, FolderRekeyPlan, JoinKeys, LockedSession, RekeyAdopted, RekeyAdoption, RekeyCommitted,
    RekeyPlan, RekeyRequest, ResealStep, RotationKind, RotationPhase, UnlockedSession,
};
#[cfg(feature = "http")]
use mosaic_vault::session::{PreparedRekey, RotationJob, RotationPlan};
use mosaic_vault::storage::Error as StorageError;

use super::consts::ATTACHMENT_DOWNLOAD_BUDGET;
use super::maintain::Cx;
use super::{SessionAccess, SyncEngine, SyncError, SyncWarning};
use crate::clock::Clock;
use crate::transport::{RekeyPublish, Transport, TransportError};

#[cfg(feature = "http")]
/// Attempts at one hosted re-key when the server says a vault moved.
const REKEY_ATTEMPTS: usize = 3;
/// Account files a device that lags behind a re-key tries to open (each costs
/// one Argon2id derivation, and anyone with write access to a folder can plant
/// files).
const MAX_ADOPT_CANDIDATES: usize = 8;

/// Why a re-key step failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum RekeyError {
    /// The sync run before the re-key failed, or a halt, finding or pending
    /// revocation is open (`RekeyNeedsCleanSync`).
    #[error("sync must be clean before a re-key")]
    NeedsCleanSync(#[source] SyncError),
    /// The server kept refusing because vaults moved: pull and try again.
    #[error("the vaults kept changing")]
    Busy,
    /// A shared vault's members or invites cannot all be sealed to a verified
    /// key (`memberKeyMismatch`): nothing was changed.
    #[error("a member key is not verified")]
    MemberKeyMismatch,
    /// The re-key request was sent and its outcome is not known: the next
    /// resume probes again.
    #[error("re-key outcome unknown")]
    OutcomeUnknown,
    /// The retyped group is not the last group of the pending Secret Key.
    #[error("the kit group does not match")]
    KitGroupMismatch,
    /// There is no rotation job of this kind in this state.
    #[error("no matching rotation job")]
    NoJob,
    /// No account file in the folder opens under the typed keys.
    #[error("no account file opens with these keys")]
    NoMatchingAccountFile,
    /// The backend is not a folder, or not a hosted server.
    #[error("not available on this backend")]
    WrongBackend,
    /// The sync engine failed.
    #[error("sync error")]
    Sync(#[from] SyncError),
    /// The backend failed.
    #[error("transport error")]
    Transport(TransportError),
    /// The session failed.
    #[error("session error")]
    Session(#[from] Error),
}

impl From<TransportError> for RekeyError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Locked => Self::Sync(SyncError::Locked),
            TransportError::OutcomeUnknown => Self::OutcomeUnknown,
            other => Self::Transport(other),
        }
    }
}

/// The credentials of a re-key. The new Secret Key must already be saved and
/// verified in its kit; it must differ from the current one and from every
/// retired one, and the new password from the current one.
pub struct RekeySecrets<'a> {
    /// The current master password.
    pub current_password: &'a MasterPassword,
    /// The current Secret Key.
    pub current_secret_key: &'a SecretKey,
    /// The new master password.
    pub new_password: &'a MasterPassword,
    /// The new Secret Key.
    pub new_secret_key: &'a SecretKey,
    /// The new profile (not weaker than the current one).
    pub profile: Argon2Profile,
}

impl core::fmt::Debug for RekeySecrets<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("RekeySecrets([REDACTED])")
    }
}

/// How far [`SyncEngine::rekey_continue`] got.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RekeyProgress {
    /// Attachments the re-seal could not reach (neither held nor fetchable):
    /// they keep their old ID and file key (THREAT_MODEL R85).
    pub unreachable_attachments: Vec<(VaultId, AttachmentId)>,
    /// The download budget of this call ran out: call again.
    pub more_attachments: bool,
    /// Every re-sealed attachment and record is on the backend and the job is
    /// closed.
    pub finished: bool,
}

/// The server part of a hosted re-key: where and as whom.
#[cfg(feature = "http")]
#[derive(Clone, Copy, Debug)]
pub struct HostedRekey<'a> {
    /// The server's base URL.
    pub base: &'a str,
    /// The account's email address.
    pub address: &'a str,
    /// The name this device shows in the device list.
    pub device_name: &'a str,
    /// The current second-factor code, if the account has one.
    pub totp_code: Option<&'a str>,
}

/// A hosted re-key the server accepted and this device committed.
#[cfg(feature = "http")]
pub struct HostedRekeyDone {
    /// What the commit did: the new Secret Key to move to the normal keystore
    /// entry, and the quick-unlock and hardware-key wrappings it deleted.
    pub committed: RekeyCommitted,
    /// The session of the re-key (the only one left): connect the transport
    /// with its token.
    pub grant: crate::http::hosted::LoginGrant,
    /// The account version after the re-key.
    pub account_version: u64,
}

#[cfg(feature = "http")]
impl core::fmt::Debug for HostedRekeyDone {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("HostedRekeyDone")
            .field("account_version", &self.account_version)
            .finish_non_exhaustive()
    }
}

/// The step of a hosted resume.
#[cfg(feature = "http")]
#[derive(Debug)]
pub enum ResumeOutcome {
    /// The server had applied the re-key: this device adopted it.
    Adopted(RekeyAdopted),
    /// The probe proved the server did not apply it; the job is back at phase
    /// 1 and the app deletes the pending Secret Key or prepares again.
    NotApplied,
}

/// Records the journal phase of a hosted re-key stage. The first outcome is
/// kept unless a later write fails.
#[cfg(feature = "http")]
fn note_stage(
    access: &mut impl SessionAccess,
    staged: &mut Option<Result<(), SyncError>>,
    stage: crate::http::hosted::RekeyStage,
) {
    use crate::http::hosted::RekeyStage;
    let phase = match stage {
        RekeyStage::Started => RotationPhase::BackendStarted,
        RekeyStage::Accepted => RotationPhase::BackendAccepted,
    };
    let r = match access.with(|s| s.set_rotation_phase(phase)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(SyncError::Session(e)),
        Err(e) => Err(e),
    };
    if staged.is_none() || r.is_err() {
        *staged = Some(r);
    }
}

/// Records the floors of this device's own logs before an adoption: own
/// segments up to `reserved` are sealed under the epoch being left.
fn note_floors(s: &mut UnlockedSession, now: u64) -> Result<(), Error> {
    for v in s.synced_vaults() {
        let own = s.own_seq(v)?;
        if own.reserved > 0 {
            s.note_folder_rekey_floor(v, own.reserved, now)?;
        }
    }
    Ok(())
}

#[cfg(feature = "http")]
fn cursor_head(cursor: Option<&[u8]>) -> u64 {
    cursor
        .and_then(|c| <[u8; 8]>::try_from(c).ok())
        .map_or(0, u64::from_be_bytes)
}

/// How far above its own epoch an unverifiable account file may name one and
/// still count for the target epoch of a re-key.
const UNVERIFIED_EPOCH_JUMP: u32 = 8;

/// Opens the journal at phase 1 for a re-key, or takes back one that stopped
/// before the backend learned anything.
fn open_job(s: &mut UnlockedSession, target_version: Option<u64>) -> Result<(), Error> {
    match s.rotation_job()? {
        None => s.begin_rotation_job(RotationKind::Rekey, target_version),
        Some(job)
            if job.kind == RotationKind::Rekey
                && job.phase <= RotationPhase::BackendStarted
                && job.target_version == target_version =>
        {
            s.set_rotation_phase(RotationPhase::Prepared)
        }
        Some(job)
            if job.kind == RotationKind::Rekey && job.phase <= RotationPhase::BackendStarted =>
        {
            // A folder version that moved: start over.
            s.abandon_rotation_job().or_else(|_| Ok::<(), Error>(()))?;
            s.begin_rotation_job(RotationKind::Rekey, target_version)
        }
        Some(_) => Err(Error::RotationInProgress),
    }
}

impl<T: Transport> SyncEngine<T> {
    /// The signals of a re-key in the backend, before anything is written:
    /// a verified marker stops the run and sets the persisted flag; account
    /// files that do not verify come back as findings (never a stop).
    pub(super) fn check_rekey_signals(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<Vec<SyncWarning>, SyncError> {
        let epoch = access.with(|s| s.ak_epoch())?;
        let signals = self.transport.rekey_signals(epoch)?;
        if signals.marker.is_some() {
            access.with(|s| s.set_rekey_pending(true))??;
            return Err(SyncError::AccountRekeyed);
        }
        let mut out = Vec::new();
        for f in signals.unverified {
            if !access.with(|s| s.file_dismissed(&f.name, &f.sha256))?? {
                out.push(SyncWarning::UnverifiedAccountFile {
                    name: f.name,
                    sha256: f.sha256,
                    epoch: f.epoch,
                });
            }
        }
        Ok(out)
    }

    /// The user dismissed an [`SyncWarning::UnverifiedAccountFile`]: the file
    /// is remembered by name and SHA-256 and never reported again. ("Keep
    /// this device's re-key" is this call for an equal-epoch file.)
    ///
    /// # Errors
    /// [`SyncError::Locked`], [`SyncError::Session`].
    pub fn dismiss_account_file(
        &mut self,
        access: &mut impl SessionAccess,
        name: &str,
        sha256: &[u8; 32],
    ) -> Result<(), SyncError> {
        access.with(|s| s.dismiss_unverifiable_file(name, sha256))??;
        Ok(())
    }

    /// The run to head before a re-key. A failed run is
    /// [`RekeyError::NeedsCleanSync`]; a folder halt on a re-key marker does
    /// not stop it (a device that learned of a re-key elsewhere may re-key
    /// again).
    fn rekey_gate(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<(), RekeyError> {
        match self.sync_once(access, clock) {
            Ok(_) | Err(SyncError::AccountRekeyed) => Ok(()),
            Err(SyncError::Locked) => Err(RekeyError::Sync(SyncError::Locked)),
            Err(e) => Err(RekeyError::NeedsCleanSync(e)),
        }
    }

    // ---- hosted ---------------------------------------------------------------

    /// The plan of a hosted re-key: the expected head of every owned vault and,
    /// for every owned shared vault, who stays (strict: nobody is locked out
    /// and no invite is cancelled; a key that is not verified, or does not
    /// match the roster, stops the re-key).
    #[cfg(feature = "http")]
    fn hosted_rekey_plan(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<RekeyPlan, RekeyError> {
        let review = access.with(|s| s.rekey_review())??;
        let mut plan = RekeyPlan::default();
        let mut owned: Vec<VaultId> = review.owned_vaults.clone();
        owned.extend(review.shared_owned.iter().map(|v| v.vault_id));
        for v in &owned {
            let head = cursor_head(access.with(|s| s.sync_cursor(*v))??.as_deref());
            plan.heads.push((*v, head));
        }
        for shared in &review.shared_owned {
            let members = self.sharing_tx()?.members(shared.vault_id)?;
            let p: Option<RotationPlan> = self.build_plan(
                access,
                shared.vault_id,
                &std::collections::BTreeSet::new(),
                &members,
                true,
            )?;
            match p {
                Some(p) if p.revoked.is_empty() => plan.shared.push((shared.vault_id, p)),
                _ => return Err(RekeyError::MemberKeyMismatch),
            }
        }
        Ok(plan)
    }

    /// The hosted re-key (CRYPTO_SPEC §6.6), steps 1 to 5: a run to head, the
    /// plan, `prepare_rekey`, the atomic request to the server and the local
    /// commit. The journal moves to phase 2 before the request, to phase 3
    /// when the server accepted and to phase 5 with the commit.
    ///
    /// The server answers `head_moved` or `generation_taken` when a vault
    /// changed since the plan: nothing was applied, and this pulls and tries
    /// again, at most three times ([`RekeyError::Busy`]).
    ///
    /// After `Ok` the device's own session on the server is gone: connect the
    /// transport with the returned grant and call
    /// [`SyncEngine::rekey_continue`].
    ///
    /// # Errors
    /// [`RekeyError`]; the Secret Key and password errors of the vault
    /// (`PasswordUnchanged`, `SecretKeyUnchanged`, `UnlockFailed`) are
    /// [`RekeyError::Session`]; the server's refusals are
    /// [`RekeyError::Transport`] with a [`crate::transport::Rejection`].
    #[cfg(feature = "http")]
    pub fn rekey_hosted(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        hosted: &HostedRekey<'_>,
        secrets: &RekeySecrets<'_>,
    ) -> Result<HostedRekeyDone, RekeyError> {
        use crate::http::hosted::{self, RekeyOutcome, RekeyStage};
        for _ in 0..REKEY_ATTEMPTS {
            self.rekey_gate(access, clock)?;
            let plan = self.hosted_rekey_plan(access)?;
            access.with(|s| open_job(s, None))??;
            let prepared: PreparedRekey = match access.with(|s| {
                s.prepare_rekey(RekeyRequest {
                    current_password: secrets.current_password,
                    current_secret_key: secrets.current_secret_key,
                    new_password: secrets.new_password,
                    new_secret_key: secrets.new_secret_key,
                    profile: secrets.profile,
                    plan,
                })
            })? {
                Ok(p) => p,
                Err(Error::RekeyNotClean) => {
                    return Err(RekeyError::NeedsCleanSync(SyncError::Session(
                        Error::RekeyNotClean,
                    )));
                }
                Err(Error::PlanMismatch) => return Err(RekeyError::MemberKeyMismatch),
                Err(e) => return Err(e.into()),
            };
            let device = access.with(|s| s.install_id())?;
            let mut staged: Option<Result<(), SyncError>> = None;
            let outcome = {
                let mut on_stage = |stage: RekeyStage| note_stage(access, &mut staged, stage);
                hosted::rekey(
                    hosted.base,
                    hosted.address,
                    &prepared,
                    device,
                    hosted.device_name,
                    hosted.totp_code,
                    &mut on_stage,
                )
            };
            match outcome {
                Ok(RekeyOutcome::Done(finished)) => {
                    let version = finished.account_version;
                    // Phase 5 is entered by the commit itself; a journal write
                    // that failed above does not undo the server's change.
                    let committed = access.with(|s| s.commit_rekey(prepared))??;
                    access.with(|s| s.note_account_version(version))??;
                    if let Some(Err(e)) = staged {
                        return Err(e.into());
                    }
                    return Ok(HostedRekeyDone {
                        committed,
                        grant: finished.grant,
                        account_version: version,
                    });
                }
                Ok(RekeyOutcome::Refused(_)) => {
                    drop(prepared);
                    access.with(|s| s.set_rotation_phase(RotationPhase::Prepared))??;
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(RekeyError::Busy)
    }

    /// Resumes a hosted re-key whose journal stands at phase 2 or 3 (CRYPTO_SPEC
    /// §6.6, phase table). The prepared keys died with the process, so this
    /// device adopts its own re-key like any other device does, with the
    /// pending Secret Key from the keystore and the new password the user
    /// typed again. `typed_last_group` is the last group of the new Secret
    /// Key, retyped from the kit: without it nothing happens
    /// ([`RekeyError::KitGroupMismatch`]).
    ///
    /// At phase 2 the probe logs in with the new password: if the server did
    /// not apply the re-key the login fails, the journal returns to phase 1
    /// and the answer is [`ResumeOutcome::NotApplied`] (the app deletes the
    /// pending Secret Key or prepares the re-key again). Attachments are not
    /// re-sealed after an adoption (the prepared queue died); the job closes.
    ///
    /// # Errors
    /// [`RekeyError`].
    #[cfg(feature = "http")]
    pub fn resume_hosted_rekey(
        &mut self,
        access: &mut impl SessionAccess,
        hosted: &HostedRekey<'_>,
        new_password: &MasterPassword,
        pending_secret_key: &SecretKey,
        current_secret_key: &SecretKey,
        typed_last_group: &str,
    ) -> Result<ResumeOutcome, RekeyError> {
        use crate::transport::Rejection;
        let job: RotationJob = access
            .with(|s| s.rotation_job())??
            .filter(|j| {
                j.kind == RotationKind::Rekey
                    && matches!(
                        j.phase,
                        RotationPhase::BackendStarted | RotationPhase::BackendAccepted
                    )
            })
            .ok_or(RekeyError::NoJob)?;
        if !pending_secret_key.matches_last_group(typed_last_group) {
            return Err(RekeyError::KitGroupMismatch);
        }
        let device = access.with(|s| s.install_id())?;
        match adopt_hosted_rekey(
            access,
            hosted,
            new_password,
            pending_secret_key,
            current_secret_key,
            device,
        ) {
            Ok(adopted) => {
                access.with(UnlockedSession::finish_rotation_job)??;
                Ok(ResumeOutcome::Adopted(adopted))
            }
            // At phase 2 the new password does not log in when the server did
            // not apply the re-key: the change was not applied.
            Err(RekeyError::Transport(TransportError::Rejected(Rejection::LoginFailed)))
                if job.phase == RotationPhase::BackendStarted =>
            {
                access.with(|s| s.set_rotation_phase(RotationPhase::Prepared))??;
                Ok(ResumeOutcome::NotApplied)
            }
            Err(e) => Err(e),
        }
    }

    // ---- folder ---------------------------------------------------------------

    /// The folder re-key (CRYPTO_SPEC §6.6): a run to head, `prepare_rekey`
    /// (it builds the marker under the old `K_fold`), the local commit (the
    /// journal moves to phase 5 and every vault's floor is recorded) and the
    /// publication: the new account file at the highest verified version plus
    /// one, a snapshot set of every vault under the new keys, the marker and
    /// the device info. The journal then stands at phase 4; every step
    /// repeats safely after a crash ([`SyncEngine::rekey_continue`] finishes
    /// the publication of a job at phase 5).
    ///
    /// The new epoch is this device's plus one. A device that learned of a
    /// re-key elsewhere (a verified marker) may re-key again; the two
    /// re-keys are then different accounts keys at the same epoch, which a
    /// device sees as an unverifiable account file of its own epoch until the
    /// user dismisses it or adopts the other (CRYPTO_SPEC §10.1).
    ///
    /// # Errors
    /// [`RekeyError`].
    pub fn rekey_folder(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        secrets: &RekeySecrets<'_>,
    ) -> Result<RekeyCommitted, RekeyError> {
        self.rekey_gate(access, clock)?;
        let next = self
            .transport
            .next_folder_account_version()?
            .ok_or(RekeyError::WrongBackend)?;
        let seen = access.with(|s| s.highest_account_version())??;
        let version = next.max(seen.checked_add(1).ok_or(RekeyError::WrongBackend)?);
        let own_epoch = access.with(|s| s.ak_epoch())?;
        let seen_epoch = self.highest_epoch_seen(own_epoch)?;
        access.with(|s| open_job(s, Some(version)))??;
        let now = clock.now_ms();
        let prepared = match access.with(|s| {
            s.prepare_rekey(RekeyRequest {
                current_password: secrets.current_password,
                current_secret_key: secrets.current_secret_key,
                new_password: secrets.new_password,
                new_secret_key: secrets.new_secret_key,
                profile: secrets.profile,
                plan: RekeyPlan {
                    folder: Some(FolderRekeyPlan {
                        to_account_version: version,
                        created_at_ms: now,
                    }),
                    seen_epoch,
                    ..RekeyPlan::default()
                },
            })
        })? {
            Ok(p) => p,
            Err(Error::RekeyNotClean) => {
                return Err(RekeyError::NeedsCleanSync(SyncError::Session(
                    Error::RekeyNotClean,
                )));
            }
            Err(e) => return Err(e.into()),
        };
        let committed = access.with(|s| s.commit_rekey(prepared))??;
        access.with(|s| s.note_account_version(version))??;
        self.publish_folder_rekey(access, now)?;
        Ok(committed)
    }

    /// The highest epoch the folder shows: a verified marker, and an
    /// unverifiable account file whose (unauthenticated) epoch is at most
    /// `UNVERIFIED_EPOCH_JUMP` above `own`, so a planted huge epoch cannot
    /// push a re-key out of reach. 0 if there is none.
    fn highest_epoch_seen(&mut self, own: u32) -> Result<u32, RekeyError> {
        let signals = self.transport.rekey_signals(own)?;
        let limit = own.saturating_add(UNVERIFIED_EPOCH_JUMP);
        let marker = signals.marker.map(|m| m.to_epoch);
        let files = signals
            .unverified
            .iter()
            .map(|f| f.epoch)
            .filter(|e| *e <= limit);
        Ok(marker.into_iter().chain(files).max().unwrap_or(0))
    }

    /// Writes the new account file, the snapshot sets and the marker of the
    /// job's re-key and records phase 4.
    fn publish_folder_rekey(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
    ) -> Result<(), RekeyError> {
        let (job, boot, vaults) = access.with(|s| -> Result<_, Error> {
            Ok((s.rotation_job()?, s.account_bootstrap()?, s.synced_vaults()))
        })??;
        let job = job.ok_or(RekeyError::NoJob)?;
        let version = job.target_version.ok_or(RekeyError::NoJob)?;
        let from_epoch = job.from_epoch.ok_or(RekeyError::NoJob)?;
        let publish = RekeyPublish {
            account_file: Some((&boot.params, &boot.wrapped_ak, version)),
            snapshot_vaults: &vaults,
            marker: job.folder_marker.as_ref(),
            from_epoch,
        };
        self.publish(access, now, &publish)?;
        access.with(|s| s.set_rotation_phase(RotationPhase::FolderWritten))??;
        Ok(())
    }

    fn publish(
        &mut self,
        access: &mut impl SessionAccess,
        now: u64,
        publish: &RekeyPublish<'_>,
    ) -> Result<(), RekeyError> {
        let mut cx = Cx::new(access, now);
        let result = self.transport.publish_rekey(&mut cx, publish);
        if let Some(e) = cx.take_failure() {
            return Err(e.into());
        }
        result?;
        Ok(())
    }

    /// Adopts a re-key another device made on a folder (CRYPTO_SPEC §6.7,
    /// ADR-0058) with the new Secret Key and the password: the account files
    /// that name this device's epoch plus one are tried best first (the one a
    /// verified marker names, then by version; at most eight, each costs one
    /// Argon2id derivation). The first whose wrapping opens under the typed
    /// keys and whose MAC verifies under the key it holds is adopted; the
    /// floors of this device's own logs are recorded first, and this device
    /// writes a snapshot set of every vault under the new keys afterwards.
    ///
    /// A typed Secret Key that is this device's current one or a retired one is
    /// refused ([`Error::SecretKeyUnchanged`]): an attacker who holds the old
    /// password and Secret Key cannot move a device to a key he made up.
    ///
    /// # Errors
    /// [`RekeyError::NoMatchingAccountFile`] when none opens; [`RekeyError`].
    pub fn adopt_folder_rekey(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
        typed_password: &MasterPassword,
        typed_secret_key: &SecretKey,
        current_secret_key: &SecretKey,
    ) -> Result<RekeyAdopted, RekeyError> {
        let epoch = access.with(|s| s.ak_epoch())?;
        let candidates = self
            .transport
            .rekey_candidates(epoch, MAX_ADOPT_CANDIDATES)?;
        for c in candidates.into_iter().take(MAX_ADOPT_CANDIDATES) {
            let params = PasswordParams {
                account_id: c.file.account_id,
                salt: c.file.salt,
                profile: c.file.profile,
            };
            if !file_opens(
                &c.file.wrapped_ak,
                &c.bytes,
                &params,
                typed_password,
                typed_secret_key,
            )? {
                continue;
            }
            let keys: JoinKeys =
                LockedSession::derive_join_keys(&params, typed_password, typed_secret_key)?;
            let now = clock.now_ms();
            // The floors of the own logs, before the adoption: own segments
            // up to `reserved` are sealed under the epoch being left.
            access.with(|s| note_floors(s, now))??;
            let adopted = access.with(|s| {
                s.adopt_remote_rekey(RekeyAdoption {
                    keys,
                    typed_secret_key,
                    current_secret_key,
                    wrapped_ak: &c.file.wrapped_ak,
                    params,
                    epoch: c.file.ak_epoch,
                    identity: None,
                    member_wraps: &[],
                })
            })??;
            access.with(|s| s.note_account_version(c.file.account_version))??;
            let vaults = access.with(|s| s.synced_vaults())?;
            self.publish(
                access,
                now,
                &RekeyPublish {
                    account_file: None,
                    snapshot_vaults: &vaults,
                    marker: None,
                    from_epoch: epoch,
                },
            )?;
            return Ok(adopted);
        }
        Err(RekeyError::NoMatchingAccountFile)
    }

    // ---- after the commit -----------------------------------------------------

    /// The rest of a re-key after the local commit (journal phases 5 to 7): a
    /// folder job at phase 5 finishes its publication first; every attachment
    /// of an owned vault is re-sealed under a fresh ID and file key (fetched
    /// through the transport when this device does not hold it, within the
    /// download budget of one call; one that cannot be had is listed and keeps
    /// its old key); then runs to head until the re-sealed records, keys and
    /// files are on the backend, and the job closes. A crash loses at most one
    /// attachment step; call it again.
    ///
    /// For a hosted re-key the transport must carry the token of
    /// [`HostedRekeyDone::grant`] by now.
    ///
    /// # Errors
    /// [`RekeyError::NoJob`] without a committed re-key job; [`RekeyError`].
    pub fn rekey_continue(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<RekeyProgress, RekeyError> {
        let job = access
            .with(|s| s.rotation_job())??
            .filter(|j| {
                j.kind == RotationKind::Rekey
                    && matches!(
                        j.phase,
                        RotationPhase::FolderWritten
                            | RotationPhase::LocalCommitted
                            | RotationPhase::Attachments
                            | RotationPhase::Uploading
                    )
            })
            .ok_or(RekeyError::NoJob)?;
        if job.phase == RotationPhase::LocalCommitted && job.folder_marker.is_some() {
            self.publish_folder_rekey(access, clock.now_ms())?;
        }
        access.with(|s| s.set_rotation_phase(RotationPhase::Attachments))??;
        let mut progress = RekeyProgress::default();
        let mut budget = ATTACHMENT_DOWNLOAD_BUDGET;
        let mut fetched: Option<Vec<u8>> = None;
        let mut asked: Option<(VaultId, AttachmentId)> = None;
        loop {
            let held = fetched.take();
            let step = access.with(|s| s.reseal_next_attachment(held.as_deref()))?;
            let step = match step {
                Ok(step) => step,
                // Bytes that do not authenticate under the file key: this
                // blob cannot be re-sealed (it keeps its old key).
                Err(Error::Core(_)) if held.is_some() => {
                    if let Some(row) = asked.take() {
                        mark_unreachable(access, &mut progress, row)?;
                    }
                    continue;
                }
                Err(e) => return Err(e.into()),
            };
            match step {
                ResealStep::Done => break,
                ResealStep::Resealed { .. } | ResealStep::Skipped { .. } => {}
                ResealStep::NeedsFetch {
                    vault_id,
                    attachment_id,
                    ..
                } => {
                    if budget == 0 {
                        progress.more_attachments = true;
                        return Ok(progress);
                    }
                    match self.transport.get_attachment(vault_id, attachment_id) {
                        Ok(Some(bytes)) => {
                            budget = budget.saturating_sub(bytes.len() as u64);
                            asked = Some((vault_id, attachment_id));
                            fetched = Some(bytes);
                        }
                        Ok(None) | Err(TransportError::NotFound) => {
                            mark_unreachable(access, &mut progress, (vault_id, attachment_id))?;
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }
        access.with(|s| s.set_rotation_phase(RotationPhase::Uploading))??;
        for _ in 0..3 {
            self.sync_once(access, clock)?;
            if self.rekey_finish_if_clean(access)? {
                break;
            }
        }
        progress.finished = access.with(|s| s.rotation_job())??.is_none();
        Ok(progress)
    }

    /// Closes the re-key job when nothing is left to upload: no re-sealed
    /// record, vault key or attachment file is queued. Returns whether the job
    /// is closed now.
    ///
    /// # Errors
    /// [`RekeyError`].
    pub fn rekey_finish_if_clean(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<bool, RekeyError> {
        let clean = access.with(|s| -> Result<bool, Error> {
            Ok(s.pending_uploads(1)?.is_empty()
                && s.pending_attachment_uploads()?.is_empty()
                && s.pending_vault_uploads()?.is_empty())
        })??;
        if !clean {
            return Ok(false);
        }
        match access.with(UnlockedSession::finish_rotation_job)? {
            Ok(()) => Ok(true),
            Err(Error::Store(StorageError::RotationNotFinished)) => Ok(false),
            Err(e) => Err(e.into()),
        }
    }
}

fn mark_unreachable(
    access: &mut impl SessionAccess,
    progress: &mut RekeyProgress,
    (vault, attachment): (VaultId, AttachmentId),
) -> Result<(), RekeyError> {
    access.with(|s| s.mark_attachment_unreachable(attachment))??;
    progress.unreachable_attachments.push((vault, attachment));
    Ok(())
}

/// Whether `wrapped` opens under the keys of the typed password and Secret Key
/// (derived under `params`) and the account key it holds verifies the account
/// file's MAC (CRYPTO_SPEC §10.1). The key is dropped at once.
fn file_opens(
    wrapped: &mosaic_core::WrappedKey,
    bytes: &[u8],
    params: &PasswordParams,
    password: &MasterPassword,
    secret_key: &SecretKey,
) -> Result<bool, RekeyError> {
    let derived = mosaic_core::derive_password_keys(password, secret_key, params)
        .map_err(|e| RekeyError::Session(Error::Core(e)))?;
    let Ok(ak) =
        wrapped.unwrap_account_key(&derived.kek, &params.account_id, &UnlockMethod::Password)
    else {
        return Ok(false);
    };
    let fold = ak
        .folder_mac_key()
        .map_err(|e| RekeyError::Session(Error::Core(e)))?;
    let Some((body, mac)) = bytes.split_last_chunk::<32>() else {
        return Ok(false);
    };
    sync_auth::verify_folder_mac(&fold, FolderMacPurpose::Account, body, mac)
        .map_err(|e| RekeyError::Session(Error::Core(e)))
}

/// Adopts a re-key another device made on the hosted backend (CRYPTO_SPEC
/// §6.7): logs in with the typed new password and the typed new Secret Key
/// ([`crate::http::hosted::recover_rekey`]; the old Secret Key does not log
/// in), refuses an account version below the highest this device has seen,
/// and hands the server's wrapping and the key rows it serves for the vaults
/// shared to this account to the vault. Only the next epoch is adopted
/// (checked arithmetic), never an equal one, and a typed Secret Key this
/// device has held is refused. The server's epoch is not authenticated: the
/// wrapping must open under the typed keys.
///
/// # Errors
/// [`RekeyError`]; a refused adoption is [`RekeyError::Session`]
/// (`EpochNotNext`, `EpochRollback`, `PasswordRollback`, `SecretKeyUnchanged`,
/// `UnlockFailed`).
#[cfg(feature = "http")]
pub fn adopt_hosted_rekey(
    access: &mut impl SessionAccess,
    hosted: &HostedRekey<'_>,
    typed_password: &MasterPassword,
    typed_secret_key: &SecretKey,
    current_secret_key: &SecretKey,
    device: mosaic_core::InstallId,
) -> Result<RekeyAdopted, RekeyError> {
    let members: Vec<(VaultId, KeyGeneration)> = access.with(|s| -> Result<_, Error> {
        let review = s.rekey_review()?;
        let gens: std::collections::BTreeMap<VaultId, KeyGeneration> = s
            .vaults()
            .into_iter()
            .map(|v| (v.vault_id, v.key_generation))
            .collect();
        Ok(review
            .member_vaults
            .iter()
            .filter_map(|m| gens.get(&m.vault_id).map(|g| (m.vault_id, *g)))
            .collect())
    })??;
    let recovered = crate::http::hosted::recover_rekey(&crate::http::hosted::RecoverRequest {
        base: hosted.base,
        address: hosted.address,
        new_password: typed_password,
        new_secret_key: typed_secret_key,
        device,
        device_name: hosted.device_name,
        totp_code: hosted.totp_code,
        member_vaults: &members,
    })?;
    access.with(|s| s.check_account_version(recovered.account.account_version))??;
    let identity = recovered
        .account
        .identity
        .as_ref()
        .map(|(ipk, wrapped)| (wrapped, ipk));
    let params = *recovered.keys.params();
    let version = recovered.account.account_version;
    let epoch = recovered.account.ak_epoch;
    let wrapped = recovered.account.wrapped_ak.clone();
    let member_wraps = recovered.member_wraps;
    let keys = recovered.keys;
    let adopted = access.with(|s| {
        s.adopt_remote_rekey(RekeyAdoption {
            keys,
            typed_secret_key,
            current_secret_key,
            wrapped_ak: &wrapped,
            params,
            epoch,
            identity,
            member_wraps: &member_wraps,
        })
    })??;
    access.with(|s| s.note_account_version(version))??;
    Ok(adopted)
}
