//! Tests of the credential rotation and breach commands (M7b T3): the
//! guided change and its outcomes, the history restore, the queue with snooze
//! and dismissal, the consent rules of the breach check, a scan against a
//! fake range source and its stop on lock. Nothing here touches the network:
//! the real HIBP client is never built (`no_network_request_without_opt_in`).

#![expect(
    clippy::too_many_lines,
    reason = "each test reads as one story: the commands in the order a user runs them"
)]

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, Weak};

use mosaic_core::model::{ItemType, LoginUrl, SecretField};
use mosaic_core::{Argon2Profile, ItemId, Timestamp, VaultId};
use mosaic_insights::breach::{RangeError, RangePrefix, RangeSource};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use mosaic_vault::storage::Store;
use zeroize::Zeroizing;

use crate::contract::credential_rotation::{
    AttemptStage, BreachScanError, HistoryReasonKey, HistoryRestoreRequest, QueueKind, QueueReason,
    RotationAttemptRequest, RotationBeginRequest, RotationCloseRequest, RotationOutcomeKey,
    RotationPolicyRequest, RotationSnoozeRequest,
};
use crate::contract::errors::{ErrorCode, IpcError};
use crate::contract::events::LockReason;
use crate::contract::items::ItemRequest;
use crate::contract::secret::SecretIn;
use crate::contract::session::CreateAccountRequest;
use crate::contract::settings::{BreachMode, Settings, UpdateSettingsRequest};
use crate::events::testing::{Log, recording_channel};
use crate::keystore::MemoryKeystore;
use crate::state::{Backend, Slot};

const NOW: Timestamp = Timestamp::from_unix_millis(1_800_000_000_000);
const DAY: i64 = 86_400_000;
const STRONG: &str = "Vw9!rT2#kL5$pM8&nQ4*";
const NEW_PASSWORD: &str = "Xy7@mB3^dF6!hJ9#zC2$";
const PASSWORD: &str = "correct horse battery staple";

struct Env {
    backend: Backend,
    vault: VaultId,
    events: Log,
}

fn env() -> Env {
    let backend = Backend::new(
        Slot::Locked(LockedSession::open(Store::open_in_memory().unwrap())),
        Box::new(MemoryKeystore::default()),
        || Ok(Argon2Profile::Baseline),
    );
    let (channel, events) = recording_channel();
    backend.subscribe(channel).unwrap();
    backend
        .create_account(CreateAccountRequest {
            email: "alex@example.com".into(),
            password: SecretIn::test(PASSWORD),
            default_vault_name: "Personal".into(),
        })
        .unwrap();
    let vault = backend.list_vaults().unwrap()[0].vault_id.get();
    Env {
        backend,
        vault,
        events,
    }
}

fn add(e: &Env, kind: &ItemType, password: &str, url: Option<&str>) -> ItemId {
    let mut slot = e.backend.slot().unwrap();
    slot.unlocked_mut()
        .unwrap()
        .add_item(e.vault, kind, |edit| {
            edit.set_title("Sample")?;
            if kind == &ItemType::Login {
                edit.set_secret(SecretField::Password, password)?;
            }
            if let Some(url) = url {
                edit.add_url(&LoginUrl::new(url))?;
            }
            Ok(())
        })
        .unwrap()
}

fn login(e: &Env, password: &str) -> ItemId {
    add(e, &ItemType::Login, password, Some("https://example.com"))
}

fn item_req(e: &Env, item: ItemId) -> ItemRequest {
    ItemRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
    }
}

fn begin(e: &Env, item: ItemId, password: &str) -> Result<String, IpcError> {
    e.backend
        .rotation_begin(&RotationBeginRequest {
            vault_id: e.vault.into(),
            item_id: item.into(),
            new_password: SecretIn::test(password),
            rules_source: None,
        })
        .map(|b| b.attempt_id)
}

fn attempt(e: &Env, item: ItemId, id: &str) -> RotationAttemptRequest {
    RotationAttemptRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
        attempt_id: id.to_owned(),
    }
}

