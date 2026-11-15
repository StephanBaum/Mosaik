//! Hosted sync, password recovery and the device reset (CRYPTO_SPEC §9,
//! ADR-0091). The transport is `mosaic_sync::http::HttpTransport`; the engine
//! runs on the unlocked session's holder, so a `lock()` ends a running sync with
//! `locked` and never deadlocks.

use std::path::{Path, PathBuf};

use mosaic_core::MasterPassword;
use mosaic_sync::engine::{EngineConfig, SyncEngine, SyncError, SyncWarning as EngineWarning};
use mosaic_sync::http::{HttpTransport, StoredToken, Token as _, hosted};
use mosaic_sync::transport::{Rejection, TransportError, TransportKind};
use mosaic_vault::session::{SharingFinding, UnlockedSession};
use mosaic_vault::storage::{SyncHalt, SyncKind, VaultFlag};
use zeroize::Zeroizing;

use crate::Core;
use crate::error::{MobileError, Result};
use crate::quick_unlock::params::Params;
use crate::secret::text_of;
use crate::state::{Phase, SessionState, Slots};
use crate::storage::{read, secret_key_name, sync_token_name};
use crate::unlock::Taken;

/// Where sync stands, for the settings screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncOverview {
    /// The account syncs with a server.
    pub configured: bool,
    /// A session token is stored.
    pub signed_in: bool,
    /// The server's origin.
    pub server_url: Option<String>,
    /// Local changes not yet uploaded (0 while locked).
    pub pending_uploads: u32,
    /// The last successful run (Unix ms), since this process started.
    pub last_synced_at_ms: Option<u64>,
    /// How the last run failed.
    pub last_error: Option<MobileError>,
    /// The engine halted (a rollback or a duplicated identity) or a vault
    /// carries a finding: the apps tell the user.
    pub halted: bool,
    /// What sync found wrong, for the status screen: the stored findings
    /// plus the run warnings not yet acknowledged (empty while locked).
    pub warnings: Vec<SyncWarning>,
}

/// The kind of a sync warning (the apps pick the text; `sync.warning.*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SyncWarningKind {
    /// A device's checkpoint is ahead of what the server shows of it.
    FreezeSuspected,
    /// The heads of this phone and a peer never met for a long time.
    NoCommonView,
    /// This device's own checkpoint went backwards.
    RollbackDetected,
    /// Another install writes under this install's ID.
    IdentityDuplicated,
    /// The server showed contradicting views of a vault.
    ServerInconsistent,
    /// The server's head for a vault is below what this phone read.
    ServerRollback,
    /// Records could not be read.
    Undecryptable,
    /// The account key epoch on the server is below this phone's.
    AccountRollback,
    /// A shared vault needs attention: a revocation rotation is waiting or
    /// blocked, or sharing recorded a finding. The phone has no sharing
    /// screen; the text sends the user to the computer.
    SharingAttention,
}

/// One sync warning. It carries no secret and no name: a kind, since when
/// (where the engine knows) and a count (where it counts).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyncWarning {
    /// What it is.
    pub kind: SyncWarningKind,
    /// Since when (Unix ms).
    pub since_ms: Option<u64>,
    /// How many (undecryptable records).
    pub count: Option<u32>,
}

impl SyncWarning {
    const fn plain(kind: SyncWarningKind) -> Self {
        Self {
            kind,
            since_ms: None,
            count: None,
        }
    }

    /// The warnings of a run that the phone shows. Kinds the phone has no
    /// screen for (folders, shared vaults, hardware vaults) are left out.
    fn from_engine(warning: &EngineWarning) -> Option<Self> {
        Some(match warning {
            EngineWarning::FreezeSuspected { since_ms, .. } => Self {
                since_ms: Some(*since_ms),
                ..Self::plain(SyncWarningKind::FreezeSuspected)
            },
            EngineWarning::NoCommonView { since_ms, .. } => Self {
                since_ms: Some(*since_ms),
                ..Self::plain(SyncWarningKind::NoCommonView)
            },
            EngineWarning::RollbackDetected => Self::plain(SyncWarningKind::RollbackDetected),
            EngineWarning::IdentityDuplicated => Self::plain(SyncWarningKind::IdentityDuplicated),
            EngineWarning::ServerInconsistent { .. } => {
                Self::plain(SyncWarningKind::ServerInconsistent)
            }
            EngineWarning::ServerRollback { .. } => Self::plain(SyncWarningKind::ServerRollback),
            EngineWarning::Undecryptable { count } => Self {
                count: Some(*count),
                ..Self::plain(SyncWarningKind::Undecryptable)
            },
            EngineWarning::AccountRollback { .. } => Self::plain(SyncWarningKind::AccountRollback),
            // A key change the owner's device could not make (CRYPTO_SPEC
            // §8.3): never dropped on the phone (SYNC-3).
            EngineWarning::RotationBlocked { .. } => Self::plain(SyncWarningKind::SharingAttention),
            _ => return None,
        })
    }
}

