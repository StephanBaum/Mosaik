//! The sync engine (ADR-0045, M6 spec §5): one protocol over any
//! [`Transport`]. [`SyncEngine::sync_once`] runs the steps in a fixed order:
//! vault uploads, checkpoints, pull, push, metadata, attachments, the
//! device's own checkpoint, backend housekeeping and the report.
//!
//! The engine never holds the session lock while it calls the transport:
//! every touch of the session is one short [`SessionAccess::with`] step.
//! Nothing secret appears in an error or a report.

mod access;
mod attachments;
mod checkpoints;
pub mod consts;
mod keys;
mod maintain;
mod pull;
mod push;
mod rotation;

use core::time::Duration;
use std::collections::BTreeMap;

use mosaic_core::{InstallId, KeyGeneration, VaultId};

pub use access::{Backoff, Jitter, SessionAccess, SystemJitter};

use self::consts::{
    BACKOFF_INITIAL_MS, BACKOFF_MAX_MS, FREEZE_GRACE_FOLDER_MS, FREEZE_GRACE_HOSTED_MS,
};
use crate::clock::Clock;
use crate::transport::{ChangeToken, Transport, TransportError, TransportKind};
use crate::wire::checkpoint::Checkpoint;

/// What the engine needs to know besides the transport.
#[derive(Clone, Copy, Debug)]
pub struct EngineConfig {
    /// The kind of backend (decides the lag rule and the freeze grace).
    pub transport_kind: TransportKind,
    /// How long another device may lag before a warning (ms).
    pub freeze_grace_ms: u64,
}

impl EngineConfig {
    /// The defaults of a backend kind.
    #[must_use]
    pub const fn for_kind(kind: TransportKind) -> Self {
        Self {
            transport_kind: kind,
            freeze_grace_ms: match kind {
                TransportKind::Hosted => FREEZE_GRACE_HOSTED_MS,
                TransportKind::Folder => FREEZE_GRACE_FOLDER_MS,
            },
        }
    }
}

/// What a rotation ended with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationOutcome {
    /// This device's key won; the generation is the new one.
    Rotated(KeyGeneration),
    /// Another device rotated first; this device took its key.
    RotatedByAnother(KeyGeneration),
}

/// Something worth telling the user.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SyncWarning {
    /// A device's checkpoint is ahead of what the backend shows of it for
    /// longer than the grace (hosted): a server withholding data.
    FreezeSuspected {
        /// The device.
        device: InstallId,
        /// Since when (ms since the Unix epoch).
        since_ms: u64,
    },
    /// The same for a folder: files of a device are missing.
    MissingFiles {
        /// The device.
        device: InstallId,
        /// Since when (ms since the Unix epoch).
        since_ms: u64,
    },
    /// This device's own checkpoint went backwards.
    RollbackDetected,
    /// Another install writes under this install's ID.
    IdentityDuplicated,
    /// Records could not be decrypted for no known reason.
    Undecryptable {
        /// How many.
        count: u32,
    },
    /// Hardware-protected vaults exist; they stay on this device.
    HardwareVaultsLocal {
        /// How many.
        count: u32,
    },
}

/// What one sync run did.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Records received and stored or merged.
    pub pulled: u32,
    /// Records uploaded.
    pub pushed: u32,
    /// Records merged with local changes.
    pub merged: u32,
    /// Conflict copies created by oversized merges.
    pub split_copies: u32,
    /// Writers that share a device ID.
    pub stamp_collisions: u32,
    /// Fields written back after a delete raced an edit.
    pub restored_fields: u32,
    /// Unreadable local records replaced by the received version.
    pub replaced_unreadable: u32,
    /// Received objects that did not open.
    pub undecryptable: u32,
    /// Received objects below the accepted key generation, dropped.
    pub rollback_dropped: u32,
    /// Attachments uploaded.
    pub attachments_up: u32,
    /// Attachments downloaded.
    pub attachments_down: u32,
    /// Vaults whose content changed.
    pub changed_vaults: Vec<VaultId>,
    /// Warnings.
    pub warnings: Vec<SyncWarning>,
}

/// Why a sync run failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SyncError {
    /// The session is locked.
    #[error("session locked")]
    Locked,
    /// The backend failed.
    #[error("transport error")]
    Transport(#[from] TransportError),
    /// The session failed.
    #[error("session error")]
    Session(#[from] mosaic_vault::session::Error),
    /// This device's own checkpoint went backwards on the backend twice in
    /// a row: the backend was rolled back, or a backup restored.
    #[error("rollback detected")]
    RollbackDetected,
    /// A verified checkpoint for this install is ahead of this device's
    /// counter: a copy of this profile is syncing under the same install ID.
    #[error("install identity duplicated")]
    IdentityDuplicated,
    /// A transient failure; the next attempt is at `retry_at_ms`.
    #[error("backing off")]
    Backoff {
        /// When the next attempt may start (ms since the Unix epoch).
        retry_at_ms: u64,
    },
}

/// What the engine remembers about vault keys between pulls (the
/// canonical-winner rule, M6 spec §5.5).
#[derive(Debug, Default)]
pub(crate) struct KeyBook {
    /// Candidates that conflicted with the key held for a generation.
    pub(crate) conflicts:
        BTreeMap<(VaultId, KeyGeneration), Vec<(InstallId, mosaic_core::WrappedKey)>>,
    /// Which device delivered the key held for a generation.
    pub(crate) delivered: BTreeMap<(VaultId, KeyGeneration), InstallId>,
}

/// The engine. See the [module documentation](self).
pub struct SyncEngine<T: Transport> {
    pub(crate) transport: T,
    pub(crate) config: EngineConfig,
    /// Since when another device lags in a vault.
    pub(crate) lag: BTreeMap<(InstallId, VaultId), u64>,
    pub(crate) backoff: Backoff,
    pub(crate) jitter: Box<dyn Jitter>,
    pub(crate) keys: KeyBook,
    /// The latest verified checkpoint per other device.
    pub(crate) others: BTreeMap<InstallId, Checkpoint>,
    pub(crate) strikes: u32,
    pub(crate) strike_at: Option<u64>,
    pub(crate) halted: Option<Halt>,
    change_token: Option<ChangeToken>,
}

/// Why automatic sync stopped until the user acts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Halt {
    /// [`SyncError::RollbackDetected`].
    Rollback,
    /// [`SyncError::IdentityDuplicated`].
    Identity,
}

