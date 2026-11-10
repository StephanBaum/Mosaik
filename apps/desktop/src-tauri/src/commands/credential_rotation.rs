//! Credential rotation (M7b): the rotate-now queue, the guided password
//! change of one login (`rotation_begin` ... `rotation_close`), the history
//! restore, snooze, dismiss and the reminder interval.
//!
//! The new password of an attempt is stored in the item, sealed and queued
//! for sync, before `rotation_begin` returns; from then on it is the item's
//! plaintext like any field. It never leaves Rust except through
//! `rotation_reveal` (shown) and `rotation_copy` (clipboard). Nothing here
//! opens a URL or makes a request: the change page is offered as a link to
//! copy, and the browser extension opens it (OD-M7b-4).

use std::collections::BTreeMap;

use mosaic_core::model::{
    HistoryId, HistoryReason, Item, ItemType, Role, RotationAttemptId, RotationStage, RulesSource,
    TextField,
};
use mosaic_core::{SecretString, Timestamp, VaultId};
use mosaic_insights::change_url::{self, ChangeUrlSource as CoreSource};
use mosaic_insights::{
    Candidate, Extra, Finding, FindingKind, ItemFindings, ItemKey, Options, Severity, analyze_with,
};
use mosaic_vault::session::{ItemFilter, ItemRef, QueueStates, RotationOutcome, UnlockedSession};
use tauri::State;

use super::insights::now;
use super::items::{host_of, non_empty};
use crate::contract::clipboard::CopyReceipt;
use crate::contract::credential_rotation::{
    AttemptStage, ChangeUrlSource, ChangeUrlView, HistoryEntryView, HistoryReasonKey,
    HistoryRestoreRequest, QueueEntry, QueueKind, QueueReason, RotationAttemptRequest,
    RotationAttemptView, RotationBeginRequest, RotationBegun, RotationCloseRequest,
    RotationOutcomeKey, RotationPolicyRequest, RotationQueue, RotationSnoozeRequest,
    RulesSourceKey,
};
use crate::contract::errors::{ErrorCode, IpcError, IpcResult};
use crate::contract::insights::InsightSeverity;
use crate::contract::items::{ItemRequest, RevealedSecret};
use crate::contract::secret::SecretOut;
use crate::state::{AppState, Backend};

/// Days the snooze buttons offer.
const SNOOZE_DAYS: [u32; 3] = [1, 7, 30];
/// The reminder intervals of an item.
const POLICY_DAYS: [u32; 4] = [30, 90, 180, 365];
const DAY_MS: i64 = 86_400_000;

/// `rotation_queue`.
#[tauri::command]
pub async fn rotation_queue(state: State<'_, AppState>) -> IpcResult<RotationQueue> {
    state.run(|b| b.rotation_queue(now())).await
}

/// `rotation_begin`.
#[tauri::command]
pub async fn rotation_begin(
    state: State<'_, AppState>,
    request: RotationBeginRequest,
) -> IpcResult<RotationBegun> {
    state.run(move |b| b.rotation_begin(&request)).await
}

/// `rotation_close`.
#[tauri::command]
pub async fn rotation_close(
    state: State<'_, AppState>,
    request: RotationCloseRequest,
) -> IpcResult<()> {
    state.run(move |b| b.rotation_close(&request)).await
}

/// `rotation_reveal`.
#[tauri::command]
pub async fn rotation_reveal(
    state: State<'_, AppState>,
    request: RotationAttemptRequest,
) -> IpcResult<RevealedSecret> {
    state.run(move |b| b.rotation_reveal(&request)).await
}

/// `rotation_copy`.
#[tauri::command]
pub async fn rotation_copy(
    state: State<'_, AppState>,
    request: RotationAttemptRequest,
) -> IpcResult<CopyReceipt> {
    state.run(move |b| b.rotation_copy(&request)).await
}

/// `password_history_restore`.
#[tauri::command]
pub async fn password_history_restore(
    state: State<'_, AppState>,
    request: HistoryRestoreRequest,
) -> IpcResult<()> {
    state
        .run(move |b| b.password_history_restore(&request))
        .await
}

/// `rotation_snooze`.
#[tauri::command]
pub async fn rotation_snooze(
    state: State<'_, AppState>,
    request: RotationSnoozeRequest,
) -> IpcResult<()> {
    state.run(move |b| b.rotation_snooze(&request, now())).await
}

/// `rotation_dismiss`.
#[tauri::command]
pub async fn rotation_dismiss(state: State<'_, AppState>, request: ItemRequest) -> IpcResult<()> {
    state.run(move |b| b.rotation_dismiss(request)).await
}

/// `rotation_set_policy`.
#[tauri::command]
pub async fn rotation_set_policy(
    state: State<'_, AppState>,
    request: RotationPolicyRequest,
) -> IpcResult<()> {
    state.run(move |b| b.rotation_set_policy(&request)).await
}