/// Adds `new` to `into`, one entry per kind: a kind that is already there is
/// replaced by the new entry (the latest count and `since` win), so the list
/// cannot grow with every run.
fn merge_warnings(into: &mut Vec<SyncWarning>, new: impl IntoIterator<Item = SyncWarning>) {
    for warning in new {
        match into.iter_mut().find(|w| w.kind == warning.kind) {
            Some(slot) => *slot = warning,
            None => into.push(warning),
        }
    }
}

/// The warnings the stored findings stand for. `sharing` says whether sharing
/// findings are stored: the phone shows them as one generic warning.
fn finding_warnings(
    findings: &mosaic_vault::session::SyncFindings,
    sharing: bool,
) -> Vec<SyncWarning> {
    let mut out = Vec::new();
    match findings.halt {
        Some(SyncHalt::Rollback) => out.push(SyncWarning::plain(SyncWarningKind::RollbackDetected)),
        Some(SyncHalt::Identity) => {
            out.push(SyncWarning::plain(SyncWarningKind::IdentityDuplicated));
        }
        None => {}
    }
    for (_, flag) in &findings.vault_flags {
        out.push(SyncWarning::plain(match flag {
            VaultFlag::ServerInconsistent => SyncWarningKind::ServerInconsistent,
            VaultFlag::ServerRolledBack => SyncWarningKind::ServerRollback,
        }));
    }
    for (_, _, since_ms) in &findings.no_common_view {
        out.push(SyncWarning {
            since_ms: Some(*since_ms),
            ..SyncWarning::plain(SyncWarningKind::NoCommonView)
        });
    }
    if sharing {
        out.push(SyncWarning::plain(SyncWarningKind::SharingAttention));
    }
    out
}

/// Clears the halt and the vault findings and, of the sharing findings, only
/// what the phone showed (`shown`): one recorded since the last overview stays
/// and is shown next. One session step, so a sync cannot add a finding between
/// the check and the clear (SYNC-3).
fn clear_findings(s: &mut UnlockedSession, shown: &[SharingFinding]) -> Result<()> {
    let findings = s.sync_findings()?;
    if findings.halt.is_some() {
        s.clear_sync_halt()?;
    }
    let vaults = findings.vaults();
    if !vaults.is_empty() {
        s.clear_vault_findings(&vaults)?;
    }
    if s.sharing_findings()?.iter().all(|f| shown.contains(f)) {
        s.clear_sharing_findings()?;
    }
    Ok(())
}

/// The confirmation of a device reset: the account email, typed.
pub struct ResetConfirm {
    /// The email of the account, as the user types it.
    pub typed_email: String,
}

/// What a reset did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResetOutcome {
    /// The state afterwards (`NoAccount`).
    pub state: SessionState,
    /// The platform keys to delete (the quick-unlock key).
    pub key_aliases: Vec<String>,
}

/// The files of a store: the database, its lock, WAL and journal files, and
/// the attachment directory.
fn store_files(store: &Path) -> Vec<PathBuf> {
    let mut files = vec![store.to_path_buf()];
    for suffix in ["-lock", "-wal", "-shm", "-journal", "-attachments"] {
        let mut name = store.as_os_str().to_owned();
        name.push(suffix);
        files.push(PathBuf::from(name));
    }
    files
}