fn close(e: &Env, item: ItemId, id: &str, outcome: RotationOutcomeKey) -> Result<(), IpcError> {
    e.backend.rotation_close(&RotationCloseRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
        attempt_id: id.to_owned(),
        outcome,
    })
}

fn password_of(e: &Env, item: ItemId) -> String {
    let slot = e.backend.slot().unwrap();
    let item = slot.unlocked().unwrap().item(e.vault, item).unwrap();
    item.secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

fn code(error: IpcError) -> ErrorCode {
    error.code
}

#[test]
fn credential_rotation_begin_stores_pending_before_returning() {
    let e = env();
    let item = login(&e, STRONG);
    let id = begin(&e, item, NEW_PASSWORD).unwrap();

    // The attempt is in the item (and so in the sync queue) already; the
    // password has not moved.
    assert_eq!(password_of(&e, item), STRONG);
    let detail = e.backend.get_item(item_req(&e, item)).unwrap();
    assert_eq!(detail.rotation_attempts.len(), 1);
    assert_eq!(detail.rotation_attempts[0].attempt_id, id);
    assert_eq!(detail.rotation_attempts[0].stage, AttemptStage::Prepared);
    assert_eq!(
        detail.change_url.as_ref().unwrap().url,
        "https://example.com/.well-known/change-password"
    );
    let json = serde_json::to_string(&detail).unwrap();
    assert!(!json.contains(NEW_PASSWORD), "no value in the detail");

    let shown = e.backend.rotation_reveal(&attempt(&e, item, &id)).unwrap();
    assert_eq!(
        serde_json::to_string(&shown.value).unwrap(),
        format!("\"{NEW_PASSWORD}\"")
    );
}

#[test]
fn credential_rotation_close_succeeded_swaps_and_failed_keeps_current() {
    let e = env();
    let item = login(&e, STRONG);

    let failed = begin(&e, item, NEW_PASSWORD).unwrap();
    close(&e, item, &failed, RotationOutcomeKey::Failed).unwrap();
    assert_eq!(password_of(&e, item), STRONG, "a failed change keeps it");
    let detail = e.backend.get_item(item_req(&e, item)).unwrap();
    assert_eq!(detail.rotation_attempts.len(), 0);
    assert_eq!(detail.history.len(), 1);
    assert_eq!(detail.history[0].reason, HistoryReasonKey::Unconfirmed);
    // Nothing the attempt held is deleted.
    let secret = e
        .backend
        .reveal_secret(&crate::contract::items::RevealRequest {
            vault_id: e.vault.into(),
            item_id: item.into(),
            secret: crate::contract::items::SecretRef::History { index: 0 },
        })
        .unwrap();
    assert_eq!(
        serde_json::to_string(&secret.value).unwrap(),
        format!("\"{NEW_PASSWORD}\"")
    );

    let worked = begin(&e, item, "Qr5#nT8$wL2!vK7&bM4*").unwrap();
    close(&e, item, &worked, RotationOutcomeKey::Succeeded).unwrap();
    assert_eq!(password_of(&e, item), "Qr5#nT8$wL2!vK7&bM4*");
    let detail = e.backend.get_item(item_req(&e, item)).unwrap();
    assert_eq!(detail.history.len(), 2);
    assert_eq!(detail.history[0].reason, HistoryReasonKey::Replaced);

    assert_eq!(
        code(close(&e, item, &worked, RotationOutcomeKey::Succeeded).unwrap_err()),
        ErrorCode::RotationNotFound,
        "closed twice"
    );
}

#[test]
fn credential_rotation_history_restore_is_rollback() {
    let e = env();
    let item = login(&e, STRONG);
    let id = begin(&e, item, NEW_PASSWORD).unwrap();
    close(&e, item, &id, RotationOutcomeKey::Failed).unwrap();
    let history_id = e.backend.get_item(item_req(&e, item)).unwrap().history[0]
        .history_id
        .clone();

    e.backend
        .password_history_restore(&HistoryRestoreRequest {
            vault_id: e.vault.into(),
            item_id: item.into(),
            history_id: history_id.clone(),
        })
        .unwrap();
    assert_eq!(password_of(&e, item), NEW_PASSWORD);
    let detail = e.backend.get_item(item_req(&e, item)).unwrap();
    // The value that was the password is in the history now: a rollback of
    // the rollback is possible.
    assert_eq!(detail.history.len(), 1);
    assert_ne!(detail.history[0].history_id, history_id);

    let missing = e.backend.password_history_restore(&HistoryRestoreRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
        history_id: "0000000000000001.0123456789abcdef.9".into(),
    });
    assert_eq!(code(missing.unwrap_err()), ErrorCode::HistoryNotFound);
    let bad = e.backend.password_history_restore(&HistoryRestoreRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
        history_id: "x".repeat(200),
    });
    assert_eq!(code(bad.unwrap_err()), ErrorCode::InvalidInput);
}