fn invalid() -> IpcError {
    IpcError::new(ErrorCode::InvalidInput)
}

fn attempt_id(text: &str) -> IpcResult<RotationAttemptId> {
    if text.len() > 64 {
        return Err(invalid());
    }
    RotationAttemptId::from_stored(text).map_err(|_| invalid())
}

const fn wire_stage(stage: RotationStage) -> AttemptStage {
    match stage {
        RotationStage::Prepared => AttemptStage::Prepared,
        RotationStage::Delivered => AttemptStage::Delivered,
    }
}

/// The open attempts of an item as the webview sees them (no value).
pub(super) fn attempt_views(item: &Item) -> Vec<RotationAttemptView> {
    item.rotation_attempts()
        .iter()
        .map(|a| RotationAttemptView {
            attempt_id: a.id.to_string(),
            stage: wire_stage(a.stage),
            started_at: a.started_at.as_unix_millis(),
        })
        .collect()
}

/// The password history of an item, newest first, without values.
pub(super) fn history_views(item: &Item) -> Vec<HistoryEntryView> {
    item.password_history()
        .iter()
        .enumerate()
        .map(|(index, e)| HistoryEntryView {
            history_id: e.id.as_str().to_owned(),
            index: u32::try_from(index).unwrap_or(u32::MAX),
            reason: match e.reason {
                HistoryReason::Unconfirmed => HistoryReasonKey::Unconfirmed,
                HistoryReason::Replaced => HistoryReasonKey::Replaced,
            },
            changed_at: e.changed_at.as_unix_millis(),
        })
        .collect()
}

/// The change page of a login (`None` without an `https` URL).
pub(super) fn change_url_view(item: &Item) -> Option<ChangeUrlView> {
    if item.item_type() != Some(ItemType::Login) {
        return None;
    }
    change_url::resolve(item).map(|c| ChangeUrlView {
        url: c.url,
        source: match c.source {
            CoreSource::Stored => ChangeUrlSource::Stored,
            CoreSource::Quirks => ChangeUrlSource::Quirks,
            CoreSource::WellKnown => ChangeUrlSource::WellKnown,
        },
    })
}

const fn wire_severity(severity: Severity) -> InsightSeverity {
    match severity {
        Severity::Low => InsightSeverity::Low,
        Severity::Medium => InsightSeverity::Medium,
        Severity::High => InsightSeverity::High,
    }
}

fn reason_of(kind: FindingKind) -> Option<QueueReason> {
    match kind {
        FindingKind::Weak => Some(QueueReason::Weak),
        FindingKind::Reused => Some(QueueReason::Reused),
        FindingKind::Old => Some(QueueReason::Old),
        FindingKind::Breached => Some(QueueReason::Breached),
        _ => None,
    }
}

/// Whether this account may change items of the vault (a viewer of a shared
/// vault may not).
fn can_write(session: &UnlockedSession, vault: VaultId) -> bool {
    matches!(
        session.vault_own_role(vault),
        Ok(Role::Owner | Role::Manage | Role::Write)
    )
}

/// The queue entry of one item, or `None` if it is not listed.
fn queue_entry(
    session: &UnlockedSession,
    e: &ItemRef<'_>,
    findings: &BTreeMap<ItemKey, &ItemFindings>,
    states: &QueueStates,
    now: Timestamp,
) -> Option<QueueEntry> {
    let key = ItemKey {
        vault_id: e.vault_id,
        item_id: e.item_id,
    };
    let attempts = attempt_views(e.item);
    let rotation_findings: Vec<&Finding> = findings
        .get(&key)
        .map(|f| {
            f.findings
                .iter()
                .filter(|x| reason_of(x.kind).is_some())
                .collect()
        })
        .unwrap_or_default();
    let mut reasons: Vec<QueueReason> = rotation_findings
        .iter()
        .filter_map(|x| reason_of(x.kind))
        .collect();
    if !attempts.is_empty() {
        reasons.push(QueueReason::Unfinished);
    }
    if reasons.is_empty() {
        return None;
    }
    // Dismissed until the password changes; an open change is never
    // dismissed away (it holds a password that may be live).
    let state = states.get(key);
    if state.is_some_and(|s| s.dismissed) && attempts.is_empty() {
        return None;
    }
    reasons.sort_by_key(|r| QueueReason::ALL.iter().position(|x| x == r));
    reasons.dedup();
    let guided = e.item.item_type() == Some(ItemType::Login)
        && change_url::resolve(e.item).is_some()
        && can_write(session, e.vault_id);
    Some(QueueEntry {
        vault_id: e.vault_id.into(),
        item_id: e.item_id.into(),
        title: non_empty(e.item.title()),
        username: non_empty(e.item.text(TextField::Username)),
        host: e
            .item
            .urls()
            .first()
            .and_then(|(_, url)| host_of(url.url.as_str())),
        kind: if guided {
            QueueKind::Guided
        } else {
            QueueKind::Manual
        },
        reasons,
        severity: rotation_findings
            .iter()
            .map(|x| x.severity)
            .max()
            .map_or(InsightSeverity::Medium, wire_severity),
        attempts,
        snoozed_until: state
            .and_then(|s| s.snoozed_until)
            .filter(|t| *t > now)
            .map(Timestamp::as_unix_millis),
    })
}

