//! Credential rotation (M7b): the rotate-now queue, the guided password
//! change of one login and the opt-in breach check (`docs/DESKTOP_IPC.md`,
//! "Credential rotation"). Not the key rotation of the account (`rotation`).
//!
//! The new password of an attempt travels only in `rotation_begin`
//! ([`SecretIn`]) and comes back only through `rotation_reveal`
//! ([`super::items::RevealedSecret`]); the queue, the item detail and the
//! events carry attempt IDs, stages and times, never a value.

use serde::{Deserialize, Serialize};

use super::ids::{UnixMillis, WireItemId, WireVaultId};
use super::insights::InsightSeverity;
use super::macros::wire_enum;
use super::secret::SecretIn;
use super::settings::BreachMode;

wire_enum!(
    /// Whether Mosaic can guide the change.
    pub enum QueueKind {
        /// A login with an `https` URL: the guided change works.
        Guided,
        /// Anything else: the queue only offers to edit the item.
        Manual,
    }
);

wire_enum!(
    /// Why an item is in the rotate-now queue.
    pub enum QueueReason {
        /// The password is weak.
        Weak,
        /// The password is used by another item.
        Reused,
        /// The password is older than the reminder interval or a year.
        Old,
        /// The password is in a known data breach (opt-in check).
        Breached,
        /// A password change was started and not confirmed.
        Unfinished,
    }
);

wire_enum!(
    /// How far a change attempt got.
    pub enum AttemptStage {
        /// The new password is stored in the item; no page got it yet.
        Prepared,
        /// A page got it (the extension filled the change form).
        Delivered,
    }
);

wire_enum!(
    /// Where the generator rules of a new password came from, when not the
    /// page itself.
    pub enum RulesSourceKey {
        /// The vendored per-site rules of the generator.
        SiteQuirks,
    }
);

wire_enum!(
    /// Where a change-page URL comes from.
    pub enum ChangeUrlSource {
        /// The item's own `change password URL` field.
        Stored,
        /// The vendored list of sites that publish one.
        Quirks,
        /// `/.well-known/change-password` of the item's site.
        WellKnown,
    }
);

wire_enum!(
    /// How the user closes a change attempt.
    pub enum RotationOutcomeKey {
        /// The site accepted the new password.
        Succeeded,
        /// It did not, or the user gave up.
        Failed,
    }
);

wire_enum!(
    /// Why a value is in the password history.
    pub enum HistoryReasonKey {
        /// A confirmed change replaced it.
        Replaced,
        /// A change attempt's value that was never confirmed.
        Unconfirmed,
    }
);

wire_enum!(
    /// Why a breach scan stopped early.
    pub enum BreachScanError {
        /// No connection.
        Offline,
        /// The service answered 429 twice.
        RateLimited,
        /// The answer was not what the API sends.
        Protocol,
        /// The user cancelled, or the vault locked.
        Cancelled,
    }
);

/// An open change attempt, without its value.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationAttemptView {
    /// The attempt (a short ASCII key suffix).
    pub attempt_id: String,
    /// How far it got.
    pub stage: AttemptStage,
    /// When it started.
    pub started_at: UnixMillis,
}

/// One entry of the password history, without its value.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntryView {
    /// The entry, for `password_history_restore`.
    pub history_id: String,
    /// Position, 0 = newest (what `reveal_secret` takes).
    pub index: u32,
    /// Why it is here.
    pub reason: HistoryReasonKey,
    /// When it was replaced.
    pub changed_at: UnixMillis,
}

/// A change-page URL. Always `https`; Mosaic never opens or probes it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeUrlView {
    /// The URL.
    pub url: String,
    /// Where it came from.
    pub source: ChangeUrlSource,
}

/// One item in the rotate-now queue.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEntry {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Its title (`null` if empty).
    pub title: Option<String>,
    /// Its user name.
    pub username: Option<String>,
    /// The host of its first URL.
    pub host: Option<String>,
    /// Whether the guided change is available.
    pub kind: QueueKind,
    /// Why it is listed, in the order weak, reused, old, breached,
    /// unfinished.
    pub reasons: Vec<QueueReason>,
    /// The most severe finding (an unfinished change alone is `medium`).
    pub severity: InsightSeverity,
    /// Its open change attempts.
    pub attempts: Vec<RotationAttemptView>,
    /// Snoozed until then (`null` if not snoozed). Snoozed entries are
    /// listed so the UI can show them apart; dismissed ones are not.
    pub snoozed_until: Option<UnixMillis>,
}

/// The breach check as the queue and the dashboard show it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreachStatus {
    /// `off` (the default), `manual` or `weekly`.
    pub mode: BreachMode,
    /// When the user agreed to the check (`null` while off).
    pub consent_at: Option<UnixMillis>,
    /// When the last scan ended (`null` if none did).
    pub last_finished_at: Option<UnixMillis>,
    /// A scan is running now.
    pub running: bool,
    /// Why the last scan stopped early (`null` if it did not).
    pub last_error: Option<BreachScanError>,
}

/// The `rotation_queue` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationQueue {
    /// Items to change: more severe first, then unfinished ones, then by
    /// title.
    pub entries: Vec<QueueEntry>,
    /// The breach check.
    pub breach: BreachStatus,
}