#[test]
fn credential_rotation_begin_refusals() {
    let e = env();
    let note = add(&e, &ItemType::SecureNote, "", None);
    assert_eq!(
        code(begin(&e, note, NEW_PASSWORD).unwrap_err()),
        ErrorCode::RotationNotSupported
    );
    let plain_http = add(&e, &ItemType::Login, STRONG, Some("http://example.com"));
    assert_eq!(
        code(begin(&e, plain_http, NEW_PASSWORD).unwrap_err()),
        ErrorCode::RotationNotSupported,
        "no https URL"
    );
    let item = login(&e, STRONG);
    assert_eq!(
        code(begin(&e, item, "").unwrap_err()),
        ErrorCode::InvalidInput
    );
    assert_eq!(
        code(begin(&e, item, &"a".repeat(1025)).unwrap_err()),
        ErrorCode::InvalidInput
    );
    for n in 0..4 {
        begin(&e, item, &format!("{NEW_PASSWORD}{n}")).unwrap();
    }
    assert_eq!(
        code(begin(&e, item, "one more password!").unwrap_err()),
        ErrorCode::RotationLimit
    );
    let unknown = ItemRequest {
        item_id: ItemId::from_bytes([9; 16]).unwrap().into(),
        ..item_req(&e, item)
    };
    let missing = e.backend.rotation_begin(&RotationBeginRequest {
        vault_id: unknown.vault_id,
        item_id: unknown.item_id,
        new_password: SecretIn::test(NEW_PASSWORD),
        rules_source: None,
    });
    assert_eq!(code(missing.unwrap_err()), ErrorCode::ItemNotFound);
}