/// More severe first, then unfinished ones, then by title and IDs.
fn sort_queue(entries: &mut [QueueEntry]) {
    let rank = |s: InsightSeverity| match s {
        InsightSeverity::High => 0,
        InsightSeverity::Medium => 1,
        InsightSeverity::Low => 2,
    };
    let unfinished = |e: &QueueEntry| !e.attempts.is_empty();
    entries.sort_by(|a, b| {
        rank(a.severity)
            .cmp(&rank(b.severity))
            .then_with(|| unfinished(b).cmp(&unfinished(a)))
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.vault_id.cmp(&b.vault_id))
            .then_with(|| a.item_id.cmp(&b.item_id))
    });
}

impl Backend {
    /// The rotate-now queue: items with a weak, reused, old or breached
    /// password or an unfinished change, minus the dismissed ones (snoozed
    /// ones carry `snoozedUntil`). Offline: the breach findings are the
    /// cached ones.
    ///
    /// # Errors
    /// `locked`; `internal` if the checks fail.
    pub fn rotation_queue(&self, now: Timestamp) -> IpcResult<RotationQueue> {
        self.touch();
        let slot = self.slot()?;
        let session = slot.unlocked()?;
        let entries = session.items(&ItemFilter::default());
        let candidates = || {
            entries.iter().map(|e| Candidate {
                vault_id: e.vault_id,
                item_id: e.item_id,
                item: e.item,
            })
        };
        let breached = session.breach_states()?.breached_items(candidates());
        let report = analyze_with(
            candidates(),
            now,
            &Options::default(),
            &Extra::new(&breached),
        )
        .map_err(|_| IpcError::INTERNAL)?;
        let states = session.queue_states()?;
        let findings: BTreeMap<ItemKey, &ItemFindings> =
            report.items.iter().map(|f| (f.item, f)).collect();
        let mut out: Vec<QueueEntry> = entries
            .iter()
            .filter_map(|e| queue_entry(session, e, &findings, &states, now))
            .collect();
        sort_queue(&mut out);
        Ok(RotationQueue {
            entries: out,
            breach: self.breach_status(session),
        })
    }

    /// Starts a change attempt: the new password is stored in the item
    /// (sealed, queued for sync) before this returns. The item's password is
    /// not touched.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `vaultUnavailable`, `rotationNotSupported`
    /// (not a login, or no `https` URL), `rotationLimit` (four attempts are
    /// open), `invalidInput` (an empty password or one over 1024
    /// characters), `sharingRoleForbidden` (a viewer).
    pub fn rotation_begin(&self, request: &RotationBeginRequest) -> IpcResult<RotationBegun> {
        self.touch();
        let (vault, item) = (request.vault_id.get(), request.item_id.get());
        let pending: &SecretString = request.new_password.as_secret();
        let length = pending.expose_secret().chars().count();
        if length == 0 || length > mosaic_core::model::MAX_PENDING_CHARS {
            return Err(invalid());
        }
        let rules = request.rules_source.map(|r| match r {
            RulesSourceKey::SiteQuirks => RulesSource::SiteQuirks,
        });
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        let current = session.item(vault, item)?;
        if current.is_deleted() {
            return Err(IpcError::new(ErrorCode::ItemDeleted));
        }
        if current.item_type() != Some(ItemType::Login) || change_url::resolve(current).is_none() {
            return Err(IpcError::new(ErrorCode::RotationNotSupported));
        }
        let id = session.begin_rotation(vault, item, pending, rules)?;
        self.items_changed(session);
        Ok(RotationBegun {
            attempt_id: id.to_string(),
            change_url: change_url_view(session.item(vault, item)?),
        })
    }

    /// Closes an attempt: on success the new value becomes the password and
    /// the old one goes to the history; on failure the value goes to the
    /// history as unconfirmed. Nothing is deleted.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `rotationNotFound`, `invalidInput`,
    /// `sharingRoleForbidden`.
    pub fn rotation_close(&self, request: &RotationCloseRequest) -> IpcResult<()> {
        self.touch();
        let attempt = attempt_id(&request.attempt_id)?;
        let outcome = match request.outcome {
            RotationOutcomeKey::Succeeded => RotationOutcome::Succeeded,
            RotationOutcomeKey::Failed => RotationOutcome::Failed,
        };
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.close_rotation(
            request.vault_id.get(),
            request.item_id.get(),
            attempt,
            outcome,
        )?;
        self.items_changed(session);
        Ok(())
    }

