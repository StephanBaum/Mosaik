//! Credential rotation in the session (M7b, CRYPTO_SPEC §7.4, §11): the
//! guided password change of a login, the breach check's cache and the
//! rotate-now queue's snooze and dismissal.
//!
//! Every change of an item goes through [`UnlockedSession::update_item`]:
//! one edit, one sealed record, one outbox entry, the normal conflict rules.
//! Each call returns only after the record is committed to the local store,
//! so a crash after the extension filled a change form cannot lose the new
//! value. A viewer of a shared vault gets [`Error::ReadOnly`] for every
//! item change.
//!
//! The breach and queue rows are local state (store schema v10). They hold
//! IDs and Lamport value marks ([`mosaic_core::model::ValueMark`]), never a
//! password or a hash. A mark means something only for one vault, item and
//! field, so a row counts only while it matches the item's current mark.

use std::collections::BTreeMap;

use mosaic_core::model::{
    HistoryId, Item, ItemType, LoginUrl, Role, RotationAttemptId, RotationStage, RulesSource,
    SecretField, TextField, ValueMark,
};
use mosaic_core::{ItemId, SecretString, Timestamp, VaultId};
use mosaic_insights::ItemKey;
use mosaic_insights::breach::{BreachField, BreachRecord, BreachResult, BreachStates, RangeError};
use mosaic_origin::{MatchKind, PageOrigin, fill_allowed};

use super::{Error, Result, UnlockedSession};
use crate::storage::{BreachRow, ScanRow};

/// The most attempts [`UnlockedSession::rotation_attempts_for_origin`]
/// returns.
pub const MAX_ATTEMPTS_FOR_ORIGIN: usize = 5;

/// How the user closed a rotation attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RotationOutcome {
    /// The site accepted the new password: it becomes the item's password.
    Succeeded,
    /// The site did not accept it (or the user gave up): the password stays
    /// and the attempt's value goes to the history as unconfirmed.
    Failed,
}

/// An open rotation attempt that matches a page's origin, without its value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RotationRef {
    /// The vault.
    pub vault_id: VaultId,
    /// The item.
    pub item_id: ItemId,
    /// The attempt.
    pub attempt_id: RotationAttemptId,
    /// The item's title.
    pub title: Option<String>,
    /// The item's user name.
    pub username: Option<String>,
    /// How far the attempt got.
    pub stage: RotationStage,
    /// When it started.
    pub started_at: Timestamp,
}

/// Why a breach scan stopped early (the store's codes 1 to 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanError {
    /// No connection.
    Offline,
    /// The service answered 429 twice.
    RateLimited,
    /// The answer was not what the API sends.
    Protocol,
    /// The user cancelled, or the vault locked.
    Cancelled,
}

impl ScanError {
    const fn code(self) -> u8 {
        match self {
            Self::Offline => 1,
            Self::RateLimited => 2,
            Self::Protocol => 3,
            Self::Cancelled => 4,
        }
    }

    const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Offline),
            2 => Some(Self::RateLimited),
            3 => Some(Self::Protocol),
            4 => Some(Self::Cancelled),
            _ => None,
        }
    }
}

impl From<RangeError> for ScanError {
    fn from(e: RangeError) -> Self {
        match e {
            RangeError::Offline => Self::Offline,
            RangeError::RateLimited { .. } => Self::RateLimited,
            RangeError::Protocol => Self::Protocol,
            RangeError::Cancelled => Self::Cancelled,
        }
    }
}

/// The summary of one breach scan, as stored.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanSummary {
    /// When the scan started.
    pub started_at: Timestamp,
    /// When it finished; `None` if it never did.
    pub finished_at: Option<Timestamp>,
    /// How many secrets it checked.
    pub checked: u32,
    /// Why it stopped early.
    pub error: Option<ScanError>,
}

/// One item's snooze and dismissal in the rotate-now queue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QueueState {
    /// Hidden until then.
    pub snoozed_until: Option<Timestamp>,
    /// Dismissed, and the password has not changed since.
    pub dismissed: bool,
}

impl QueueState {
    /// Whether the queue hides the item at `now`.
    #[must_use]
    pub fn hides(&self, now: Timestamp) -> bool {
        self.dismissed || self.snoozed_until.is_some_and(|until| until > now)
    }
}