#[test]
fn credential_rotation_queue_snooze_dismiss_and_policy() {
    let e = env();
    let weak = login(&e, "Sommer2024!");
    let fine = login(&e, STRONG);
    let manual = add(&e, &ItemType::Login, "Sommer2023!", None);

    let queue = e.backend.rotation_queue(NOW).unwrap();
    let ids: Vec<_> = queue.entries.iter().map(|x| x.item_id.get()).collect();
    assert!(ids.contains(&weak) && ids.contains(&manual) && !ids.contains(&fine));
    let weak_entry = queue
        .entries
        .iter()
        .find(|x| x.item_id.get() == weak)
        .unwrap();
    assert_eq!(weak_entry.kind, QueueKind::Guided);
    assert!(weak_entry.reasons.contains(&QueueReason::Weak));
    let manual_entry = queue
        .entries
        .iter()
        .find(|x| x.item_id.get() == manual)
        .unwrap();
    assert_eq!(manual_entry.kind, QueueKind::Manual, "no https URL");
    assert_eq!(queue.breach.mode, BreachMode::Off);
    assert!(!queue.breach.running);

    // An open attempt lists a strong password too, as unfinished.
    let id = begin(&e, fine, NEW_PASSWORD).unwrap();
    let queue = e.backend.rotation_queue(NOW).unwrap();
    let entry = queue
        .entries
        .iter()
        .find(|x| x.item_id.get() == fine)
        .unwrap();
    assert_eq!(entry.reasons, vec![QueueReason::Unfinished]);
    assert_eq!(entry.attempts[0].attempt_id, id);

    // Snooze keeps it listed, with the time; the days are 1, 7 or 30.
    e.backend
        .rotation_snooze(
            &RotationSnoozeRequest {
                vault_id: e.vault.into(),
                item_id: weak.into(),
                days: 7,
            },
            NOW,
        )
        .unwrap();
    let queue = e.backend.rotation_queue(NOW).unwrap();
    let entry = queue
        .entries
        .iter()
        .find(|x| x.item_id.get() == weak)
        .unwrap();
    assert_eq!(entry.snoozed_until, Some(NOW.as_unix_millis() + 7 * DAY));
    let later = Timestamp::from_unix_millis(NOW.as_unix_millis() + 8 * DAY);
    let queue = e.backend.rotation_queue(later).unwrap();
    let entry = queue
        .entries
        .iter()
        .find(|x| x.item_id.get() == weak)
        .unwrap();
    assert_eq!(entry.snoozed_until, None, "the snooze ran out");
    assert_eq!(
        code(
            e.backend
                .rotation_snooze(
                    &RotationSnoozeRequest {
                        vault_id: e.vault.into(),
                        item_id: weak.into(),
                        days: 3,
                    },
                    NOW
                )
                .unwrap_err()
        ),
        ErrorCode::InvalidInput
    );

    // Dismiss hides it until the password changes (and not an open change); Undo brings it back.
    e.backend.rotation_dismiss(item_req(&e, weak)).unwrap();
    e.backend.rotation_dismiss(item_req(&e, fine)).unwrap();
    e.backend.rotation_undismiss(item_req(&e, fine)).unwrap();
    e.backend.rotation_dismiss(item_req(&e, fine)).unwrap();
    let queue = e.backend.rotation_queue(NOW).unwrap();
    let ids: Vec<_> = queue.entries.iter().map(|x| x.item_id.get()).collect();
    assert!(!ids.contains(&weak));
    assert!(
        ids.contains(&fine),
        "an open change is never dismissed away"
    );

    // The reminder interval: 30, 90, 180, 365 or none.
    for days in [Some(90), None] {
        e.backend
            .rotation_set_policy(&RotationPolicyRequest {
                vault_id: e.vault.into(),
                item_id: weak.into(),
                every_days: days,
            })
            .unwrap();
        assert_eq!(
            e.backend
                .get_item(item_req(&e, weak))
                .unwrap()
                .rotation_policy,
            days
        );
    }
    assert_eq!(
        code(
            e.backend
                .rotation_set_policy(&RotationPolicyRequest {
                    vault_id: e.vault.into(),
                    item_id: weak.into(),
                    every_days: Some(45),
                })
                .unwrap_err()
        ),
        ErrorCode::InvalidInput
    );
}

#[test]
fn credential_rotation_every_command_is_locked() {
    let e = env();
    let item = login(&e, STRONG);
    // A real attempt, so that only the lock can be the reason for a refusal.
    let id = begin(&e, item, NEW_PASSWORD).unwrap();
    let id = id.as_str();
    e.backend.lock(LockReason::Manual).unwrap();
    let locked = |r: Result<(), IpcError>| assert_eq!(code(r.unwrap_err()), ErrorCode::Locked);
    locked(e.backend.rotation_queue(NOW).map(|_| ()));
    locked(begin(&e, item, NEW_PASSWORD).map(|_| ()));
    locked(close(&e, item, id, RotationOutcomeKey::Failed));
    locked(
        e.backend
            .rotation_reveal(&attempt(&e, item, id))
            .map(|_| ()),
    );
    locked(e.backend.rotation_copy(&attempt(&e, item, id)).map(|_| ()));
    locked(e.backend.password_history_restore(&HistoryRestoreRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
        history_id: id.into(),
    }));
    locked(e.backend.rotation_dismiss(item_req(&e, item)));
    locked(e.backend.rotation_undismiss(item_req(&e, item)));
    locked(e.backend.rotation_snooze(
        &RotationSnoozeRequest {
            vault_id: e.vault.into(),
            item_id: item.into(),
            days: 1,
        },
        NOW,
    ));
    locked(e.backend.rotation_set_policy(&RotationPolicyRequest {
        vault_id: e.vault.into(),
        item_id: item.into(),
        every_days: None,
    }));
    locked(e.backend.breach_scan_cancel());
    locked(e.backend.breach_scan_start(&Weak::new()));
}

