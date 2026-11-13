//! `vault_rotate_key` (M7-R2): a new key for one vault, with the optional
//! re-seal of its attachments under fresh IDs and file keys, on the rotation
//! journal as a vault-only job (CRYPTO_SPEC §7.1). The job survives a crash
//! or a lock: `rotation_resume` continues it ([`Backend::resume_vault_job`]).
//!
//! The key is offered to the backend first and committed locally only when the
//! backend accepted it (engine). The re-seal runs only when this device's key
//! won. Re-authentication is the proof `reauthenticate` leaves (or the password
//! itself), like the other sensitive commands.

use mosaic_core::VaultId;
use mosaic_sync::clock::SystemClock;
use mosaic_sync::engine::{EngineConfig, RekeyError, RekeyProgress, SyncEngine, SyncError};
use mosaic_sync::http::HttpTransport;
use mosaic_sync::transport::{Transport, TransportKind};
use mosaic_vault::session::{RotationKind as VaultKind, RotationPhase, SyncKind};

use super::{MAX_CONTINUE_CALLS, Renewed, Who, rekey_error};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::rotation::{RotationDone, VaultRotateKeyRequest};
use crate::contract::session::ReauthPurpose;
use crate::state::Backend;
use crate::sync::access::BackendSession;
use crate::sync::open_folder;

/// What to run on the engine: a rotation, or the resume of an open job.
enum Work {
    Rotate {
        vault: VaultId,
        reseal_attachments: bool,
    },
    Resume,
}

impl Backend {
    /// `vault_rotate_key`.
    ///
    /// # Errors
    /// `locked`, `reauthRequired`, `wrongCredentials`, `vaultNotFound`,
    /// `vaultUnavailable`, `rotationInProgress` (a rotation job is open; a
    /// phase-2 job of this vault, a key offer whose outcome is not known, is
    /// settled by the call instead, M9-F1),
    /// `syncNotConfigured` (a vault key is offered to a backend; a device
    /// without sync has none), `sharingRoleForbidden` (a shared vault rotates
    /// from its owner's device) and the transport's `sync*` codes.
    pub fn vault_rotate_key(&self, request: VaultRotateKeyRequest) -> IpcResult<RotationDone> {
        self.touch();
        // The proof is taken before anything runs and spent whatever the
        // outcome (M9-F5): of two parallel calls one gets it, and a failed
        // rotation, which may have offered a key, does not leave it behind.
        match request.proof {
            Some(password) if !password.expose_secret().is_empty() => {
                self.reauthenticate(password, ReauthPurpose::VaultKeyRotation)?;
                let _ = self
                    .quick_unlock
                    .take_reauth(ReauthPurpose::VaultKeyRotation);
            }
            _ => {
                let _ = self.take_reauth_grant(ReauthPurpose::VaultKeyRotation)?;
            }
        }
        let who = self.who()?;
        let _change = self.sync.begin_change()?;
        let result = self.run_vault_work(
            &who,
            &Work::Rotate {
                vault: request.vault_id.get(),
                reseal_attachments: request.reseal_attachments,
            },
        );
        self.sync_restart();
        result.map(|finished| self.rotation_done(&no_renewal(), finished))
    }

    /// The resume of an open vault-only job (`rotation_resume`): re-seals what
    /// is left, uploads and closes the job.
    pub(super) fn resume_vault_job(&self, who: &Who) -> IpcResult<RotationDone> {
        let finished = self.run_vault_work(who, &Work::Resume)?;
        Ok(self.rotation_done(&no_renewal(), finished))
    }

    /// Builds the engine over the chosen backend (the sync thread stopped, the
    /// session reached one closure at a time) and runs `work`.
    fn run_vault_work(&self, who: &Who, work: &Work) -> IpcResult<bool> {
        let config = who
            .config
            .clone()
            .ok_or(IpcError::new(ErrorCode::SyncNotConfigured))?;
        self.sync.stop();
        let mut access = self.sync.session()?;
        match config.kind {
            SyncKind::Hosted => {
                let ctx = self.hosted_ctx()?;
                let token = self.stored_token(who.account)?;
                let transport = HttpTransport::connect(&ctx.url, &token, who.install)?;
                let mut engine =
                    SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
                self.vault_steps(&mut engine, &mut access, work)
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
                self.vault_steps(&mut engine, &mut access, work)
            }
        }
    }

    /// The rotation (or the resume), then `vault_rotate_continue` while the
    /// download budget of a call ran out. A failure after the key was
    /// committed is not an error: the journal job resumes on the next call.
    fn vault_steps<T: Transport>(
        &self,
        engine: &mut SyncEngine<T>,
        access: &mut BackendSession,
        work: &Work,
    ) -> IpcResult<bool> {
        // A job that was open before is not this call's: its errors stay errors.
        let job_before = self.vault_job_committed();
        let first = match work {
            Work::Rotate {
                vault,
                reseal_attachments,
            } => engine
                .rotate_vault_key(access, &SystemClock, *vault, *reseal_attachments)
                .map(|done| done.progress)
                .map_err(after_rotation_error),
            Work::Resume => engine
                .vault_rotate_continue(access, &SystemClock)
                .map(Some)
                .map_err(after_rotation_error),
        };
        let progress = match first {
            Ok(progress) => progress,
            Err(error)
                if !job_before && self.vault_job_committed() && error.code != ErrorCode::Locked =>
            {
                // The key moved on; the re-seal waits for "Continue".
                self.emit_rotation_progress();
                return Ok(false);
            }
            Err(error) => return Err(error),
        };
        self.emit_rotation_progress();
        let Some(mut progress) = progress else {
            // No re-seal was asked for, or another device's key won.
            return Ok(true);
        };
        for _ in 0..MAX_CONTINUE_CALLS {
            if progress.finished {
                return Ok(true);
            }
            if !progress.more_attachments {
                return Ok(false);
            }
            progress = match continue_once(engine, access) {
                Ok(next) => next,
                Err(error) if error.code == ErrorCode::Locked => return Err(error),
                Err(_) => return Ok(false),
            };
            self.emit_rotation_progress();
        }
        Ok(false)
    }

    /// Whether a vault-only job is open past the key commit (the re-seal is
    /// not done). A job still at phase 2 (the key offered, the outcome not
    /// known, M9-F1) is not: an error then stays an error, and the job waits
    /// for `rotation_resume` or a retried `vault_rotate_key`, which settle it.
    fn vault_job_committed(&self) -> bool {
        self.slot().is_ok_and(|slot| {
            slot.unlocked()
                .ok()
                .and_then(|s| s.rotation_job().ok().flatten())
                .is_some_and(|job| {
                    job.kind == VaultKind::Vault && job.phase >= RotationPhase::LocalCommitted
                })
        })
    }
}

fn continue_once<T: Transport>(
    engine: &mut SyncEngine<T>,
    access: &mut BackendSession,
) -> IpcResult<RekeyProgress> {
    engine
        .vault_rotate_continue(access, &SystemClock)
        .map_err(after_rotation_error)
}

/// The IPC error of a failed step: a lock stays a lock.
fn after_rotation_error(error: RekeyError) -> IpcError {
    match error {
        RekeyError::Sync(SyncError::Locked) => IpcError::LOCKED,
        other => rekey_error(other),
    }
}

const fn no_renewal() -> Renewed {
    Renewed {
        quick_unlock: false,
        hardware_keys: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vault_rotation_renews_no_account_credentials() {
        let none = no_renewal();
        assert!(!none.quick_unlock);
        assert_eq!(none.hardware_keys, 0);
    }
}