/// The queue state of every item that has one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QueueStates(BTreeMap<ItemKey, QueueState>);

impl QueueStates {
    /// The state of one item.
    #[must_use]
    pub fn get(&self, key: ItemKey) -> Option<&QueueState> {
        self.0.get(&key)
    }

    /// Whether the queue hides the item at `now`.
    #[must_use]
    pub fn hides(&self, key: ItemKey, now: Timestamp) -> bool {
        self.get(key).is_some_and(|s| s.hides(now))
    }

    /// Every item with a state.
    pub fn iter(&self) -> impl Iterator<Item = (&ItemKey, &QueueState)> {
        self.0.iter()
    }

    /// How many items have a state.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether no item has a state.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The mark a snooze or dismissal binds to: the login password's, else the
/// Wi-Fi password's.
fn tracked_mark(item: &Item) -> Option<ValueMark> {
    item.secret_mark(SecretField::Password)
        .or_else(|| item.secret_mark(SecretField::WifiPassword))
}

fn ms(t: Timestamp) -> i64 {
    t.as_unix_millis()
}

impl UnlockedSession {
    /// The vault must be unlocked, the item live, and this account must be
    /// allowed to write: a viewer of a shared vault is not.
    fn writable_item(&self, vault: VaultId, item: ItemId) -> Result<()> {
        self.vault(vault)?.live_item(item)?;
        match self.vault_own_role(vault)? {
            Role::Read | Role::Removed => Err(Error::ReadOnly),
            Role::Owner | Role::Manage | Role::Write => Ok(()),
        }
    }

    /// Starts a rotation attempt: `pending`, the new password, is stored in
    /// the item (sealed, queued for sync) before anything reaches a site.
    /// The current password is not touched. Returns after the local commit.
    ///
    /// # Errors
    /// [`Error::ReadOnly`] for a viewer; [`Error::Core`] with
    /// `NotALogin`, `Malformed` (empty or over 1024 characters) or
    /// `TooManyAttempts`; and the errors of [`UnlockedSession::update_item`].
    pub fn begin_rotation(
        &mut self,
        vault: VaultId,
        item: ItemId,
        pending: &SecretString,
        rules: Option<RulesSource>,
    ) -> Result<RotationAttemptId> {
        self.writable_item(vault, item)?;
        let mut id = None;
        self.update_item(vault, item, |e| {
            id = Some(e.begin_rotation(pending.expose_secret(), rules)?);
            Ok(())
        })?;
        id.ok_or(Error::UnknownItem)
    }

    /// Records that the attempt's value was handed to a page. A no-op when
    /// it already was.
    ///
    /// # Errors
    /// [`Error::ReadOnly`]; [`Error::Core`] with `RotationNotFound`; and the
    /// errors of [`UnlockedSession::update_item`].
    pub fn mark_rotation_delivered(
        &mut self,
        vault: VaultId,
        item: ItemId,
        attempt: RotationAttemptId,
    ) -> Result<()> {
        self.writable_item(vault, item)?;
        self.update_item(vault, item, |e| {
            e.mark_rotation_delivered(attempt).map(|_| ())
        })
    }

    /// Closes an attempt (CRYPTO_SPEC §7.4): on success the new value
    /// becomes the password, the old one goes to the history and every other
    /// open attempt goes to the history as unconfirmed; on failure the
    /// password stays and the attempt's value goes to the history as
    /// unconfirmed. One edit either way. The value of an attempt is never
    /// deleted.
    ///
    /// # Errors
    /// As [`UnlockedSession::mark_rotation_delivered`].
    pub fn close_rotation(
        &mut self,
        vault: VaultId,
        item: ItemId,
        attempt: RotationAttemptId,
        outcome: RotationOutcome,
    ) -> Result<()> {
        self.writable_item(vault, item)?;
        self.update_item(vault, item, |e| match outcome {
            RotationOutcome::Succeeded => e.confirm_rotation(attempt),
            RotationOutcome::Failed => e.fail_rotation(attempt),
        })
    }

