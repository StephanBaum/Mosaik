//! Account keys (M7, CRYPTO_SPEC §5.3, §6.4 to §6.7, ADR-0057 to ADR-0062):
//! the Secret Key rotation, the KDF profile upgrade, the full account re-key
//! and the crash resume of all three, plus the adoption of a re-key another
//! device made.
//!
//! - The new Secret Key is made here, kept in the **pending** keystore entry
//!   ([`Backend::pending_secret_keys`]) and printed only into the new
//!   Emergency Kit. The commit refuses without a kit the user saved and
//!   verified in this process (`kitNotVerified`); it moves the key to the
//!   normal entry after the vault committed.
//! - Every step that talks to a backend runs with the sync thread stopped, a
//!   lock epoch recorded first and the session slot held for single closures
//!   only (the engine and the transports never hold it across the network).
//!   A lock waits for the running step ([`SyncService::begin_change`], three
//!   minutes at most); the job journal resumes what a lock or a crash cut off.
//! - The webview never sees a key: it types passwords, the last kit group and
//!   second-factor codes ([`SecretIn`]).

use mosaic_core::{
    AccountId, Argon2Profile, CredentialChange, InstallId, MasterPassword, SecretKey,
};
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{
    EngineConfig, HostedRekey, RekeyError, RekeySecrets, ResumeOutcome, SyncEngine, SyncError,
    adopt_hosted_rekey,
};
use mosaic_sync::http::{HttpTransport, StoredToken, hosted};
use mosaic_sync::transport::{Rejection, Transport, TransportError, TransportKind};
use mosaic_sync::wire::json::LoginStatus;
use mosaic_vault::session::{
    PasswordChanged, QuickUnlockEnrollment, RekeyCommitted, RotationJob, RotationKind as VaultKind,
    RotationPhase, SyncConfig, SyncKind,
};
use zeroize::Zeroizing;

use super::access::BackendSession;
use super::errors::{sync_code, transport_code};
use super::hosted_flow::code_of;
use super::open_folder;
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::events::AppEvent;
use crate::contract::rotation::{
    DismissFileRequest, MemberVaultView, ProfileUpgradeRequest, RekeyBackend, RekeyCommitRequest,
    RekeyPrepareRequest, RekeyReviewView, RotateCommitRequest, RotatePrepareRequest, RotationDone,
    RotationJobView, RotationKind, RotationNeeds, RotationPhaseName, RotationProfile,
    RotationResumeRequest, RotationStatus, SharedOwnedView,
};
use crate::contract::secret::SecretIn;
use crate::contract::sync::{RecoverPasswordRequest, SyncOverview};
use crate::state::{Backend, guard};

/// The most `rekey_continue` calls one command makes (each re-seals attachments
/// within the engine's download budget).
const MAX_CONTINUE_CALLS: usize = 64;
/// The longest an unlock may take with a stronger profile before
/// `profileTooSlow` asks the user to confirm.
const SLOW_UNLOCK: std::time::Duration = std::time::Duration::from_secs(2);

fn password_of(secret: &SecretIn) -> IpcResult<MasterPassword> {
    let password = MasterPassword::new(secret.expose_secret().to_owned());
    if password.expose_secret().is_empty() {
        return Err(IpcError::new(ErrorCode::EmptyPassword));
    }
    Ok(password)
}

const fn kind_view(kind: VaultKind) -> RotationKind {
    match kind {
        VaultKind::SecretKey => RotationKind::SecretKey,
        VaultKind::Rekey => RotationKind::Rekey,
        VaultKind::Profile => RotationKind::Profile,
        VaultKind::Vault => RotationKind::Vault,
    }
}

const fn phase_view(phase: RotationPhase) -> RotationPhaseName {
    match phase {
        RotationPhase::Prepared => RotationPhaseName::Prepared,
        RotationPhase::BackendStarted => RotationPhaseName::BackendStarted,
        RotationPhase::BackendAccepted => RotationPhaseName::BackendAccepted,
        RotationPhase::FolderWritten => RotationPhaseName::FolderWritten,
        RotationPhase::LocalCommitted => RotationPhaseName::LocalCommitted,
        RotationPhase::Attachments => RotationPhaseName::Attachments,
        RotationPhase::Uploading => RotationPhaseName::Uploading,
    }
}

/// What continuing a job needs from the user.
const fn needs_of(job: &RotationJob) -> RotationNeeds {
    match (job.kind, job.phase) {
        (_, RotationPhase::Prepared) | (VaultKind::Profile, RotationPhase::BackendStarted) => {
            RotationNeeds::Password
        }
        (
            VaultKind::SecretKey | VaultKind::Rekey,
            RotationPhase::BackendStarted | RotationPhase::BackendAccepted,
        ) => RotationNeeds::KitGroup,
        _ => RotationNeeds::None,
    }
}

fn job_view(job: &RotationJob) -> RotationJobView {
    RotationJobView {
        kind: kind_view(job.kind),
        phase: phase_view(job.phase),
        done: job.done,
        total: job.total,
        needs: needs_of(job),
    }
}

fn profile_of(profile: RotationProfile) -> Argon2Profile {
    match profile {
        RotationProfile::Standard => Argon2Profile::Standard,
        RotationProfile::Strong => Argon2Profile::Strong,
        RotationProfile::Max => Argon2Profile::Max,
    }
}