// ---------------------------------------------------------------- breach check

/// A range source that answers from a table and records every request.
struct FakeSource {
    seen: Arc<Mutex<Vec<String>>>,
    answers: Vec<(&'static str, &'static str)>,
    error: Option<RangeError>,
}

impl RangeSource for FakeSource {
    fn range(&mut self, prefix: &RangePrefix) -> Result<Zeroizing<Vec<u8>>, RangeError> {
        self.seen.lock().unwrap().push(prefix.as_str().to_owned());
        if let Some(error) = self.error {
            return Err(error);
        }
        let body = self
            .answers
            .iter()
            .find(|(p, _)| *p == prefix.as_str())
            .map_or("0000000000000000000000000000000000A:0\r\n", |(_, b)| *b);
        Ok(Zeroizing::new(body.as_bytes().to_vec()))
    }
}

/// SHA-1("password") = 5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8.
const PASSWORD_ANSWER: (&str, &str) = (
    "5BAA6",
    "1E4C9B93F3F0682250B6CF8331B7EE68FD8:3861493\r\n0018A45C4D1DEF81644B54AB7F969B88D65:0\r\n",
);

fn enable_breach(e: &Env, mode: BreachMode) {
    e.backend
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                breach_check: mode,
                breach_consent_at: Some(NOW.as_unix_millis()),
                ..Settings::default()
            },
            password: None,
        })
        .unwrap();
}

fn panicking_source(e: &Env) -> Arc<AtomicU32> {
    let calls = Arc::new(AtomicU32::new(0));
    let seen = Arc::clone(&calls);
    e.backend.breach.set_source(Box::new(move || {
        seen.fetch_add(1, Ordering::SeqCst);
        panic!("the client must not be built");
    }));
    calls
}