    /// Adds `url` to the login while the attempt is open (the browser's
    /// "add this address" after a change page redirected). The caller has
    /// checked that the address belongs to the login's site.
    ///
    /// # Errors
    /// As [`UnlockedSession::mark_rotation_delivered`], plus the errors of
    /// the edit (`Malformed` for a URL the item cannot take).
    pub fn add_origin_during_rotation(
        &mut self,
        vault: VaultId,
        item: ItemId,
        attempt: RotationAttemptId,
        url: &LoginUrl,
    ) -> Result<()> {
        self.writable_item(vault, item)?;
        if !self
            .vault(vault)?
            .live_item(item)?
            .rotation_attempts()
            .iter()
            .any(|a| a.id == attempt)
        {
            return Err(Error::Core(mosaic_core::Error::RotationNotFound));
        }
        self.update_item(vault, item, |e| e.add_url(url).map(|_| ()))
    }

    /// The rollback: the history entry's value becomes the password and the
    /// current password goes to the history.
    ///
    /// # Errors
    /// [`Error::ReadOnly`]; [`Error::Core`] with `HistoryNotFound`; and the
    /// errors of [`UnlockedSession::update_item`].
    pub fn restore_password_history(
        &mut self,
        vault: VaultId,
        item: ItemId,
        history: &HistoryId,
    ) -> Result<()> {
        self.writable_item(vault, item)?;
        self.update_item(vault, item, |e| e.restore_history(history))
    }

    /// Sets or clears the item's reminder interval (30, 90, 180 or 365
    /// days).
    ///
    /// # Errors
    /// [`Error::ReadOnly`]; [`Error::Core`] with `Malformed` for another
    /// interval; and the errors of [`UnlockedSession::update_item`].
    pub fn set_rotation_policy(
        &mut self,
        vault: VaultId,
        item: ItemId,
        every_days: Option<u16>,
    ) -> Result<()> {
        self.writable_item(vault, item)?;
        self.update_item(vault, item, |e| e.set_rotation_policy(every_days))
    }

    /// The open attempts of logins that have a URL at exactly `origin`
    /// (scheme, host and port equal, through `mosaic_origin::fill_allowed`:
    /// no eTLD+1 match, no plain-http match without the item's override),
    /// newest first, at most [`MAX_ATTEMPTS_FOR_ORIGIN`]. Items in the
    /// trash are skipped. The attempts' values are not returned.
    #[must_use]
    pub fn rotation_attempts_for_origin(&self, origin: &PageOrigin) -> Vec<RotationRef> {
        let mut out = Vec::new();
        for (vault_id, ov) in &self.vaults {
            for (item_id, item) in &ov.items {
                if item.is_deleted()
                    || item.trashed_at().is_some()
                    || item.item_type() != Some(ItemType::Login)
                {
                    continue;
                }
                let attempts = item.rotation_attempts();
                if attempts.is_empty()
                    || !item.urls().iter().any(|(_, url)| {
                        matches!(fill_allowed(url, origin), Ok(MatchKind::ExactOrigin))
                    })
                {
                    continue;
                }
                for a in attempts {
                    out.push(RotationRef {
                        vault_id: *vault_id,
                        item_id: *item_id,
                        attempt_id: a.id,
                        title: item.title().map(str::to_owned),
                        username: item.text(TextField::Username).map(str::to_owned),
                        stage: a.stage,
                        started_at: a.started_at,
                    });
                }
            }
        }
        out.sort_by(|a, b| {
            (b.started_at, b.vault_id, b.item_id, b.attempt_id).cmp(&(
                a.started_at,
                a.vault_id,
                a.item_id,
                a.attempt_id,
            ))
        });
        out.truncate(MAX_ATTEMPTS_FOR_ORIGIN);
        out
    }

    /// The cached breach answers that still describe the item's current
    /// value: a row whose mark differs from the item's (the password
    /// changed, a rotation was confirmed), or whose item is gone, is
    /// dropped.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn breach_states(&self) -> Result<BreachStates> {
        let mut out = BreachStates::new();
        for row in self.store.breach_rows()? {
            let Some(field) = BreachField::from_code(row.field) else {
                continue;
            };
            let Some(item) = self.live_item_of(row.vault_id, row.item_id) else {
                continue;
            };
            let Some(current) = item.secret_mark(field.secret_field()) else {
                continue;
            };
            if ValueMark::from_bytes(&row.mark) != Some(current) {
                continue;
            }
            out.insert(
                ItemKey {
                    vault_id: row.vault_id,
                    item_id: row.item_id,
                },
                field,
                BreachRecord {
                    mark: current,
                    checked_at: Timestamp::from_unix_millis(row.checked_at_ms),
                    found: row.found,
                },
            );
        }
        Ok(out)
    }