impl<T: Transport> SyncEngine<T> {
    /// An engine over `transport` with the system jitter.
    #[must_use]
    pub fn new(transport: T, config: EngineConfig) -> Self {
        Self::with_jitter(transport, config, Box::new(SystemJitter))
    }

    /// An engine with a chosen jitter source (tests).
    #[must_use]
    pub fn with_jitter(transport: T, config: EngineConfig, jitter: Box<dyn Jitter>) -> Self {
        Self {
            transport,
            config,
            lag: BTreeMap::new(),
            backoff: Backoff::default(),
            jitter,
            keys: KeyBook::default(),
            others: BTreeMap::new(),
            strikes: 0,
            strike_at: None,
            halted: None,
            change_token: None,
        }
    }

    /// The transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// When the next attempt may start, while the engine backs off.
    #[must_use]
    pub const fn next_retry_at(&self) -> Option<u64> {
        self.backoff.retry_at
    }

    /// Why automatic sync is stopped, if it is.
    #[must_use]
    pub const fn halted(&self) -> Option<Halt> {
        self.halted
    }

    /// The user acknowledged a halt (the desktop asks first, §7).
    pub fn clear_halt(&mut self) {
        self.halted = None;
        self.strikes = 0;
        self.strike_at = None;
    }

    /// Waits until the backend changes or `timeout` passes.
    ///
    /// # Errors
    /// [`SyncError::Transport`].
    pub fn wait_for_change(&mut self, timeout: Duration) -> Result<(), SyncError> {
        let token = self
            .transport
            .wait_for_change(self.change_token.as_ref(), timeout)?;
        self.change_token = Some(token);
        Ok(())
    }

    /// One sync run (see the [module documentation](self)).
    ///
    /// # Errors
    /// [`SyncError::Backoff`] while backing off; [`SyncError::Locked`];
    /// [`SyncError::RollbackDetected`] and [`SyncError::IdentityDuplicated`]
    /// (the engine then stays halted until [`SyncEngine::clear_halt`]);
    /// [`SyncError::Transport`] and [`SyncError::Session`].
    pub fn sync_once(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<SyncReport, SyncError> {
        let now = clock.now_ms();
        match self.halted {
            Some(Halt::Rollback) => return Err(SyncError::RollbackDetected),
            Some(Halt::Identity) => return Err(SyncError::IdentityDuplicated),
            None => {}
        }
        if let Some(retry_at_ms) = self.backoff.retry_at
            && now < retry_at_ms
        {
            return Err(SyncError::Backoff { retry_at_ms });
        }
        let result = self.run(access, clock);
        match &result {
            Ok(_) => self.backoff.reset(),
            Err(SyncError::Transport(e)) => {
                let floor = match e {
                    TransportError::RateLimited { retry_after_s } => *retry_after_s,
                    _ => 0,
                };
                if matches!(
                    e,
                    TransportError::Unreachable
                        | TransportError::Timeout
                        | TransportError::RateLimited { .. }
                        | TransportError::Io
                        | TransportError::FolderUnavailable
                ) {
                    let permille = self.jitter.permille();
                    self.backoff
                        .fail(now, floor, permille, BACKOFF_INITIAL_MS, BACKOFF_MAX_MS);
                }
            }
            Err(SyncError::RollbackDetected) => self.halted = Some(Halt::Rollback),
            Err(SyncError::IdentityDuplicated) => self.halted = Some(Halt::Identity),
            Err(_) => {}
        }
        result
    }

    fn run(
        &mut self,
        access: &mut impl SessionAccess,
        clock: &impl Clock,
    ) -> Result<SyncReport, SyncError> {
        let mut report = SyncReport::default();
        let mut stats = pull::RunStats::default();
        let now = clock.now_ms();
        self.upload_vaults(access, &mut report)?;
        self.verify_checkpoints(access)?;
        self.pull_phase(access, &mut report, &mut stats)?;
        self.push_phase(access, &mut report, &mut stats, None)?;
        self.push_metas(access, &mut stats, &mut report)?;
        self.attachments_phase(access, &mut report)?;
        let warnings = self.evaluate_lag(access, now)?;
        self.publish_checkpoint(access, clock)?;
        self.maintain(access, now)?;
        Self::finish_report(access, &mut report, &stats, warnings)?;
        Ok(report)
    }
}