/// The IPC error of a failed re-key step.
fn rekey_error(error: RekeyError) -> IpcError {
    match error {
        RekeyError::NeedsCleanSync(SyncError::Session(e)) | RekeyError::Session(e) => {
            IpcError::from(e)
        }
        RekeyError::NeedsCleanSync(_) => IpcError::new(ErrorCode::RekeyNeedsCleanSync),
        RekeyError::Busy => IpcError::new(ErrorCode::RekeyBusy),
        RekeyError::MemberKeyMismatch => IpcError::new(ErrorCode::RekeyMemberKeyMismatch),
        RekeyError::OutcomeUnknown => IpcError::new(ErrorCode::RekeyOutcomeUnknown),
        RekeyError::KitGroupMismatch => IpcError::new(ErrorCode::KitGroupMismatch),
        RekeyError::NoJob => IpcError::new(ErrorCode::RotationNone),
        RekeyError::NoMatchingAccountFile => IpcError::new(ErrorCode::RekeyNoAccountFile),
        RekeyError::WrongBackend => IpcError::new(ErrorCode::SyncNotConfigured),
        RekeyError::Sync(SyncError::Session(e)) => IpcError::from(e),
        RekeyError::Sync(e) => IpcError::new(sync_code(&e)),
        RekeyError::Transport(TransportError::Unreachable | TransportError::Timeout) => {
            IpcError::new(ErrorCode::RekeyNeedsServer)
        }
        RekeyError::Transport(e) => IpcError::new(transport_code(e)),
        _ => IpcError::INTERNAL,
    }
}

/// The account, device, backend choice and the Secret Key in use.
struct Who {
    account: AccountId,
    install: InstallId,
    config: Option<SyncConfig>,
    current_key: SecretKey,
    profile: Argon2Profile,
}

/// What a re-key did here, before the data follows.
struct Renewed {
    quick_unlock: bool,
    hardware_keys: u32,
}