fn remove_all(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Maps a sync failure and says whether it ends the session token.
fn token_ended(e: &TransportError) -> bool {
    matches!(
        e,
        TransportError::Unauthorized
            | TransportError::Revoked(_)
            | TransportError::Rejected(Rejection::DeviceRevoked)
    )
}

impl Core {
    /// Removes the store files and the settings file of this install, after
    /// the store is closed.
    pub(crate) fn wipe_files(&self) -> Result<()> {
        for path in store_files(&self.config.store_path()) {
            remove_all(&path).map_err(|_| MobileError::StoreUnavailable)?;
        }
        let settings = self.config.settings_path();
        remove_all(&settings).map_err(|_| MobileError::StoreUnavailable)?;
        remove_all(&settings.with_extension("json.tmp")).map_err(|_| MobileError::StoreUnavailable)
    }

    /// Deletes the session token after the server ended it, and remembers that
    /// the phone is signed out.
    fn drop_token(&self, token_name: &str) {
        let _ = self.storage.delete(token_name);
        self.note_stored(None, Some(false));
    }

    /// Where sync stands. Total: a failing read gives the plainest answer.
    #[must_use]
    pub fn sync_overview(&self) -> SyncOverview {
        let mut overview = SyncOverview {
            configured: false,
            signed_in: false,
            server_url: None,
            pending_uploads: 0,
            last_synced_at_ms: None,
            last_error: None,
            halted: false,
            warnings: Vec::new(),
        };
        if self.account_id().is_err() {
            return overview;
        }
        let (config, last_sync, last_error, unlocked, signed_in, run_warnings) = {
            let Ok(guard) = self.guard() else {
                return overview;
            };
            let config = match &guard.phase {
                Phase::Locked(ls) => ls.sync_config().ok().flatten(),
                _ => None,
            };
            (
                config,
                guard.last_sync_at_ms,
                guard.last_sync_error,
                matches!(guard.phase, Phase::Unlocked(_)),
                guard.signed_in,
                guard.sync_warnings.clone(),
            )
        };
        overview.last_synced_at_ms = last_sync;
        overview.last_error = last_error;
        let mut config = config;
        if unlocked {
            let facts = self.with_session(|s| {
                let findings = s.sync_findings()?;
                let sharing = s.sharing_findings()?;
                let pending = s.pending_uploads(1000)?.len();
                Ok((
                    s.sync_config()?,
                    !findings.is_clear() || !sharing.is_empty(),
                    pending,
                    finding_warnings(&findings, !sharing.is_empty()),
                    sharing,
                ))
            });
            if let Ok((c, halted, pending, stored, sharing)) = facts {
                // What this overview shows: an acknowledgement clears only
                // these sharing findings (SYNC-3).
                if let Ok(mut guard) = self.guard() {
                    guard.shown_sharing = sharing;
                }
                config = c;
                overview.halted = halted;
                let mut warnings = stored;
                merge_warnings(&mut warnings, run_warnings);
                overview.warnings = warnings;
                overview.pending_uploads = u32::try_from(pending).unwrap_or(u32::MAX);
            }
        }
        if let Some(config) = config {
            overview.configured = config.kind == SyncKind::Hosted;
            overview.server_url = config.server_url;
        }
        // Cached (G1-2): the token is not read from secure storage to tell
        // whether it is there.
        overview.signed_in = signed_in;
        overview
    }

    /// One sync run: upload local changes, download remote ones.
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `locked`,
    /// `syncNotConfigured`, `folderAccountUnsupported`, `syncSignedOut`,
    /// `syncPasswordChanged`, `syncAccountRekeyed`, `syncRateLimited`,
    /// `syncServerUnreachable` and the other sync codes.
    pub fn sync_now(&self) -> Result<SyncOverview> {
        self.require_app()?;
        let result = self.sync_once();
        if let Ok(mut guard) = self.guard() {
            match &result {
                Ok(warnings) => {
                    guard.last_sync_at_ms = Some(self.now_ms());
                    guard.last_sync_error = None;
                    merge_warnings(&mut guard.sync_warnings, warnings.iter().cloned());
                }
                Err(e) => guard.last_sync_error = Some(*e),
            }
        }
        result?;
        Ok(self.sync_overview())
    }

    fn sync_once(&self) -> Result<Vec<SyncWarning>> {
        use mosaic_sync::wire::SessionHandle as _;
        let owned = self.owned_session()?;
        let (config, install, account) = owned
            .with_session(|s| (s.sync_config(), s.install_id(), s.account_id()))
            .ok_or(MobileError::Locked)?;
        let config = config?.ok_or(MobileError::SyncNotConfigured)?;
        if config.kind != SyncKind::Hosted {
            return Err(MobileError::FolderAccountUnsupported);
        }
        let url = config.server_url.ok_or(MobileError::SyncNotConfigured)?;
        let token_name = sync_token_name(&account);
        let Some(stored) = read(self.storage.as_ref(), &token_name)? else {
            // The token is not there: the cached flag was stale (N5).
            self.note_stored(None, Some(false));
            return Err(MobileError::SyncSignedOut);
        };
        let mut bytes: [u8; 32] = stored
            .as_slice()
            .try_into()
            .map_err(|_| MobileError::StorageUnavailable)?;
        let token = StoredToken::from_array(&mut bytes);
        let transport = HttpTransport::connect(&url, &token, install).map_err(|e| {
            if token_ended(&e) {
                self.drop_token(&token_name);
            }
            MobileError::from(e)
        })?;
        let mut engine = SyncEngine::new(transport, EngineConfig::for_kind(TransportKind::Hosted));
        let mut access = owned.clone();
        self.work_pause();
        match engine.sync_once(&mut access, &self.clock) {
            Ok(report) => Ok(report
                .warnings
                .iter()
                .filter_map(SyncWarning::from_engine)
                .collect()),
            Err(error) => {
                if let SyncError::Transport(t) = &error
                    && token_ended(t)
                {
                    self.drop_token(&token_name);
                }
                Err(error.into())
            }
        }
    }

    /// Acknowledges the halt and the findings against the server, as the
    /// desktop does (`sync_acknowledge`): the user has checked them. The halt
    /// and the vault findings are cleared (the vaults are read again and what
    /// the server lacks is queued again) and the remembered run warnings are
    /// dropped. The app starts a sync afterwards.
    ///
    /// The session must be unlocked; like the desktop it asks for no
    /// re-authentication (the app asks the user to confirm first).
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `locked`, `syncNotConfigured`,
    /// or the store's code.
    pub fn acknowledge_sync_findings(&self) -> Result<SyncOverview> {
        self.require_app()?;
        let shown = self.guard()?.shown_sharing.clone();
        self.with_session(|s| {
            if s.sync_config()?.is_none() {
                return Err(MobileError::SyncNotConfigured);
            }
            clear_findings(s, &shown)
        })?;
        // Dropped under the mutex, after the findings are cleared.
        {
            let mut guard = self.guard()?;
            guard.sync_warnings.clear();
            guard.shown_sharing.clear();
        }
        Ok(self.sync_overview())
    }

    /// Recovers from a master password changed on another device: the new
    /// password signs in to the server and must open this device's own
    /// account key. The quick-unlock enrolment and the boot marker are
    /// deleted (the platform reads the old key alias from the state).
    ///
    /// # Errors
    /// `invalidInput` in the extension context, `locked`, `emptyPassword`,
    /// `secretKeyRequired`, `syncNotConfigured`, `wrongCredentials`,
    /// `syncTwoFactorRequired` (an account with a second factor cannot recover
    /// on the phone), and the other sync codes.
    pub fn recover_password(&self, new_password: Vec<u8>) -> Result<SyncOverview> {
        self.require_app()?;
        let mut password = text_of(new_password)?;
        if password.is_empty() {
            return Err(MobileError::EmptyPassword);
        }
        let account = self.account_id()?;
        let secret_key = self.stored_secret_key(&account)?;
        let owned = self.owned_session()?;
        let (config, install) = {
            use mosaic_sync::wire::SessionHandle as _;
            owned
                .with_session(|s| (s.sync_config(), s.install_id()))
                .ok_or(MobileError::Locked)?
        };
        let config = config?.ok_or(MobileError::SyncNotConfigured)?;
        if config.kind != SyncKind::Hosted {
            return Err(MobileError::FolderAccountUnsupported);
        }
        let (url, email) = config
            .server_url
            .zip(config.email)
            .ok_or(MobileError::SyncNotConfigured)?;
        let master = MasterPassword::new(std::mem::take(&mut *password));
        let unlocked = self.take_unlocked()?;
        let recovered = hosted::recover_password(
            &url,
            &email,
            &master,
            &secret_key,
            install,
            &config.device_name,
            None,
        );
        let adopted = match recovered {
            Ok(recovered) => {
                let params = *recovered.keys.params();
                let wrapped = recovered.account.wrapped_ak.clone();
                let token = Zeroizing::new(*recovered.grant.token.expose_bytes());
                let changed = {
                    use mosaic_sync::wire::SessionHandle as _;
                    unlocked.owned.with_session_mut(|s| {
                        s.adopt_remote_password(recovered.keys, &wrapped, params)
                            .map_err(MobileError::from)
                    })
                };
                Ok((changed.unwrap_or(Err(MobileError::Locked)), token))
            }
            Err(e) => Err(MobileError::from(e)),
        };
        self.work_pause();
        // A `lock()` that came meanwhile wins the phase, but the adoption below
        // already happened: its results (the key alias to delete, the new
        // token) are recorded either way, so a lock cannot lose them (G1-13).
        let restored = self.restore_unlocked(unlocked);
        let (changed, token) = adopted?;
        let changed = changed?;
        if let Some(removed) = changed.removed_quick_unlock {
            let alias = Params::decode(&removed.platform_params, self.config.platform)
                .ok()
                .map(|p| p.key_alias());
            if let Ok(mut guard) = self.guard() {
                guard.expired_alias = alias;
            }
            self.update_info(|i| i.quick_unlock_enrolled = false);
        }
        self.storage
            .put(&sync_token_name(&account), token.as_slice())?;
        self.note_stored(None, Some(true));
        if let Ok(mut guard) = self.guard() {
            guard.last_sync_error = None;
        }
        restored?;
        Ok(self.sync_overview())
    }

    /// Wipes this phone's account: the store, the settings file and the stored
    /// secrets; the platform deletes the returned keys. The user confirms with
    /// the account email.
    ///
    /// # Errors
    /// `invalidInput` (the extension context, or the email does not match),
    /// `busy`, `storageUnavailable`, `storeUnavailable`.
    pub fn reset_device(&self, confirm: ResetConfirm) -> Result<ResetOutcome> {
        self.require_app()?;
        self.lock();
        let taken = self.take_locked()?;
        let Taken::Locked(locked) = taken else {
            self.put_back(taken);
            return Ok(ResetOutcome {
                state: self.state(),
                key_aliases: Vec::new(),
            });
        };
        let account = locked.account_id();
        let email = locked.sync_config().ok().flatten().and_then(|c| c.email);
        let typed = confirm.typed_email.trim().to_lowercase();
        let matches = email.is_some_and(|e| e.trim().to_lowercase() == typed) && !typed.is_empty();
        if !matches || account.is_err() {
            self.put_back(Taken::Locked(locked));
            return Err(MobileError::InvalidInput);
        }
        let mut aliases: Vec<String> = locked
            .quick_unlock_enrollment()
            .ok()
            .flatten()
            .and_then(|e| Params::decode(&e.platform_params, self.config.platform).ok())
            .map(|p| p.key_alias())
            .into_iter()
            .collect();
        if let Ok(mut guard) = self.guard() {
            aliases.extend(guard.expired_alias.take());
        }
        let account = account?;
        // A reset that fails half way must not leave a phone that quick-unlocks:
        // the enrolment and the boot marker go first, and the key alias is
        // reported by the state until the platform deleted the key (N7).
        let mut locked = locked;
        let _ = locked.remove_quick_unlock();
        locked.require_password();
        // The secrets first, then the files; a failure keeps the store so the
        // user can try again.
        let deleted = self
            .storage
            .delete(&secret_key_name(&account))
            .and_then(|()| self.storage.delete(&sync_token_name(&account)));
        if let Err(e) = deleted {
            self.put_back(Taken::Locked(locked));
            if let Ok(mut guard) = self.guard() {
                guard.expired_alias = aliases.into_iter().next();
            }
            return Err(e.into());
        }
        self.note_stored(Some(false), Some(false));
        drop(locked);
        // The phase must not stay `Unlocking` if the files cannot be removed
        // (G1-13): the store is opened again as it is (a new, empty one where
        // the file is gone), and the failure is reported after.
        let wiped = self.wipe_files();
        let (phase, busy) = crate::open_phase(&self.config).unwrap_or((Phase::Closed, false));
        {
            let mut guard = self.guard()?;
            guard.phase = phase;
            guard.store_busy = busy;
            guard.slots = Slots::default();
            // A wipe that failed keeps the alias of the removed enrolment
            // reported (N7).
            guard.expired_alias = if wiped.is_err() {
                aliases.first().cloned()
            } else {
                None
            };
            guard.last_sync_at_ms = None;
            guard.last_sync_error = None;
            guard.sync_warnings.clear();
            guard.settings = crate::settings::SettingsGuard::load(self.config.settings_path());
            guard.settle_fresh_phase();
        }
        wiped?;
        Ok(ResetOutcome {
            state: self.state(),
            key_aliases: aliases,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(dir: &std::path::Path) -> UnlockedSession {
        let store = mosaic_vault::storage::Store::open(dir.join("phone.db")).unwrap();
        mosaic_vault::session::LockedSession::open(store)
            .create_account(
                &MasterPassword::new("pw-correct horse".to_owned()),
                mosaic_core::Argon2Profile::Baseline,
            )
            .unwrap()
            .0
    }

    /// SYNC-3: an acknowledgement clears only the sharing findings the phone
    /// showed. One recorded after the last overview stays until it was shown.
    #[test]
    fn acknowledge_keeps_sharing_findings_the_phone_never_showed() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = session(dir.path());
        let vault = s.create_vault("Shared").unwrap();
        let shown = SharingFinding::VaultHeld { vault };
        s.record_sharing_finding(shown).unwrap();
        let unseen = SharingFinding::RosterInvalid { vault };
        s.record_sharing_finding(unseen).unwrap();
        // The overview showed only the first one.
        clear_findings(&mut s, &[shown]).unwrap();
        assert_eq!(s.sharing_findings().unwrap().len(), 2, "nothing is cleared");
        // Once both were shown, the acknowledgement clears them.
        clear_findings(&mut s, &[shown, unseen]).unwrap();
        assert_eq!(s.sharing_findings().unwrap().len(), 0);
    }

    /// SYNC-3: a blocked revocation rotation (and a sharing finding) is a
    /// warning on the phone, not dropped.
    #[test]
    fn rotation_blocked_and_sharing_findings_show_on_the_phone() {
        let vault = mosaic_core::VaultId::new_random().unwrap();
        let blocked = SyncWarning::from_engine(&EngineWarning::RotationBlocked { vault });
        assert_eq!(
            blocked.map(|w| w.kind),
            Some(SyncWarningKind::SharingAttention)
        );
        let none = mosaic_vault::session::SyncFindings::default();
        let kinds: Vec<_> = finding_warnings(&none, true)
            .into_iter()
            .map(|w| w.kind)
            .collect();
        assert_eq!(kinds, vec![SyncWarningKind::SharingAttention]);
        assert_eq!(finding_warnings(&none, false).len(), 0);
    }

    fn warning(kind: SyncWarningKind, since_ms: Option<u64>, count: Option<u32>) -> SyncWarning {
        SyncWarning {
            kind,
            since_ms,
            count,
        }
    }

    /// One entry per kind, the latest count and `since` kept; other kinds and
    /// their order stay.
    #[test]
    fn merge_warnings_keeps_one_entry_per_kind_with_the_latest_values() {
        let mut list = vec![
            warning(SyncWarningKind::NoCommonView, Some(10), None),
            warning(SyncWarningKind::FreezeSuspected, Some(5), None),
        ];
        merge_warnings(
            &mut list,
            [
                warning(SyncWarningKind::NoCommonView, Some(20), Some(3)),
                warning(SyncWarningKind::NoCommonView, Some(30), Some(4)),
                warning(SyncWarningKind::RollbackDetected, None, None),
            ],
        );
        assert_eq!(
            list,
            vec![
                warning(SyncWarningKind::NoCommonView, Some(30), Some(4)),
                warning(SyncWarningKind::FreezeSuspected, Some(5), None),
                warning(SyncWarningKind::RollbackDetected, None, None),
            ]
        );
        // Many runs do not grow the list.
        for run in 0..50 {
            merge_warnings(
                &mut list,
                [warning(
                    SyncWarningKind::NoCommonView,
                    Some(100 + run),
                    None,
                )],
            );
        }
        assert_eq!(list.len(), 3);
    }
}