#[test]
fn credential_rotation_breach_scan_refused_without_consent() {
    let e = env();
    login(&e, "password");
    let calls = panicking_source(&e);
    assert_eq!(
        code(e.backend.breach_scan_start(&Weak::new()).unwrap_err()),
        ErrorCode::BreachConsentRequired
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(!e.backend.breach.is_running());
}

#[test]
fn no_network_request_without_opt_in() {
    let e = env();
    login(&e, "password");
    let calls = panicking_source(&e);
    // `off`: never due, never started.
    assert!(!e.backend.breach_weekly_due(NOW));
    assert!(e.backend.breach_scan_start(&Weak::new()).is_err());
    // `manual`: only "Check now" (not the weekly timer).
    enable_breach(&e, BreachMode::Manual);
    assert!(!e.backend.breach_weekly_due(NOW));
    assert_eq!(calls.load(Ordering::SeqCst), 0, "nothing built a client");

    // `weekly`: due when no scan ran, not for a week after one.
    enable_breach(&e, BreachMode::Weekly);
    assert!(e.backend.breach_weekly_due(NOW));
    let job = e.backend.breach_scan_prepare(NOW).unwrap();
    assert!(e.backend.breach.is_running());
    assert!(!e.backend.breach_weekly_due(NOW), "one scan at a time");
    assert_eq!(
        code(e.backend.breach_scan_prepare(NOW).unwrap_err()),
        ErrorCode::BreachRunning
    );
    let mut source = FakeSource {
        seen: Arc::default(),
        answers: vec![PASSWORD_ANSWER],
        error: None,
    };
    e.backend.breach_scan_run(job, &mut source, &mut |_| true);
    assert!(!e.backend.breach.is_running());
    let after = |days: i64| Timestamp::from_unix_millis(NOW.as_unix_millis() + days * DAY);
    assert!(
        !e.backend.breach_weekly_due(after(1)),
        "a scan ran a day ago"
    );
    assert!(e.backend.breach_weekly_due(after(8)), "a week passed");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn credential_rotation_breach_scan_records_findings_and_sends_only_prefixes() {
    let e = env();
    let bad = login(&e, "password");
    let good = login(&e, STRONG);
    enable_breach(&e, BreachMode::Manual);

    let job = e.backend.breach_scan_prepare(NOW).unwrap();
    assert_eq!(job.prefix_count(), 2);
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut source = FakeSource {
        seen: Arc::clone(&seen),
        answers: vec![PASSWORD_ANSWER],
        error: None,
    };
    let mut pauses = 0;
    e.backend.breach_scan_run(job, &mut source, &mut |_| {
        pauses += 1;
        true
    });
    assert_eq!(pauses, 1, "one pause between the two requests");
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.len(), 2);
    assert!(
        seen.iter().all(|p| p.len() == 5),
        "only 5-character prefixes"
    );
    assert!(seen.contains(&"5BAA6".to_owned()));

    // The finding shows in the dashboard report and the queue; the other item is clean.
    let report = e.backend.security_report(NOW).unwrap();
    assert_eq!(report.counts.breached, 1);
    let queue = e.backend.rotation_queue(NOW).unwrap();
    let entry = queue
        .entries
        .iter()
        .find(|x| x.item_id.get() == bad)
        .unwrap();
    assert!(entry.reasons.contains(&QueueReason::Breached));
    assert!(queue.entries.iter().all(|x| x.item_id.get() != good));
    assert!(queue.breach.last_finished_at.is_some());
    assert_eq!(queue.breach.last_error, None);

    let events = e.events.lock().unwrap().join("\n");
    assert!(events.contains("breachScanProgress"));
    assert!(events.contains(r#""type":"breachScanFinished","checked":2,"found":1,"error":null"#));

    // Changing the password clears the finding: the cached row is for the old value.
    let id = begin(&e, bad, NEW_PASSWORD).unwrap();
    close(&e, bad, &id, RotationOutcomeKey::Succeeded).unwrap();
    assert_eq!(e.backend.security_report(NOW).unwrap().counts.breached, 0);
    // Nothing of any password, hash or prefix is on the wire.
    assert!(!events.contains("password") && !events.contains("5BAA6"));
}

#[test]
fn credential_rotation_offline_scan_keeps_the_error_and_nothing_else() {
    let e = env();
    login(&e, "password");
    enable_breach(&e, BreachMode::Manual);
    let job = e.backend.breach_scan_prepare(NOW).unwrap();
    let mut source = FakeSource {
        seen: Arc::default(),
        answers: vec![],
        error: Some(RangeError::Offline),
    };
    e.backend.breach_scan_run(job, &mut source, &mut |_| true);
    let queue = e.backend.rotation_queue(NOW).unwrap();
    assert_eq!(queue.breach.last_error, Some(BreachScanError::Offline));
    assert!(!queue.breach.running);
    assert_eq!(e.backend.security_report(NOW).unwrap().counts.breached, 0);
}

#[test]
fn breach_scan_stops_on_lock() {
    let e = env();
    login(&e, "password");
    login(&e, STRONG);
    login(&e, "Another-Pa55word#1");
    enable_breach(&e, BreachMode::Manual);
    let job = e.backend.breach_scan_prepare(NOW).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut source = FakeSource {
        seen: Arc::clone(&seen),
        answers: vec![PASSWORD_ANSWER],
        error: None,
    };
    // The vault locks during the first pause: the scan must not send another request.
    e.backend.breach_scan_run(job, &mut source, &mut |_| {
        e.backend.lock(LockReason::Idle).unwrap();
        false
    });
    assert_eq!(seen.lock().unwrap().len(), 1, "no request after the lock");
    assert!(!e.backend.breach.is_running());
    let events = e.events.lock().unwrap().join("\n");
    assert!(events.contains(r#""type":"breachScanFinished""#));
    assert!(events.contains(r#""error":"cancelled""#));
    // Nothing was stored while locked: after the next unlock there is no finding.
    e.backend
        .unlock(crate::contract::session::UnlockRequest {
            password: SecretIn::test(PASSWORD),
            secret_key: None,
        })
        .unwrap();
    assert_eq!(e.backend.security_report(NOW).unwrap().counts.breached, 0);
}

#[test]
fn credential_rotation_cancel_stops_a_running_scan() {
    let e = env();
    login(&e, "password");
    login(&e, STRONG);
    enable_breach(&e, BreachMode::Manual);
    let job = e.backend.breach_scan_prepare(NOW).unwrap();
    let mut source = FakeSource {
        seen: Arc::default(),
        answers: vec![PASSWORD_ANSWER],
        error: None,
    };
    e.backend.breach_scan_run(job, &mut source, &mut |_| {
        e.backend.breach_scan_cancel().unwrap();
        false
    });
    let queue = e.backend.rotation_queue(NOW).unwrap();
    assert_eq!(queue.breach.last_error, Some(BreachScanError::Cancelled));
    // The answer of the first prefix is kept: a cancelled scan loses nothing it finished.
}

#[test]
fn breach_setting_without_mac_resets_to_off() {
    use crate::persist::{StoredState, canonical_settings, parse, render};
    let state = StoredState {
        settings: Settings {
            breach_check: BreachMode::Weekly,
            breach_consent_at: Some(NOW.as_unix_millis()),
            ..Settings::default()
        },
        ..StoredState::default()
    };
    // The file is untrusted until the MAC is checked: read as `off`.
    let file = render(&state, None).unwrap();
    let read = parse(&file).unwrap();
    assert_eq!(read.settings.breach_check, BreachMode::Off);
    assert_eq!(read.settings.breach_consent_at, None);
    // The MAC covers the setting, and an install that never used it keeps its old MAC input.
    let with = canonical_settings(&state);
    let without = canonical_settings(&StoredState::default());
    assert!(
        String::from_utf8(with)
            .unwrap()
            .contains("\"breachCheck\":\"weekly\"")
    );
    assert!(!String::from_utf8(without).unwrap().contains("breach"));
}

#[test]
fn credential_rotation_breach_settings_pair_mode_and_consent() {
    let e = env();
    // A mode without consent reads as off; turning it off withdraws the consent.
    let stored = e
        .backend
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                breach_check: BreachMode::Weekly,
                breach_consent_at: None,
                ..Settings::default()
            },
            password: None,
        })
        .unwrap();
    assert_eq!(stored.breach_check, BreachMode::Off);
    enable_breach(&e, BreachMode::Manual);
    assert_eq!(
        e.backend.settings_snapshot().breach_check,
        BreachMode::Manual
    );
    let off = e
        .backend
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                breach_check: BreachMode::Off,
                breach_consent_at: Some(1),
                ..Settings::default()
            },
            password: None,
        })
        .unwrap();
    assert_eq!(off.breach_consent_at, None);
}

// ------------------------------------------------ review follow-ups (M1, L1, L2, I1)

/// A source whose request takes `hold`: it counts the requests that started and says when one
/// returned.
struct SlowSource {
    entered: Arc<AtomicU32>,
    returned: Arc<AtomicBool>,
    hold: std::time::Duration,
}

impl RangeSource for SlowSource {
    fn range(&mut self, _prefix: &RangePrefix) -> Result<Zeroizing<Vec<u8>>, RangeError> {
        self.entered.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.hold);
        self.returned.store(true, Ordering::SeqCst);
        Ok(Zeroizing::new(
            b"0000000000000000000000000000000000A:0\r\n".to_vec(),
        ))
    }
}