impl Backend {
    fn who(&self) -> IpcResult<Who> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let account = session.account_id();
        Ok(Who {
            account,
            install: session.install_id(),
            config: session.sync_config()?,
            current_key: self
                .secret_key_of(account)
                .ok_or(IpcError::new(ErrorCode::SecretKeyRequired))?,
            profile: session.account_bootstrap()?.params.profile,
        })
    }

    fn emit_rotation_progress(&self) {
        let job = self.slot().ok().and_then(|slot| {
            slot.unlocked()
                .ok()
                .and_then(|s| s.rotation_job().ok().flatten())
        });
        if let Some(job) = job {
            self.emit(&AppEvent::RotationProgress {
                kind: kind_view(job.kind),
                phase: phase_view(job.phase),
                done: job.done,
                total: job.total,
            });
        }
    }

    /// `rotation_status`.
    ///
    /// # Errors
    /// `locked`, or the store's code.
    pub fn rotation_status(&self) -> IpcResult<RotationStatus> {
        let slot = self.slot()?;
        let job = slot.unlocked()?.rotation_job()?;
        Ok(RotationStatus {
            job: job.as_ref().map(job_view),
        })
    }

    /// Opens (or takes back) the journal row of `kind` at phase 1 and stages
    /// the new Secret Key when the kind has one. A job of another kind, or one
    /// that went further, is `rotationInProgress`.
    fn stage_rotation(&self, kind: VaultKind) -> IpcResult<()> {
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let account = session.account_id();
        let fresh = match session.rotation_job()? {
            None => {
                session.begin_rotation_job(kind, None)?;
                true
            }
            Some(job) if job.kind == kind && job.phase == RotationPhase::Prepared => false,
            Some(_) => return Err(IpcError::new(ErrorCode::RotationInProgress)),
        };
        *guard(&self.pending_kit_verified)? = None;
        if matches!(kind, VaultKind::Profile) {
            return Ok(());
        }
        if fresh || self.pending_secret_keys.load(account)?.is_none() {
            let key = SecretKey::generate()?;
            if let Err(error) = self.pending_secret_keys.save(account, &key) {
                let _ = session.abandon_rotation_job();
                return Err(error);
            }
        }
        Ok(())
    }

    /// The kit check of a commit: saved and verified in this process.
    fn require_new_kit(&self, account: AccountId) -> IpcResult<SecretKey> {
        if *guard(&self.pending_kit_verified)? != Some(account) {
            return Err(IpcError::new(ErrorCode::KitNotVerified));
        }
        self.pending_secret_keys
            .load(account)?
            .ok_or(IpcError::new(ErrorCode::RotationNone))
    }

    /// The platform key of a quick-unlock enrollment a rotation deleted.
    pub(super) fn remove_platform_enrollment(&self, removed: Option<&QuickUnlockEnrollment>) {
        if let Some(enrollment) = removed
            && let Some(platform) = self.quick_unlock.biometric()
        {
            let _ = platform.remove(&enrollment.platform_params);
        }
    }

    /// The new Secret Key replaces the one in use in the keystore. The pending
    /// entry is deleted only once the normal entry holds the key.
    fn promote_secret_key(&self, account: AccountId, key: &SecretKey) -> IpcResult<()> {
        self.secret_keys.save(account, key)?;
        let _ = self.pending_secret_keys.delete(account);
        *guard(&self.pending_kit_verified)? = None;
        Ok(())
    }

    // ---- Secret Key rotation and profile upgrade ----------------------------------

    /// `secret_key_rotate_prepare`: checks the password, opens the job and
    /// stages the new Secret Key; the kit flow follows (`pending: true`).
    ///
    /// # Errors
    /// `locked`, `emptyPassword`, `wrongCredentials`, `rotationInProgress`.
    pub fn secret_key_rotate_prepare(&self, request: &RotatePrepareRequest) -> IpcResult<()> {
        self.verify_master_only(&password_of(&request.password)?)?;
        self.stage_rotation(VaultKind::SecretKey)
    }

    /// `secret_key_rotate_commit`: the backend learns the new Secret Key (a
    /// fresh login on the server, a new account file in the folder), then this
    /// device commits and moves the key to the normal keystore entry.
    ///
    /// # Errors
    /// `kitNotVerified`, `rotationNone`, `wrongCredentials`, `secretKeyUnchanged`,
    /// `syncPasswordNeedsServer`, `syncTwoFactorRequired`,
    /// `syncPasswordOutcomeUnknown` and the transport's `sync*` codes.
    pub fn secret_key_rotate_commit(
        &self,
        request: &RotateCommitRequest,
    ) -> IpcResult<RotationDone> {
        self.commit_credentials(
            &request.password,
            request.code.as_ref(),
            VaultKind::SecretKey,
            None,
        )
    }

    /// `profile_upgrade`: a stronger key derivation, same keys (no kit).
    ///
    /// # Errors
    /// `profileDowngrade` (not stronger), `profileTooSlow` (this device needs
    /// more than two seconds with it and `force` is off), and those of
    /// `secret_key_rotate_commit`.
    pub fn profile_upgrade(&self, request: &ProfileUpgradeRequest) -> IpcResult<RotationDone> {
        let target = profile_of(request.profile);
        let current = self.who()?.profile;
        if target <= current {
            return Err(IpcError::new(ErrorCode::ProfileDowngrade));
        }
        if !request.force {
            let took = mosaic_core::kdf::measure_profile(target)?;
            if took > SLOW_UNLOCK {
                return Err(IpcError::new(ErrorCode::ProfileTooSlow));
            }
        }
        self.verify_master_only(&password_of(&request.password)?)?;
        self.stage_rotation(VaultKind::Profile)?;
        self.commit_credentials(
            &request.password,
            request.code.as_ref(),
            VaultKind::Profile,
            Some(target),
        )
    }

    fn commit_credentials(
        &self,
        password: &SecretIn,
        code: Option<&SecretIn>,
        kind: VaultKind,
        profile: Option<Argon2Profile>,
    ) -> IpcResult<RotationDone> {
        let current = password_of(password)?;
        self.verify_master_only(&current)?;
        let who = self.who()?;
        let new_key = match kind {
            VaultKind::SecretKey => Some(self.require_new_kit(who.account)?),
            _ => None,
        };
        let _change = self.sync.begin_change()?;
        let prepared = {
            let slot = self.slot()?;
            let session = slot.unlocked()?;
            session.prepare_credential_change(
                &current,
                &who.current_key,
                &CredentialChange {
                    new_password: None,
                    new_secret_key: new_key.as_ref(),
                    profile: profile.unwrap_or(who.profile),
                },
            )?
        };
        drop(current);
        let sends = who.config.is_some();
        if sends {
            self.with_unlocked(|s| Ok(s.set_rotation_phase(RotationPhase::BackendStarted)?))?;
        }
        let applied = self.apply_credential_change(
            who.config.clone(),
            who.account,
            who.install,
            code,
            prepared,
        );
        let changed = match applied {
            Ok(changed) => changed,
            Err(error) => {
                // Nothing reached the backend for these errors: the job goes
                // back to phase 1 and can be discarded.
                let sent = matches!(
                    error.code,
                    ErrorCode::SyncPasswordOutcomeUnknown | ErrorCode::SyncPasswordChanged
                );
                if sends && !sent {
                    let _ =
                        self.with_unlocked(|s| Ok(s.set_rotation_phase(RotationPhase::Prepared)?));
                }
                return Err(error);
            }
        };
        // The vault committed: whatever happens next, the key moves.
        if let Some(key) = &new_key {
            self.promote_secret_key(who.account, key)?;
        }
        self.remove_platform_enrollment(changed.removed_quick_unlock.as_ref());
        let _ = self.with_unlocked(|s| Ok(s.finish_rotation_job()?));
        self.emit_rotation_progress();
        Ok(RotationDone {
            quick_unlock_removed: changed.removed_quick_unlock.is_some(),
            hardware_keys_removed: 0,
            unreachable_attachments: 0,
            finished: true,
        })
    }

    // ---- re-key -------------------------------------------------------------------

    /// `rekey_review`: what a re-key touches.
    ///
    /// # Errors
    /// `locked`, `rekeyNeedsCleanSync`, or the store's code.
    pub fn rekey_review_view(&self) -> IpcResult<RekeyReviewView> {
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let review = session.rekey_review()?;
        let names: std::collections::BTreeMap<_, _> = session
            .vaults()
            .into_iter()
            .map(|v| (v.vault_id, v.name.map(str::to_owned)))
            .collect();
        let name = |id: &mosaic_core::VaultId| names.get(id).cloned().flatten();
        let backend = match session.sync_config()?.map(|c| c.kind) {
            Some(SyncKind::Hosted) => RekeyBackend::Hosted,
            Some(SyncKind::Folder) => RekeyBackend::Folder,
            None => RekeyBackend::Local,
        };
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        Ok(RekeyReviewView {
            owned_vaults: count(review.owned_vaults.len()),
            shared_owned: review
                .shared_owned
                .iter()
                .map(|v| SharedOwnedView {
                    vault_id: v.vault_id.into(),
                    name: name(&v.vault_id),
                    members: count(v.members.len()),
                })
                .collect(),
            member_vaults: review
                .member_vaults
                .iter()
                .map(|v| MemberVaultView {
                    vault_id: v.vault_id.into(),
                    name: name(&v.vault_id),
                })
                .collect(),
            hardware_vaults: count(review.tk_vaults.len()),
            grants: count(review.grants.len()),
            hardware_keys: review.hardware_keys,
            quick_unlock: review.quick_unlock,
            unreachable_attachments: count(review.unreachable_attachments.len()),
            backend,
            account_id: session.account_id().into(),
        })
    }

    /// `rekey_prepare`: checks both passwords, opens the job and stages the new
    /// Secret Key; the kit flow follows.
    ///
    /// # Errors
    /// `emptyPassword`, `wrongCredentials`, `passwordUnchanged`,
    /// `syncNotConfigured` (a re-key renews what a backend holds),
    /// `rotationInProgress`.
    pub fn rekey_prepare(&self, request: &RekeyPrepareRequest) -> IpcResult<()> {
        let current = password_of(&request.password)?;
        let new = password_of(&request.new_password)?;
        if current.expose_secret() == new.expose_secret() {
            return Err(IpcError::new(ErrorCode::PasswordUnchanged));
        }
        if self.who()?.config.is_none() {
            return Err(IpcError::new(ErrorCode::SyncNotConfigured));
        }
        self.verify_master_only(&current)?;
        self.stage_rotation(VaultKind::Rekey)
    }

    /// `rekey_commit`: the account's keys are renewed on the backend and here,
    /// then the attachments and the re-sealed data follow.
    ///
    /// # Errors
    /// `kitNotVerified`, `passwordUnchanged`, `wrongCredentials`,
    /// `rekeyNeedsCleanSync`, `rekeyMemberKeyMismatch`, `rekeyBusy`,
    /// `rekeyNeedsServer`, `rekeyOutcomeUnknown`, `syncTwoFactorRequired` and the
    /// transport's `sync*` codes. After the vault committed, a failure of the
    /// follow-up is not an error: `finished` is `false`.
    pub fn rekey_commit(&self, request: &RekeyCommitRequest) -> IpcResult<RotationDone> {
        let current = password_of(&request.password)?;
        let new = password_of(&request.new_password)?;
        if current.expose_secret() == new.expose_secret() {
            return Err(IpcError::new(ErrorCode::PasswordUnchanged));
        }
        self.verify_master_only(&current)?;
        let who = self.who()?;
        let new_key = self.require_new_kit(who.account)?;
        let _change = self.sync.begin_change()?;
        let config = who
            .config
            .clone()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let secrets = RekeySecrets {
            current_password: &current,
            current_secret_key: &who.current_key,
            new_password: &new,
            new_secret_key: &new_key,
            profile: who.profile,
        };
        let renewed = match config.kind {
            SyncKind::Hosted => {
                self.rekey_hosted_flow(&who, &config, request.code.as_ref(), &secrets)
            }
            SyncKind::Folder => self.rekey_folder_flow(&who, &config, &secrets),
        };
        drop((current, new, new_key));
        self.sync_restart();
        let (renewed, finished) = renewed?;
        Ok(self.rotation_done(&renewed, finished))
    }

    fn rotation_done(&self, renewed: &Renewed, finished: bool) -> RotationDone {
        let unreachable = self
            .slot()
            .ok()
            .and_then(|s| {
                s.unlocked()
                    .ok()
                    .and_then(|u| u.unreachable_attachments().ok())
            })
            .map_or(0, |rows| u32::try_from(rows.len()).unwrap_or(u32::MAX));
        RotationDone {
            quick_unlock_removed: renewed.quick_unlock,
            hardware_keys_removed: renewed.hardware_keys,
            unreachable_attachments: unreachable,
            finished,
        }
    }

    /// What the device does at once after `commit_rekey`: the Secret Key moves
    /// to the normal entry, the platform key of quick unlock goes, and
    /// `settings.json` is protected again under the new account key.
    fn after_local_rekey(&self, account: AccountId, committed: &RekeyCommitted) -> Renewed {
        let renewed = Renewed {
            quick_unlock: committed.removed_quick_unlock.is_some(),
            hardware_keys: u32::try_from(committed.removed_hardware_keys.len()).unwrap_or(u32::MAX),
        };
        // A key that cannot be written stays in the pending entry; the next
        // unlock asks for it from the kit.
        let _ = self.promote_secret_key(account, &committed.new_secret_key);
        self.remove_platform_enrollment(committed.removed_quick_unlock.as_ref());
        self.settings_remac();
        self.emit_rotation_progress();
        renewed
    }

    fn rekey_hosted_flow(
        &self,
        who: &Who,
        config: &SyncConfig,
        code: Option<&SecretIn>,
        secrets: &RekeySecrets<'_>,
    ) -> IpcResult<(Renewed, bool)> {
        let ctx = self.hosted_ctx()?;
        let code = code_of(code)?;
        if code.is_none() && self.totp_enabled_of(&ctx) {
            return Err(IpcError::new(ErrorCode::SyncTwoFactorRequired));
        }
        let token = self.stored_token(who.account)?;
        self.sync.stop();
        let mut access = self.sync.session()?;
        let mut engine = SyncEngine::new(
            HttpTransport::connect(&ctx.url, &token, who.install)?,
            EngineConfig::for_kind(TransportKind::Hosted),
        );
        let hosted = HostedRekey {
            base: &ctx.url,
            address: &ctx.email,
            device_name: &config.device_name,
            totp_code: code,
        };
        let done = engine
            .rekey_hosted(&mut access, &SystemClock, &hosted, secrets)
            .map_err(rekey_error)?;
        let renewed = self.after_local_rekey(who.account, &done.committed);
        // The session of the re-key is the only one left.
        let stored = StoredToken::from_session(&done.grant.token);
        if self.sync.save_token(self, who.account, &stored).is_err() {
            let _ = self.sync_tokens.delete(who.account);
            self.sync_force_signed_out();
            return Ok((renewed, false));
        }
        let finished = match HttpTransport::connect(&ctx.url, &stored, who.install) {
            Ok(transport) => {
                let mut engine =
                    SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
                self.continue_loop(&mut engine, &mut access)?
            }
            Err(_) => false,
        };
        Ok((renewed, finished))
    }

    fn rekey_folder_flow(
        &self,
        who: &Who,
        config: &SyncConfig,
        secrets: &RekeySecrets<'_>,
    ) -> IpcResult<(Renewed, bool)> {
        let root = config
            .folder_path
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        self.sync.stop();
        let mut access = self.sync.session()?;
        let transport = open_folder(
            self.sync.session()?,
            root,
            who.account,
            who.install,
            &config.device_name,
        )?;
        let mut engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Folder));
        let committed = engine
            .rekey_folder(&mut access, &SystemClock, secrets)
            .map_err(rekey_error)?;
        let renewed = self.after_local_rekey(who.account, &committed);
        let finished = self.continue_loop(&mut engine, &mut access)?;
        Ok((renewed, finished))
    }

    /// `rekey_continue` until the job closed or nothing more can be done now.
    /// After the vault committed a failure of the follow-up is not an error
    /// (the journal resumes it); only a lock is.
    fn continue_loop<T: Transport>(
        &self,
        engine: &mut SyncEngine<T>,
        access: &mut BackendSession,
    ) -> IpcResult<bool> {
        for _ in 0..MAX_CONTINUE_CALLS {
            match engine.rekey_continue(access, &SystemClock) {
                Ok(progress) => {
                    self.emit_rotation_progress();
                    if progress.finished {
                        return Ok(true);
                    }
                    if !progress.more_attachments {
                        return Ok(false);
                    }
                }
                Err(RekeyError::Sync(SyncError::Locked)) => return Err(IpcError::LOCKED),
                Err(_) => return Ok(false),
            }
        }
        Ok(false)
    }

    // ---- resume and abandon -------------------------------------------------------

    /// `rotation_resume`: continues the open job with what its `needs` asks for.
    ///
    /// # Errors
    /// `rotationNone`, `invalidInput` (a missing password or kit group, or a job
    /// still at phase 1: the wizard starts again there), `kitGroupMismatch`,
    /// `rotationNotApplied` (the probe proved the change never reached the
    /// backend; the job is back at phase 1), `wrongCredentials` and the codes of
    /// `rekey_commit`.
    pub fn rotation_resume(&self, request: &RotationResumeRequest) -> IpcResult<RotationDone> {
        let who = self.who()?;
        let job = self
            .slot()?
            .unlocked()?
            .rotation_job()?
            .ok_or(IpcError::new(ErrorCode::RotationNone))?;
        let _change = self.sync.begin_change()?;
        let result = match (job.kind, job.phase) {
            (_, RotationPhase::Prepared) | (VaultKind::Vault, _) => {
                Err(IpcError::new(ErrorCode::InvalidInput))
            }
            (VaultKind::Rekey, RotationPhase::BackendStarted | RotationPhase::BackendAccepted) => {
                self.resume_hosted_rekey(&who, request)
            }
            (
                VaultKind::SecretKey | VaultKind::Profile,
                RotationPhase::BackendStarted | RotationPhase::BackendAccepted,
            ) => self.resume_credentials(&who, job.kind, request),
            (VaultKind::Rekey, _) => self.resume_rekey_data(&who),
            (_, _) => {
                let _ = self.with_unlocked(|s| Ok(s.finish_rotation_job()?));
                Ok(RotationDone {
                    quick_unlock_removed: false,
                    hardware_keys_removed: 0,
                    unreachable_attachments: 0,
                    finished: true,
                })
            }
        };
        self.sync_restart();
        result
    }

    /// Phase 4 to 7 of a re-key: the data follows.
    fn resume_rekey_data(&self, who: &Who) -> IpcResult<RotationDone> {
        let config = who
            .config
            .clone()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        self.sync.stop();
        let mut access = self.sync.session()?;
        let finished = match config.kind {
            SyncKind::Hosted => {
                let ctx = self.hosted_ctx()?;
                let token = self.stored_token(who.account)?;
                let transport = HttpTransport::connect(&ctx.url, &token, who.install)?;
                let mut engine =
                    SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
                self.continue_loop(&mut engine, &mut access)?
            }
            SyncKind::Folder => {
                let root = config
                    .folder_path
                    .as_deref()
                    .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
                let transport = open_folder(
                    self.sync.session()?,
                    root,
                    who.account,
                    who.install,
                    &config.device_name,
                )?;
                let mut engine =
                    SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Folder));
                self.continue_loop(&mut engine, &mut access)?
            }
        };
        Ok(self.rotation_done(
            &Renewed {
                quick_unlock: false,
                hardware_keys: 0,
            },
            finished,
        ))
    }

    /// Phase 2 or 3 of a hosted re-key: the prepared keys died with the
    /// process, so this device adopts its own re-key with the pending Secret
    /// Key, the new password and the last group retyped from the new kit.
    fn resume_hosted_rekey(
        &self,
        who: &Who,
        request: &RotationResumeRequest,
    ) -> IpcResult<RotationDone> {
        let new = password_of(
            request
                .password
                .as_ref()
                .ok_or(IpcError::new(ErrorCode::InvalidInput))?,
        )?;
        let group = Zeroizing::new(
            request
                .kit_group
                .as_ref()
                .ok_or(IpcError::new(ErrorCode::InvalidInput))?
                .expose_secret()
                .to_owned(),
        );
        let pending = self
            .pending_secret_keys
            .load(who.account)?
            .ok_or(IpcError::new(ErrorCode::RotationNone))?;
        let ctx = self.hosted_ctx()?;
        let code = code_of(request.code.as_ref())?;
        self.sync.stop();
        // The transport only has to exist: the adoption logs in by itself, and
        // the stored token is dead after a re-key.
        let token = match self.stored_token(who.account) {
            Ok(token) => token,
            Err(_) => {
                StoredToken::from_session(&mosaic_core::sync_auth::session_token(&[1u8; 32])?)
            }
        };
        let mut access = self.sync.session()?;
        let mut engine = SyncEngine::new(
            HttpTransport::connect(&ctx.url, &token, who.install)?,
            EngineConfig::for_kind(TransportKind::Hosted),
        );
        let hosted = HostedRekey {
            base: &ctx.url,
            address: &ctx.email,
            device_name: &ctx.device_name,
            totp_code: code,
        };
        let outcome = engine
            .resume_hosted_rekey(
                &mut access,
                &hosted,
                &new,
                &pending,
                &who.current_key,
                &group,
            )
            .map_err(rekey_error)?;
        match outcome {
            ResumeOutcome::NotApplied => Err(IpcError::new(ErrorCode::RotationNotApplied)),
            ResumeOutcome::Adopted(adopted) => {
                self.promote_secret_key(who.account, &pending)?;
                self.remove_platform_enrollment(adopted.removed_quick_unlock.as_ref());
                self.settings_remac();
                // The session of the adoption is not kept by the engine: sign in
                // with the password just proved. A second factor needs a new
                // code, so it falls back to the sign-in form.
                self.sign_in_after_adoption(
                    who,
                    &ctx.url,
                    &ctx.email,
                    &ctx.device_name,
                    &adopted.opaque_password,
                );
                self.emit_rotation_progress();
                Ok(RotationDone {
                    quick_unlock_removed: adopted.removed_quick_unlock.is_some(),
                    hardware_keys_removed: u32::try_from(adopted.removed_hardware_keys.len())
                        .unwrap_or(u32::MAX),
                    unreachable_attachments: 0,
                    finished: true,
                })
            }
        }
    }

    /// A login with the adopted password. A failure is not fatal: the device
    /// is signed out and the sign-in form takes over.
    fn sign_in_after_adoption(
        &self,
        who: &Who,
        url: &str,
        email: &str,
        device: &str,
        opw: &mosaic_core::OpaquePassword,
    ) {
        match hosted::login(url, email, who.account, opw, who.install, device) {
            Ok(grant) if grant.status == LoginStatus::Ok => {
                let stored = StoredToken::from_session(&grant.token);
                if self.sync.save_token(self, who.account, &stored).is_err() {
                    let _ = self.sync_tokens.delete(who.account);
                    self.sync_force_signed_out();
                }
            }
            _ => {
                let _ = self.sync_tokens.delete(who.account);
                self.sync_force_signed_out();
            }
        }
    }

    /// Phase 2 of a Secret Key rotation or a profile upgrade: did the backend
    /// apply it? The device derives the keys the change would have created (the
    /// pending Secret Key, or the unchanged one for a profile upgrade) and
    /// adopts them if the backend holds them.
    fn resume_credentials(
        &self,
        who: &Who,
        kind: VaultKind,
        request: &RotationResumeRequest,
    ) -> IpcResult<RotationDone> {
        let password = password_of(
            request
                .password
                .as_ref()
                .ok_or(IpcError::new(ErrorCode::InvalidInput))?,
        )?;
        let pending = if kind == VaultKind::SecretKey {
            let group = request
                .kit_group
                .as_ref()
                .ok_or(IpcError::new(ErrorCode::InvalidInput))?;
            let key = self
                .pending_secret_keys
                .load(who.account)?
                .ok_or(IpcError::new(ErrorCode::RotationNone))?;
            if !key.matches_last_group(group.expose_secret()) {
                return Err(IpcError::new(ErrorCode::KitGroupMismatch));
            }
            Some(key)
        } else {
            None
        };
        self.verify_master_only(&password)?;
        let config = who
            .config
            .clone()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let key = pending.as_ref().unwrap_or(&who.current_key);
        let epoch = self.sync.epoch();
        let adopted = match config.kind {
            SyncKind::Hosted => self.probe_hosted(who, &config, &password, key, request, epoch)?,
            SyncKind::Folder => {
                match self.recover_folder(&config, who.account, who.install, &password, key, epoch)
                {
                    Ok(changed) => Some(changed),
                    // The folder holds no newer account file: not applied.
                    Err(e)
                        if matches!(
                            e.code,
                            ErrorCode::SyncPasswordRollback | ErrorCode::SyncFolderNoAccount
                        ) =>
                    {
                        None
                    }
                    Err(e) => return Err(e),
                }
            }
        };
        let Some(changed) = adopted else {
            let _ = self.with_unlocked(|s| Ok(s.set_rotation_phase(RotationPhase::Prepared)?));
            return Err(IpcError::new(ErrorCode::RotationNotApplied));
        };
        if let Some(key) = &pending {
            self.promote_secret_key(who.account, key)?;
        }
        self.remove_platform_enrollment(changed.removed_quick_unlock.as_ref());
        let _ = self.with_unlocked(|s| Ok(s.finish_rotation_job()?));
        self.emit_rotation_progress();
        Ok(RotationDone {
            quick_unlock_removed: changed.removed_quick_unlock.is_some(),
            hardware_keys_removed: 0,
            unreachable_attachments: 0,
            finished: true,
        })
    }

    /// The hosted probe: `None` when the server does not hold the credentials
    /// (the login fails), else the adopted change.
    fn probe_hosted(
        &self,
        who: &Who,
        config: &SyncConfig,
        password: &MasterPassword,
        key: &SecretKey,
        request: &RotationResumeRequest,
        epoch: u64,
    ) -> IpcResult<Option<PasswordChanged>> {
        let url = config
            .server_url
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        let email = config.email.clone().unwrap_or_default();
        let code = code_of(request.code.as_ref())?;
        self.sync.stop();
        let result = hosted::recover_password(
            url,
            &email,
            password,
            key,
            who.install,
            &config.device_name,
            code,
        );
        let outcome = match result {
            Err(TransportError::Rejected(Rejection::LoginFailed)) => Ok(None),
            Err(error) => Err(error.into()),
            Ok(recovered) => {
                self.sync.check_epoch(epoch)?;
                let hosted::Recovered {
                    keys,
                    account: remote,
                    grant,
                } = recovered;
                let params = *keys.params();
                let changed = self.with_unlocked(|session| {
                    Ok(session.adopt_remote_password(keys, &remote.wrapped_ak, params)?)
                })?;
                let token = StoredToken::from_session(&grant.token);
                if self.sync.save_token(self, who.account, &token).is_err() {
                    let _ = self.sync_tokens.delete(who.account);
                    self.sync_force_signed_out();
                }
                Ok(Some(changed))
            }
        };
        self.sync_restart();
        outcome
    }

    /// `rotation_abandon`: drops a job the backend never saw and the pending
    /// Secret Key with it.
    ///
    /// # Errors
    /// `rotationNone`, `rotationTooFar` (the change reached the backend).
    pub fn rotation_abandon(&self) -> IpcResult<()> {
        let account = {
            let mut slot = self.slot()?;
            let session = slot.unlocked_mut()?;
            session.abandon_rotation_job()?;
            session.account_id()
        };
        self.pending_secret_keys.delete(account)?;
        *guard(&self.pending_kit_verified)? = None;
        self.emit_rotation_progress();
        Ok(())
    }

    // ---- other devices ------------------------------------------------------------

    /// `sync_recover_password` with the Secret Key from the newest kit: the
    /// account's keys were renewed on another device. A Secret Key rotation (the
    /// same account key) is adopted like a password change; a re-key (a new
    /// account key) by the engine's adoption, which refuses a Secret Key this
    /// device has held (current or retired).
    pub(super) fn recover_with_new_key(
        &self,
        request: &RecoverPasswordRequest,
        typed: &SecretIn,
    ) -> IpcResult<SyncOverview> {
        let typed = SecretKey::parse(typed.expose_secret())?;
        let new = password_of(&request.new_password)?;
        let code = code_of(request.code.as_ref())?;
        let who = self.who()?;
        let config = who
            .config
            .clone()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        {
            let slot = self.slot()?;
            let retired = slot.unlocked()?.retired_secret_keys()?;
            if typed == who.current_key || typed.is_retired(&who.account, &retired) {
                return Err(IpcError::new(ErrorCode::SecretKeyUnchanged));
            }
        }
        let epoch = self.sync.epoch();
        let (quick, rekeyed) = match config.kind {
            SyncKind::Hosted => self.adopt_hosted(&who, &config, &new, &typed, code, epoch)?,
            SyncKind::Folder => self.adopt_folder(&who, &config, &new, &typed, epoch)?,
        };
        drop(new);
        // The new key is the key in use; the platform key of quick unlock goes.
        self.secret_keys.save(who.account, &typed)?;
        self.remove_platform_enrollment(quick.as_ref());
        if rekeyed {
            self.settings_remac();
        }
        self.quick_unlock.clear_reauth_failures();
        self.sync.update_view(|view| {
            view.password_changed = false;
            true
        });
        self.emit_sync_state();
        self.sync_restart();
        self.sync_status()
    }

    fn adopt_hosted(
        &self,
        who: &Who,
        config: &SyncConfig,
        new: &MasterPassword,
        typed: &SecretKey,
        code: Option<&str>,
        epoch: u64,
    ) -> IpcResult<(Option<QuickUnlockEnrollment>, bool)> {
        let ctx = self.hosted_ctx()?;
        self.sync.stop();
        let mut access = self.sync.session()?;
        let hosted = HostedRekey {
            base: &ctx.url,
            address: &ctx.email,
            device_name: &config.device_name,
            totp_code: code,
        };
        match adopt_hosted_rekey(
            &mut access,
            &hosted,
            new,
            typed,
            &who.current_key,
            who.install,
        ) {
            Ok(adopted) => {
                self.sync.check_epoch(epoch)?;
                self.sign_in_after_adoption(
                    who,
                    &ctx.url,
                    &ctx.email,
                    &config.device_name,
                    &adopted.opaque_password,
                );
                Ok((adopted.removed_quick_unlock, true))
            }
            // An equal epoch with the same account key: the Secret Key (or
            // password) was changed elsewhere, not a re-key.
            Err(RekeyError::Session(mosaic_vault::session::Error::UnlockFailed)) => {
                let changed =
                    self.recover_hosted(config, who.account, who.install, new, typed, code, epoch)?;
                Ok((changed.removed_quick_unlock, false))
            }
            Err(error) => Err(rekey_error(error)),
        }
    }

    fn adopt_folder(
        &self,
        who: &Who,
        config: &SyncConfig,
        new: &MasterPassword,
        typed: &SecretKey,
        epoch: u64,
    ) -> IpcResult<(Option<QuickUnlockEnrollment>, bool)> {
        let root = config
            .folder_path
            .as_deref()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        self.sync.stop();
        let mut access = self.sync.session()?;
        let transport = open_folder(
            self.sync.session()?,
            root,
            who.account,
            who.install,
            &config.device_name,
        )?;
        let mut engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Folder));
        match engine.adopt_folder_rekey(&mut access, &SystemClock, new, typed, &who.current_key) {
            Ok(adopted) => Ok((adopted.removed_quick_unlock, true)),
            Err(RekeyError::NoMatchingAccountFile) => {
                let changed = self
                    .recover_folder(config, who.account, who.install, new, typed, epoch)
                    .map_err(|e| {
                        if e.code == ErrorCode::SyncFolderNoAccount {
                            IpcError::new(ErrorCode::RekeyNoAccountFile)
                        } else {
                            e
                        }
                    })?;
                Ok((changed.removed_quick_unlock, false))
            }
            Err(error) => Err(rekey_error(error)),
        }
    }

    /// `sync_dismiss_account_file`: an account file this device cannot verify
    /// is remembered by name and SHA-256 and never reported again.
    ///
    /// # Errors
    /// `locked`, `invalidInput` (a name or digest that is not one).
    pub fn sync_dismiss_account_file(
        &self,
        request: &DismissFileRequest,
    ) -> IpcResult<SyncOverview> {
        let sha = parse_sha256(&request.sha256)?;
        if request.name.is_empty()
            || request.name.len() > 255
            || request.name.chars().any(char::is_control)
        {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        }
        self.with_unlocked(|session| Ok(session.dismiss_unverifiable_file(&request.name, &sha)?))?;
        self.sync.update_view(|view| {
            view.warnings.retain(|w| {
                !matches!(
                    w,
                    crate::contract::sync::SyncWarningView::UnverifiedAccountFile { sha256, .. }
                        if *sha256 == request.sha256
                )
            });
            true
        });
        self.emit_sync_state();
        self.sync.poke();
        self.sync_status()
    }
}

/// 64 hex digits to bytes.
fn parse_sha256(text: &str) -> IpcResult<[u8; 32]> {
    let bytes = text.as_bytes();
    if bytes.len() != 64 {
        return Err(IpcError::new(ErrorCode::InvalidInput));
    }
    let mut out = [0u8; 32];
    let (pairs, _) = bytes.as_chunks::<2>();
    for (slot, [high, low]) in out.iter_mut().zip(pairs) {
        let digit = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        };
        let (Some(high), Some(low)) = (digit(*high), digit(*low)) else {
            return Err(IpcError::new(ErrorCode::InvalidInput));
        };
        *slot = (high << 4) | low;
    }
    Ok(out)
}

/// Lower-case hex of a SHA-256, for the warning the webview shows.
pub(super) fn hex(bytes: &[u8; 32]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}