/// `rotation_begin`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationBeginRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The login.
    pub item_id: WireItemId,
    /// The new password (1 to 1024 characters).
    pub new_password: SecretIn,
    /// Where the generator rules came from; `null` for the defaults.
    pub rules_source: Option<RulesSourceKey>,
}

/// The `rotation_begin` response.
#[derive(Clone, PartialEq, Eq, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RotationBegun {
    /// The new attempt.
    pub attempt_id: String,
    /// The change page to open in the browser, if the login has an `https`
    /// URL.
    pub change_url: Option<ChangeUrlView>,
}

/// An open attempt of an item (`rotation_reveal`, `rotation_copy`).
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationAttemptRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The attempt.
    pub attempt_id: String,
}

/// `rotation_close`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationCloseRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The attempt.
    pub attempt_id: String,
    /// How it ended.
    pub outcome: RotationOutcomeKey,
}

/// `password_history_restore`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryRestoreRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// The history entry.
    pub history_id: String,
}

/// `rotation_snooze`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationSnoozeRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// 1, 7 or 30; anything else is `invalidInput`.
    pub days: u32,
}

/// `rotation_set_policy`.
#[derive(Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RotationPolicyRequest {
    /// The vault.
    pub vault_id: WireVaultId,
    /// The item.
    pub item_id: WireItemId,
    /// Remind every 30, 90, 180 or 365 days; `null` clears the reminder.
    pub every_days: Option<u32>,
}

#[cfg(test)]
pub(crate) mod samples {
    use super::*;
    use crate::contract::ids::samples as ids;

    pub(crate) fn attempt() -> RotationAttemptView {
        RotationAttemptView {
            attempt_id: "00000000000000a1.0123456789abcdef.0".into(),
            stage: AttemptStage::Prepared,
            started_at: 1_761_480_000_000,
        }
    }

    pub(crate) fn history() -> Vec<HistoryEntryView> {
        vec![
            HistoryEntryView {
                history_id: "00000000000000a2.0123456789abcdef.0".into(),
                index: 0,
                reason: HistoryReasonKey::Unconfirmed,
                changed_at: 1_761_480_100_000,
            },
            HistoryEntryView {
                history_id: "00000000000000a0.0123456789abcdef.0".into(),
                index: 1,
                reason: HistoryReasonKey::Replaced,
                changed_at: 1_761_400_000_000,
            },
        ]
    }

    pub(crate) fn change_url() -> ChangeUrlView {
        ChangeUrlView {
            url: "https://example.com/.well-known/change-password".into(),
            source: ChangeUrlSource::WellKnown,
        }
    }

    pub(crate) fn breach_status() -> Vec<BreachStatus> {
        vec![
            BreachStatus {
                mode: BreachMode::Off,
                consent_at: None,
                last_finished_at: None,
                running: false,
                last_error: None,
            },
            BreachStatus {
                mode: BreachMode::Weekly,
                consent_at: Some(1_761_000_000_000),
                last_finished_at: Some(1_761_470_000_000),
                running: true,
                last_error: Some(BreachScanError::Offline),
            },
        ]
    }

    pub(crate) fn queues() -> Vec<RotationQueue> {
        vec![
            RotationQueue {
                entries: vec![
                    QueueEntry {
                        vault_id: ids::vault(1),
                        item_id: ids::item(1),
                        title: Some("Example".into()),
                        username: Some("alex@example.com".into()),
                        host: Some("example.com".into()),
                        kind: QueueKind::Guided,
                        reasons: vec![QueueReason::Weak, QueueReason::Unfinished],
                        severity: InsightSeverity::High,
                        attempts: vec![attempt()],
                        snoozed_until: None,
                    },
                    QueueEntry {
                        vault_id: ids::vault(1),
                        item_id: ids::item(2),
                        title: None,
                        username: None,
                        host: None,
                        kind: QueueKind::Manual,
                        reasons: vec![QueueReason::Old, QueueReason::Breached],
                        severity: InsightSeverity::High,
                        attempts: Vec::new(),
                        snoozed_until: Some(1_761_990_000_000),
                    },
                ],
                breach: breach_status().remove(1),
            },
            RotationQueue {
                entries: Vec::new(),
                breach: breach_status().remove(0),
            },
        ]
    }

    pub(crate) fn begin_requests() -> Vec<RotationBeginRequest> {
        vec![
            RotationBeginRequest {
                vault_id: ids::vault(1),
                item_id: ids::item(1),
                new_password: SecretIn::test("q7#Vt2-new"),
                rules_source: None,
            },
            RotationBeginRequest {
                vault_id: ids::vault(1),
                item_id: ids::item(1),
                new_password: SecretIn::test("q7#Vt2-new"),
                rules_source: Some(RulesSourceKey::SiteQuirks),
            },
        ]
    }

    pub(crate) fn begun() -> Vec<RotationBegun> {
        vec![
            RotationBegun {
                attempt_id: attempt().attempt_id,
                change_url: Some(change_url()),
            },
            RotationBegun {
                attempt_id: attempt().attempt_id,
                change_url: None,
            },
        ]
    }

    pub(crate) fn attempt_request() -> RotationAttemptRequest {
        RotationAttemptRequest {
            vault_id: ids::vault(1),
            item_id: ids::item(1),
            attempt_id: attempt().attempt_id,
        }
    }
}
