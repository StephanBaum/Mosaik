//! The sync engine (ADR-0045, M6 spec §5): one protocol over any
//! [`Transport`]. [`SyncEngine::sync_once`] runs the steps in a fixed order:
//! vault uploads, checkpoints, pull, push, metadata, attachments, the
//! device's own checkpoint, backend housekeeping and the report.
//!
//! The engine never holds the session lock while it calls the transport:
//! every touch of the session is one short [`SessionAccess::with`] step.
//! Nothing secret appears in an error or a report.
//!
//! What the engine must remember between runs lives in the vault store (the
//! halt, the checkpoint strikes, the newest checkpoint of every other
//! device, the lag timers, the vault keys seen per generation, the first
//! time a blob delete marker was seen): the engine and its transport are
//! rebuilt whenever the app locks or restarts, and an attacker who can make
//! the app restart must not be able to reset a timer or a strike.

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
use std::collections::BTreeSet;

use mosaic_core::{AttachmentId, InstallId, KeyGeneration, VaultId};
use mosaic_vault::session::UnlockedSession;
use mosaic_vault::storage::SyncHalt;

pub use access::{Backoff, Jitter, SessionAccess, SystemJitter};

use self::consts::{
    BACKOFF_INITIAL_MS, BACKOFF_MAX_MS, FREEZE_GRACE_FOLDER_MS, FREEZE_GRACE_HOSTED_MS,
};
use crate::clock::Clock;
use crate::transport::{ChangeToken, Transport, TransportError, TransportKind};

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
    /// Downloaded attachment files that did not authenticate under the key
    /// of the item that refers to them (they stay missing).
    pub attachments_rejected: u32,
    /// Attachment files on the backend that did not authenticate and were
    /// replaced by this device's authentic copy.
    pub attachments_healed: u32,
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
    /// The backend failed. A locked session is [`SyncError::Locked`], not
    /// this.
    #[error("transport error")]
    Transport(TransportError),
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

impl From<TransportError> for SyncError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Locked => Self::Locked,
            other => Self::Transport(other),
        }
    }
}

/// The engine. See the [module documentation](self).
pub struct SyncEngine<T: Transport> {
    pub(crate) transport: T,
    pub(crate) config: EngineConfig,
    pub(crate) backoff: Backoff,
    pub(crate) jitter: Box<dyn Jitter>,
    /// Attachment files already checked against the item's key in this
    /// engine's lifetime (the holder re-checks a blob once, see
    /// `attachments`).
    pub(crate) verified_blobs: BTreeSet<AttachmentId>,
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

impl From<SyncHalt> for Halt {
    fn from(h: SyncHalt) -> Self {
        match h {
            SyncHalt::Rollback => Self::Rollback,
            SyncHalt::Identity => Self::Identity,
        }
    }
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
            backoff: Backoff::default(),
            jitter,
            verified_blobs: BTreeSet::new(),
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

    /// Why automatic sync is stopped, if it is (persisted in the store: it
    /// survives a lock and a restart).
    ///
    /// # Errors
    /// [`SyncError::Locked`], [`SyncError::Session`].
    pub fn halted(&self, access: &mut impl SessionAccess) -> Result<Option<Halt>, SyncError> {
        Ok(access
            .with(|s| s.sync_engine_state())??
            .halt
            .map(Halt::from))
    }

    /// The user acknowledged a halt (the desktop asks first, §7): the
    /// persisted halt and strikes are cleared. After a rollback the own logs
    /// count as unchecked until written again; sequence numbers already used
    /// are never reused.
    ///
    /// # Errors
    /// [`SyncError::Locked`], [`SyncError::Session`].
    pub fn clear_halt(&mut self, access: &mut impl SessionAccess) -> Result<(), SyncError> {
        access.with(UnlockedSession::clear_sync_halt)??;
        self.backoff.reset();
        Ok(())
    }

    /// Persists a halt and returns the error that reports it.
    fn raise(access: &mut impl SessionAccess, halt: SyncHalt) -> SyncError {
        match access.with(|s| s.set_sync_halt(halt)) {
            Ok(Ok(())) => match halt {
                SyncHalt::Rollback => SyncError::RollbackDetected,
                SyncHalt::Identity => SyncError::IdentityDuplicated,
            },
            Ok(Err(e)) => SyncError::Session(e),
            Err(e) => e,
        }
    }

    /// Stops with a rollback halt if the own log of a synced vault lost
    /// segments this install wrote.
    pub(crate) fn check_own_logs(
        &mut self,
        access: &mut impl SessionAccess,
    ) -> Result<(), SyncError> {
        let vaults = access.with(|s| s.synced_vaults())?;
        if self.transport.own_logs_rolled_back(&vaults)? {
            return Err(Self::raise(access, SyncHalt::Rollback));
        }
        Ok(())
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
        match access.with(|s| s.sync_engine_state())??.halt {
            Some(SyncHalt::Rollback) => return Err(SyncError::RollbackDetected),
            Some(SyncHalt::Identity) => return Err(SyncError::IdentityDuplicated),
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
        // Before anything is written to the backend.
        self.check_own_logs(access)?;
        self.upload_vaults(access, &mut report)?;
        self.verify_checkpoints(access, now)?;
        self.pull_phase(access, &mut report, &mut stats)?;
        self.push_phase(access, &mut report, &mut stats, None)?;
        self.push_metas(access, &mut stats, &mut report)?;
        self.attachments_phase(access, &mut report)?;
        let (warnings, none_lagging) = self.evaluate_lag(access, now)?;
        self.publish_checkpoint(access, clock)?;
        let no_conflict = !access.with(|s| s.has_any_unresolved_key_conflict())??;
        let caught_up = none_lagging && no_conflict && stats.unexplained == 0;
        self.maintain(access, now, caught_up)?;
        Self::finish_report(access, &mut report, &stats, warnings)?;
        Ok(report)
    }
}