fn slow_source(backend: &Backend, hold_ms: u64) -> (Arc<AtomicU32>, Arc<AtomicBool>) {
    let entered = Arc::new(AtomicU32::new(0));
    let returned = Arc::new(AtomicBool::new(false));
    let (e, r) = (Arc::clone(&entered), Arc::clone(&returned));
    backend.breach.set_source(Box::new(move || {
        Ok(Box::new(SlowSource {
            entered: Arc::clone(&e),
            returned: Arc::clone(&r),
            hold: std::time::Duration::from_millis(hold_ms),
        }))
    }));
    (entered, returned)
}

fn wait_for(what: &str, mut done: impl FnMut() -> bool) {
    for _ in 0..400 {
        if done() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("timed out waiting for: {what}");
}

#[test]
fn turning_the_breach_check_off_stops_a_running_scan() {
    let e = env();
    login(&e, "password");
    login(&e, STRONG);
    enable_breach(&e, BreachMode::Manual);
    let backend = Arc::new(e.backend);
    let (entered, _returned) = slow_source(&backend, 400);
    backend
        .breach_scan_start(&Arc::downgrade(&backend))
        .unwrap();
    wait_for("the first request", || entered.load(Ordering::SeqCst) >= 1);
    // The user turns the check off while the first request is on the wire.
    backend
        .update_settings(UpdateSettingsRequest {
            settings: Settings {
                breach_check: BreachMode::Off,
                ..Settings::default()
            },
            password: None,
        })
        .unwrap();
    wait_for("the scan to end", || !backend.breach.is_running());
    // Give a (wrong) second request time to start.
    std::thread::sleep(std::time::Duration::from_millis(600));
    assert_eq!(entered.load(Ordering::SeqCst), 1, "no request after off");
}

#[test]
fn lock_ends_a_scan_while_its_request_is_on_the_wire() {
    let e = env();
    login(&e, "password");
    login(&e, STRONG);
    enable_breach(&e, BreachMode::Manual);
    let backend = Arc::new(e.backend);
    let (entered, returned) = slow_source(&backend, 2_500);
    backend
        .breach_scan_start(&Arc::downgrade(&backend))
        .unwrap();
    wait_for("the first request", || entered.load(Ordering::SeqCst) >= 1);
    backend.lock(LockReason::Idle).unwrap();
    // The scan ends, and with it the plan, long before the request returns.
    wait_for("the scan to end", || !backend.breach.is_running());
    assert!(
        !returned.load(Ordering::SeqCst),
        "the scan did not wait for the transport"
    );
    assert_eq!(entered.load(Ordering::SeqCst), 1);
}

#[test]
fn a_failing_random_source_ends_the_scan_before_any_request() {
    let e = env();
    login(&e, "password");
    login(&e, STRONG);
    enable_breach(&e, BreachMode::Manual);
    let job = e.backend.breach_scan_prepare(NOW).unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut source = FakeSource {
        seen: Arc::clone(&seen),
        answers: vec![PASSWORD_ANSWER],
        error: None,
    };
    e.backend
        .breach_scan_run_with(job, &mut source, &mut |_| true, &mut |_| None);
    assert!(seen.lock().unwrap().is_empty(), "a fixed order is not used");
    assert!(!e.backend.breach.is_running());
    let events = e.events.lock().unwrap().join("\n");
    assert!(events.contains(r#""type":"breachScanFinished""#));
    assert!(events.contains(r#""error":"protocol""#), "{events}");
}

#[test]
fn delete_forever_is_refused_while_a_change_is_open() {
    let e = env();
    let item = login(&e, STRONG);
    let id = begin(&e, item, NEW_PASSWORD).unwrap();
    assert_eq!(
        code(
            e.backend
                .edit_item(item_req(&e, item), UnlockedSession::purge_item)
                .unwrap_err()
        ),
        ErrorCode::RotationOpen
    );
    // The item and its attempt are untouched; the trash is allowed; a closed attempt lifts it.
    assert_eq!(
        e.backend
            .get_item(item_req(&e, item))
            .unwrap()
            .rotation_attempts
            .len(),
        1
    );
    close(&e, item, &id, RotationOutcomeKey::Failed).unwrap();
    e.backend
        .edit_item(item_req(&e, item), UnlockedSession::purge_item)
        .unwrap();
}