    /// Stores the answers of a scan and its summary in one transaction. An
    /// answer whose mark no longer matches the item (the password changed
    /// while the scan ran) or whose item is gone is skipped. Nothing of the
    /// password, not even its hash prefix, is stored.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn record_breach_results(
        &mut self,
        results: &[BreachResult],
        scan: ScanSummary,
    ) -> Result<()> {
        let rows: Vec<BreachRow> = results
            .iter()
            .filter(|r| {
                self.live_item_of(r.key.vault_id, r.key.item_id)
                    .and_then(|item| item.secret_mark(r.field.secret_field()))
                    == Some(r.mark)
            })
            .map(|r| BreachRow {
                vault_id: r.key.vault_id,
                item_id: r.key.item_id,
                field: r.field.code(),
                mark: r.mark.as_bytes().to_vec(),
                checked_at_ms: ms(r.checked_at),
                found: r.found,
            })
            .collect();
        let scan_row = ScanRow {
            started_at_ms: Some(ms(scan.started_at)),
            finished_at_ms: scan.finished_at.map(ms),
            checked: scan.checked,
            error: scan.error.map(ScanError::code),
        };
        self.store.put_breach_results(&rows, &scan_row)?;
        Ok(())
    }

    /// The summary of the last breach scan, if one ran.
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn breach_scan(&self) -> Result<Option<ScanSummary>> {
        Ok(self.store.breach_scan()?.map(|r| ScanSummary {
            started_at: Timestamp::from_unix_millis(r.started_at_ms.unwrap_or(0)),
            finished_at: r.finished_at_ms.map(Timestamp::from_unix_millis),
            checked: r.checked,
            error: r.error.and_then(ScanError::from_code),
        }))
    }

    /// Hides the item from the rotate-now queue until `until`.
    ///
    /// # Errors
    /// [`Error::UnknownVault`], [`Error::UnknownItem`], [`Error::ItemDeleted`],
    /// [`Error::Store`].
    pub fn snooze_rotation(
        &mut self,
        vault: VaultId,
        item: ItemId,
        until: Timestamp,
    ) -> Result<()> {
        self.vault(vault)?.live_item(item)?;
        self.store.set_snooze(&vault, &item, ms(until))?;
        Ok(())
    }

    /// Dismisses the item in the rotate-now queue until its password
    /// changes: the current password mark is stored, and the dismissal
    /// counts only while the mark matches.
    ///
    /// # Errors
    /// [`Error::NoPassword`] for an item without a login or Wi-Fi password;
    /// as [`UnlockedSession::snooze_rotation`].
    pub fn dismiss_rotation(&mut self, vault: VaultId, item: ItemId) -> Result<()> {
        let mark = tracked_mark(self.vault(vault)?.live_item(item)?).ok_or(Error::NoPassword)?;
        self.store.set_dismissed(&vault, &item, mark.as_bytes())?;
        Ok(())
    }

    /// The snooze and dismissal of every live item that has one. A
    /// dismissal whose mark no longer matches the password is not reported;
    /// a snooze is reported as stored ([`QueueState::hides`] compares it
    /// with the clock).
    ///
    /// # Errors
    /// [`Error::Store`].
    pub fn queue_states(&self) -> Result<QueueStates> {
        let mut out = BTreeMap::new();
        for row in self.store.queue_rows()? {
            let Some(item) = self.live_item_of(row.vault_id, row.item_id) else {
                continue;
            };
            let dismissed = match (&row.dismissed_mark, tracked_mark(item)) {
                (Some(stored), Some(current)) => stored.as_slice() == current.as_bytes(),
                _ => false,
            };
            let snoozed_until = row.snoozed_until_ms.map(Timestamp::from_unix_millis);
            if !dismissed && snoozed_until.is_none() {
                continue;
            }
            out.insert(
                ItemKey {
                    vault_id: row.vault_id,
                    item_id: row.item_id,
                },
                QueueState {
                    snoozed_until,
                    dismissed,
                },
            );
        }
        Ok(QueueStates(out))
    }

    fn live_item_of(&self, vault: VaultId, item: ItemId) -> Option<&Item> {
        self.vaults
            .get(&vault)?
            .items
            .get(&item)
            .filter(|i| !i.is_deleted())
    }
}