    fn attempt_value(&self, request: &RotationAttemptRequest) -> IpcResult<SecretString> {
        let attempt = attempt_id(&request.attempt_id)?;
        let slot = self.slot()?;
        let item = slot
            .unlocked()?
            .item(request.vault_id.get(), request.item_id.get())?;
        item.rotation_attempts()
            .into_iter()
            .find(|a| a.id == attempt)
            .map(|a| a.pending)
            .ok_or(IpcError::new(ErrorCode::RotationNotFound))
    }

    /// The new password of an open attempt, to show it.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `rotationNotFound`, `invalidInput`.
    pub fn rotation_reveal(&self, request: &RotationAttemptRequest) -> IpcResult<RevealedSecret> {
        self.touch();
        Ok(RevealedSecret {
            value: SecretOut::new(self.attempt_value(request)?),
            hide_after_seconds: super::items::REVEAL_SECONDS,
        })
    }

    /// Copies the new password of an open attempt; the clipboard clears
    /// itself. The value leaves the app, so the attempt is marked delivered
    /// (ADR-0066): if the copy is pasted into a site and the user never says
    /// how it went, the attempt keeps its "delivered" stage.
    ///
    /// # Errors
    /// As [`Backend::rotation_reveal`], and `clipboardUnavailable`,
    /// `sharingRoleForbidden`.
    pub fn rotation_copy(&self, request: &RotationAttemptRequest) -> IpcResult<CopyReceipt> {
        self.touch();
        let attempt = attempt_id(&request.attempt_id)?;
        let value = self.attempt_value(request)?;
        let receipt = self.copy_secret(value)?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        match session.mark_rotation_delivered(
            request.vault_id.get(),
            request.item_id.get(),
            attempt,
        ) {
            Ok(()) => self.items_changed(session),
            // A viewer of a shared vault may copy the value but cannot change the item.
            Err(mosaic_vault::session::Error::ReadOnly) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(receipt)
    }

    /// The rollback: a history entry's value becomes the password and the
    /// current password goes to the history.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `historyNotFound`, `invalidInput`,
    /// `sharingRoleForbidden`.
    pub fn password_history_restore(&self, request: &HistoryRestoreRequest) -> IpcResult<()> {
        self.touch();
        if request.history_id.len() > 128 {
            return Err(invalid());
        }
        let history = HistoryId::from_stored(&request.history_id).map_err(|_| invalid())?;
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.restore_password_history(
            request.vault_id.get(),
            request.item_id.get(),
            &history,
        )?;
        self.items_changed(session);
        Ok(())
    }

    /// Hides the item from the queue for 1, 7 or 30 days.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `itemDeleted`, `invalidInput`.
    pub fn rotation_snooze(
        &self,
        request: &RotationSnoozeRequest,
        now: Timestamp,
    ) -> IpcResult<()> {
        self.touch();
        if !SNOOZE_DAYS.contains(&request.days) {
            return Err(invalid());
        }
        let until = Timestamp::from_unix_millis(
            now.as_unix_millis()
                .saturating_add(i64::from(request.days).saturating_mul(DAY_MS)),
        );
        let mut slot = self.slot()?;
        slot.unlocked_mut()?.snooze_rotation(
            request.vault_id.get(),
            request.item_id.get(),
            until,
        )?;
        Ok(())
    }

    /// Hides the item from the queue until its password changes.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `itemDeleted`, `fieldEmpty` (no password).
    pub fn rotation_dismiss(&self, request: ItemRequest) -> IpcResult<()> {
        self.touch();
        let mut slot = self.slot()?;
        slot.unlocked_mut()?
            .dismiss_rotation(request.vault_id.get(), request.item_id.get())?;
        Ok(())
    }

    /// Sets or clears the item's reminder interval.
    ///
    /// # Errors
    /// `locked`, `itemNotFound`, `invalidInput` (not 30, 90, 180 or 365),
    /// `sharingRoleForbidden`.
    pub fn rotation_set_policy(&self, request: &RotationPolicyRequest) -> IpcResult<()> {
        self.touch();
        let days = match request.every_days {
            None => None,
            Some(d) if POLICY_DAYS.contains(&d) => Some(u16::try_from(d).map_err(|_| invalid())?),
            Some(_) => return Err(invalid()),
        };
        let mut slot = self.slot()?;
        let session = slot.unlocked_mut()?;
        session.set_rotation_policy(request.vault_id.get(), request.item_id.get(), days)?;
        self.items_changed(session);
        Ok(())
    }
}
