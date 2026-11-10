//! Tests of credential rotation in the session (M7b, CRYPTO_SPEC §7.4, §11):
//! attempts, their close, the rollback, the exact-origin lookup, the breach
//! cache, the queue's snooze and dismissal, and what the store keeps.

#![expect(
    clippy::many_single_char_names,
    reason = "accounts a, b, c and the vault and item IDs read best as one letter"
)]

use mosaic_core::model::{
    HistoryReason, ItemType, LoginUrl, RotationStage, RulesSource, SecretField, UrlMatch,
};
use mosaic_core::{Argon2Profile, ItemId, SecretString, Timestamp, VaultId};
use mosaic_insights::ItemKey;
use mosaic_insights::breach::{BreachField, BreachResult};
use mosaic_origin::PageOrigin;

use super::sharing_tests::{pw, trio, user};
use super::{
    Error, ItemFilter, LockedSession, QueueStates, RotationOutcome, ScanError, ScanSummary,
    UnlockedSession,
};
use crate::storage::{PendingPolicy, Store};

fn secret(s: &str) -> SecretString {
    SecretString::from_str_copy(s).unwrap()
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

fn origin(s: &str) -> PageOrigin {
    PageOrigin::parse(s).unwrap()
}

/// A login with a password and one URL.
fn login(s: &mut UnlockedSession, v: VaultId, url: &str, password: &str) -> ItemId {
    s.add_item(v, &ItemType::Login, |e| {
        e.set_title("Bank")?;
        e.set_secret(SecretField::Password, password)?;
        e.add_url(&LoginUrl::new(url))?;
        Ok(())
    })
    .unwrap()
}

fn password_of(s: &UnlockedSession, v: VaultId, i: ItemId) -> String {
    s.item(v, i)
        .unwrap()
        .secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

fn key(v: VaultId, i: ItemId) -> ItemKey {
    ItemKey {
        vault_id: v,
        item_id: i,
    }
}

fn mark_of(s: &UnlockedSession, v: VaultId, i: ItemId) -> mosaic_core::model::ValueMark {
    s.item(v, i)
        .unwrap()
        .secret_mark(SecretField::Password)
        .unwrap()
}

fn result_for(s: &UnlockedSession, v: VaultId, i: ItemId, found: bool) -> BreachResult {
    BreachResult {
        key: key(v, i),
        field: BreachField::LoginPassword,
        mark: mark_of(s, v, i),
        checked_at: at(1_000),
        found,
    }
}

fn scan() -> ScanSummary {
    ScanSummary {
        started_at: at(900),
        finished_at: Some(at(1_100)),
        checked: 1,
        error: None,
    }
}

fn change_password(s: &mut UnlockedSession, v: VaultId, i: ItemId, to: &str) {
    s.update_item(v, i, |e| e.set_secret(SecretField::Password, to))
        .unwrap();
}

#[test]
fn begin_commits_before_returning() {
    let (mut s, sk) = {
        let (s, c) = LockedSession::open(Store::open_in_memory().unwrap())
            .create_account(&pw(), Argon2Profile::Baseline)
            .unwrap();
        (s, c.secret_key)
    };
    let v = s.create_vault("V").unwrap();
    let i = login(&mut s, v, "https://bank.example", "old-password");
    let before = s.store.record(&v, &i).unwrap().unwrap();
    let id = s
        .begin_rotation(v, i, &secret("new-password"), Some(RulesSource::Page))
        .unwrap();
    // The sealed record changed and waits in the outbox.
    let after = s.store.record(&v, &i).unwrap().unwrap();
    assert_ne!(before.body.as_bytes(), after.body.as_bytes());
    assert!(after.outbox_seq.is_some());
    // The password is untouched; the attempt is in the item.
    assert_eq!(password_of(&s, v, i), "old-password");
    let attempts = s.item(v, i).unwrap().rotation_attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].id, id);
    assert_eq!(attempts[0].stage, RotationStage::Prepared);
    // A lock and an unlock (a crash after the fill) lose nothing: the value
    // is in the stored record.
    let s = LockedSession::open(s.lock().into_store())
        .unlock(&pw(), &sk)
        .unwrap();
    let attempts = s.item(v, i).unwrap().rotation_attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].pending.expose_secret(), "new-password");
    assert_eq!(password_of(&s, v, i), "old-password");
}

#[test]
fn begin_refused_for_viewer_and_non_login() {
    let (mut a, mut b, mut c, v) = trio();
    let i = login(&mut a.s, v, "https://bank.example", "old-password");
    let rec = a.s.store.record(&v, &i).unwrap().unwrap();
    for u in [&mut b, &mut c] {
        u.s.apply_remote_record(&super::RemoteRecord {
            vault_id: v,
            record_id: i,
            body: rec.body.clone(),
            server_seq: 1,
        })
        .unwrap();
    }
    let n = secret("new-password");
    // The viewer is refused for every item change, and nothing is written.
    let before = c.s.store.record(&v, &i).unwrap().unwrap();
    assert!(matches!(
        c.s.begin_rotation(v, i, &n, None),
        Err(Error::ReadOnly)
    ));
    let id = b.s.begin_rotation(v, i, &n, None).unwrap();
    assert!(matches!(
        c.s.mark_rotation_delivered(v, i, id),
        Err(Error::ReadOnly)
    ));
    assert!(matches!(
        c.s.close_rotation(v, i, id, RotationOutcome::Succeeded),
        Err(Error::ReadOnly)
    ));
    assert!(matches!(
        c.s.set_rotation_policy(v, i, Some(90)),
        Err(Error::ReadOnly)
    ));
    let after = c.s.store.record(&v, &i).unwrap().unwrap();
    assert_eq!(before.body.as_bytes(), after.body.as_bytes());
    assert!(c.s.item(v, i).unwrap().rotation_attempts().is_empty());
    // A writer may.
    assert_eq!(b.s.item(v, i).unwrap().rotation_attempts().len(), 1);

    // Not a login: the core refuses.
    let note =
        a.s.add_item(v, &ItemType::SecureNote, |e| e.set_title("n"))
            .unwrap();
    assert!(matches!(
        a.s.begin_rotation(v, note, &n, None),
        Err(Error::Core(mosaic_core::Error::NotALogin))
    ));
    // Unknown item and an empty value.
    assert!(matches!(
        a.s.begin_rotation(v, ItemId::new_v7().unwrap(), &n, None),
        Err(Error::UnknownItem)
    ));
    assert!(matches!(
        a.s.begin_rotation(v, i, &secret(""), None),
        Err(Error::Core(mosaic_core::Error::Malformed))
    ));
}

#[test]
fn purge_is_refused_while_an_attempt_is_open_and_the_trash_is_not() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "old-password");
    let id = s
        .begin_rotation(v, i, &secret("new-password"), None)
        .unwrap();
    assert!(matches!(
        s.purge_item(v, i),
        Err(Error::Core(mosaic_core::Error::RotationOpen))
    ));
    // Nothing changed: the attempt is still open and the item is live.
    assert_eq!(s.item(v, i).unwrap().rotation_attempts().len(), 1);
    // The trash stays allowed, and so does moving it to another vault.
    s.trash_item(v, i).unwrap();
    s.restore_item(v, i).unwrap();
    let other = s.create_vault("W").unwrap();
    let moved = s.move_item(v, i, other).unwrap();
    assert_eq!(s.item(other, moved).unwrap().rotation_attempts().len(), 1);
    // Closing the attempt lifts the refusal.
    let attempt = s.item(other, moved).unwrap().rotation_attempts()[0].id;
    assert_eq!(attempt, id);
    s.close_rotation(other, moved, attempt, RotationOutcome::Failed)
        .unwrap();
    s.purge_item(other, moved).unwrap();
}

#[test]
fn add_origin_during_rotation_needs_a_writer_and_an_open_attempt() {
    let (mut a, mut b, mut c, v) = trio();
    let i = login(&mut a.s, v, "https://bank.example", "old-password");
    let rec = a.s.store.record(&v, &i).unwrap().unwrap();
    for u in [&mut b, &mut c] {
        u.s.apply_remote_record(&super::RemoteRecord {
            vault_id: v,
            record_id: i,
            body: rec.body.clone(),
            server_seq: 1,
        })
        .unwrap();
    }
    let id =
        b.s.begin_rotation(v, i, &secret("new-password"), None)
            .unwrap();
    let rec = b.s.store.record(&v, &i).unwrap().unwrap();
    c.s.apply_remote_record(&super::RemoteRecord {
        vault_id: v,
        record_id: i,
        body: rec.body,
        server_seq: 2,
    })
    .unwrap();
    let url = LoginUrl::new("https://login.bank.example");
    // The viewer is refused and nothing is written.
    let before = c.s.store.record(&v, &i).unwrap().unwrap();
    assert!(matches!(
        c.s.add_origin_during_rotation(v, i, id, &url),
        Err(Error::ReadOnly)
    ));
    let after = c.s.store.record(&v, &i).unwrap().unwrap();
    assert_eq!(before.body.as_bytes(), after.body.as_bytes());
    // The writer may, while the attempt is open and not after it closed.
    b.s.add_origin_during_rotation(v, i, id, &url).unwrap();
    assert_eq!(b.s.item(v, i).unwrap().urls().len(), 2);
    b.s.close_rotation(v, i, id, RotationOutcome::Failed)
        .unwrap();
    assert!(matches!(
        b.s.add_origin_during_rotation(v, i, id, &LoginUrl::new("https://x.bank.example")),
        Err(Error::Core(mosaic_core::Error::RotationNotFound))
    ));
}

#[test]
fn close_succeeded_syncs_as_one_edit() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "old-password");
    let first = s.begin_rotation(v, i, &secret("new-one"), None).unwrap();
    let second = s.begin_rotation(v, i, &secret("new-two"), None).unwrap();
    s.mark_rotation_delivered(v, i, first).unwrap();
    // Marking twice is a no-op, not a second write.
    let seq = s.store.record(&v, &i).unwrap().unwrap().outbox_seq.unwrap();
    s.mark_rotation_delivered(v, i, first).unwrap();
    assert_eq!(
        s.store.record(&v, &i).unwrap().unwrap().outbox_seq.unwrap(),
        seq
    );

    s.close_rotation(v, i, first, RotationOutcome::Succeeded)
        .unwrap();
    let after = s.store.record(&v, &i).unwrap().unwrap().outbox_seq.unwrap();
    assert!(after > seq, "the close is one more queued write");
    assert_eq!(password_of(s, v, i), "new-one");
    let item = s.item(v, i).unwrap();
    assert!(item.rotation_attempts().is_empty());
    // The old password and the other attempt's value are in the history;
    // nothing that was ever a password is gone.
    let history = item.password_history();
    let held = |value: &str, reason: HistoryReason| {
        history
            .iter()
            .any(|h| h.value.expose_secret() == value && h.reason == reason)
    };
    assert!(held("old-password", HistoryReason::Replaced));
    assert!(held("new-two", HistoryReason::Unconfirmed));
    // The other attempt is gone, so closing it now is refused.
    assert!(matches!(
        s.close_rotation(v, i, second, RotationOutcome::Failed),
        Err(Error::Core(mosaic_core::Error::RotationNotFound))
    ));
}

#[test]
fn close_failed_keeps_the_password_and_restore_is_the_rollback() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "old-password");
    let id = s.begin_rotation(v, i, &secret("new-one"), None).unwrap();
    s.close_rotation(v, i, id, RotationOutcome::Failed).unwrap();
    assert_eq!(password_of(s, v, i), "old-password");
    let entry = s
        .item(v, i)
        .unwrap()
        .password_history()
        .into_iter()
        .find(|h| h.value.expose_secret() == "new-one")
        .unwrap();
    assert_eq!(entry.reason, HistoryReason::Unconfirmed);
    // The site applied it after all: restore it.
    s.restore_password_history(v, i, &entry.id).unwrap();
    assert_eq!(password_of(s, v, i), "new-one");
    assert!(
        s.item(v, i)
            .unwrap()
            .password_history()
            .iter()
            .any(|h| h.value.expose_secret() == "old-password")
    );
    assert!(matches!(
        s.restore_password_history(v, i, &entry.id),
        Err(Error::Core(mosaic_core::Error::HistoryNotFound))
    ));
}

#[test]
fn attempts_for_origin_exact_only() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let bank = login(s, v, "https://bank.example", "p1");
    s.begin_rotation(v, bank, &secret("n1"), None).unwrap();
    // An item saved with eTLD+1 matching: a subdomain page is still no match.
    let shop = s
        .add_item(v, &ItemType::Login, |e| {
            let mut url = LoginUrl::new("https://shop.example");
            url.match_mode = UrlMatch::Etld1;
            e.add_url(&url)?;
            Ok(())
        })
        .unwrap();
    s.begin_rotation(v, shop, &secret("n2"), None).unwrap();
    // A login without an attempt never shows.
    login(s, v, "https://bank.example", "p3");

    let hit = s.rotation_attempts_for_origin(&origin("https://bank.example"));
    assert_eq!(hit.len(), 1);
    assert_eq!((hit[0].vault_id, hit[0].item_id), (v, bank));
    assert_eq!(hit[0].title.as_deref(), Some("Bank"));
    assert_eq!(hit[0].stage, RotationStage::Prepared);
    for miss in [
        "https://login.bank.example",
        "https://bank.example:8443",
        "http://bank.example",
        "https://bank.example.evil.test",
        "https://bamk.example",
        "https://pay.shop.example",
    ] {
        assert!(
            s.rotation_attempts_for_origin(&origin(miss)).is_empty(),
            "{miss}"
        );
    }
    // The saved origin itself matches an item saved as eTLD+1.
    assert_eq!(
        s.rotation_attempts_for_origin(&origin("https://shop.example"))
            .len(),
        1
    );
    // An item in the trash is skipped.
    s.trash_item(v, bank).unwrap();
    assert_eq!(
        s.rotation_attempts_for_origin(&origin("https://bank.example"))
            .len(),
        0
    );
}

#[test]
fn attempts_for_origin_returns_at_most_five() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    for _ in 0..2 {
        let i = login(s, v, "https://bank.example", "p");
        for n in 0..4 {
            s.begin_rotation(v, i, &secret(&format!("n{n}")), None)
                .unwrap();
        }
    }
    let hit = s.rotation_attempts_for_origin(&origin("https://bank.example"));
    assert_eq!(hit.len(), 5);
    assert!(hit.windows(2).all(|w| w[0].started_at >= w[1].started_at));
}

#[test]
fn breach_rows_stale_after_password_change() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "p1");
    let r = result_for(s, v, i, true);
    s.record_breach_results(&[r], scan()).unwrap();
    let states = s.breach_states().unwrap();
    let row = states.get(key(v, i), BreachField::LoginPassword).unwrap();
    assert!(row.found);
    assert_eq!(row.checked_at, at(1_000));
    assert!(
        states
            .breached_items(s.items(&ItemFilter::default()).iter().map(|r| {
                mosaic_insights::Candidate {
                    vault_id: r.vault_id,
                    item_id: r.item_id,
                    item: r.item,
                }
            }))
            .contains(&key(v, i))
    );
    let summary = s.breach_scan().unwrap().unwrap();
    assert_eq!(summary, scan());

    // The stale answer is not recorded, and the old one stops counting.
    change_password(s, v, i, "p2");
    assert!(
        s.breach_states()
            .unwrap()
            .get(key(v, i), BreachField::LoginPassword)
            .is_none()
    );
    s.record_breach_results(&[r], scan()).unwrap();
    assert!(
        s.store.breach_rows().unwrap().len() == 1,
        "the old row stays, unused"
    );
    assert!(
        s.breach_states()
            .unwrap()
            .get(key(v, i), BreachField::LoginPassword)
            .is_none()
    );
    // A fresh answer replaces the row.
    let fresh = result_for(s, v, i, false);
    s.record_breach_results(&[fresh], scan()).unwrap();
    let states = s.breach_states().unwrap();
    assert!(
        !states
            .get(key(v, i), BreachField::LoginPassword)
            .unwrap()
            .found
    );
    assert_eq!(s.store.breach_rows().unwrap().len(), 1);

    // A confirmed rotation changes the mark too.
    let id = s.begin_rotation(v, i, &secret("p3"), None).unwrap();
    s.close_rotation(v, i, id, RotationOutcome::Succeeded)
        .unwrap();
    assert!(
        s.breach_states()
            .unwrap()
            .get(key(v, i), BreachField::LoginPassword)
            .is_none()
    );
}

#[test]
fn scan_summary_round_trips_with_its_error() {
    let mut u = user("a@x");
    let s = &mut u.s;
    assert!(s.breach_scan().unwrap().is_none());
    for e in [
        ScanError::Offline,
        ScanError::RateLimited,
        ScanError::Protocol,
        ScanError::Cancelled,
    ] {
        let summary = ScanSummary {
            started_at: at(5),
            finished_at: None,
            checked: 3,
            error: Some(e),
        };
        s.record_breach_results(&[], summary).unwrap();
        assert_eq!(s.breach_scan().unwrap().unwrap(), summary);
    }
}

#[test]
fn dismiss_lasts_until_password_changes() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "p1");
    assert!(s.queue_states().unwrap().is_empty());
    s.dismiss_rotation(v, i).unwrap();
    assert!(s.queue_states().unwrap().hides(key(v, i), at(0)));
    // Other edits keep it; a password change ends it.
    s.set_favorite(v, i, true).unwrap();
    assert!(s.queue_states().unwrap().hides(key(v, i), at(0)));
    change_password(s, v, i, "p2");
    assert!(!s.queue_states().unwrap().hides(key(v, i), at(0)));
    // A note has no password to dismiss at.
    let note = s
        .add_item(v, &ItemType::SecureNote, |e| e.set_title("n"))
        .unwrap();
    assert!(matches!(
        s.dismiss_rotation(v, note),
        Err(Error::NoPassword)
    ));
}

#[test]
fn snooze_expires() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "p1");
    s.snooze_rotation(v, i, at(10_000)).unwrap();
    let states: QueueStates = s.queue_states().unwrap();
    assert!(states.hides(key(v, i), at(9_999)));
    assert!(!states.hides(key(v, i), at(10_000)));
    assert!(!states.hides(key(v, i), at(20_000)));
    // A later snooze replaces the earlier one and keeps a dismissal.
    s.dismiss_rotation(v, i).unwrap();
    s.snooze_rotation(v, i, at(30_000)).unwrap();
    let st = *s.queue_states().unwrap().get(key(v, i)).unwrap();
    assert_eq!(st.snoozed_until, Some(at(30_000)));
    assert!(st.dismissed);
    assert!(matches!(
        s.snooze_rotation(v, ItemId::new_v7().unwrap(), at(1)),
        Err(Error::UnknownItem)
    ));
}

#[test]
fn purge_removes_breach_and_queue_rows() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let v = s.create_vault("V").unwrap();
    let i = login(s, v, "https://bank.example", "p1");
    let kept = login(s, v, "https://other.example", "p2");
    for item in [i, kept] {
        let r = result_for(s, v, item, true);
        s.record_breach_results(&[r], scan()).unwrap();
        s.snooze_rotation(v, item, at(10_000)).unwrap();
        s.dismiss_rotation(v, item).unwrap();
    }
    s.purge_item(v, i).unwrap();
    let rows = s.store.breach_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].item_id, kept);
    let queue = s.store.queue_rows().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].item_id, kept);
    // The scan summary is not tied to an item.
    assert!(s.breach_scan().unwrap().is_some());
}

#[test]
fn removing_a_vault_removes_its_rows() {
    let mut u = user("a@x");
    let s = &mut u.s;
    let gone = s.create_vault("Gone").unwrap();
    let stays = s.create_vault("Stays").unwrap();
    let a = login(s, gone, "https://bank.example", "p1");
    let b = login(s, stays, "https://bank.example", "p1");
    for (v, i) in [(gone, a), (stays, b)] {
        let r = result_for(s, v, i, true);
        s.record_breach_results(&[r], scan()).unwrap();
        s.snooze_rotation(v, i, at(10_000)).unwrap();
    }
    assert!(s.store.delete_vault(&gone, PendingPolicy::Discard).unwrap());
    let rows = s.store.breach_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].vault_id, stays);
    let queue = s.store.queue_rows().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].vault_id, stays);
}

#[test]
fn reset_sync_state_and_relock_keep_the_rows() {
    let mut u = user("a@x");
    let v = u.s.create_vault("V").unwrap();
    let i = login(&mut u.s, v, "https://bank.example", "p1");
    let r = result_for(&u.s, v, i, true);
    u.s.record_breach_results(&[r], scan()).unwrap();
    u.s.snooze_rotation(v, i, at(10_000)).unwrap();
    u.s.dismiss_rotation(v, i).unwrap();
    u.s.reset_sync_state().unwrap();
    assert_eq!(u.s.store.breach_rows().unwrap().len(), 1);
    assert_eq!(u.s.store.queue_rows().unwrap().len(), 1);
    // Rows survive a lock and unlock and still describe the item.
    let s = LockedSession::open(u.s.lock().into_store())
        .unlock(&pw(), &u.secret)
        .unwrap();
    assert!(
        s.breach_states()
            .unwrap()
            .get(key(v, i), BreachField::LoginPassword)
            .unwrap()
            .found
    );
    assert!(s.queue_states().unwrap().hides(key(v, i), at(0)));
}

#[test]
fn store_holds_no_secret_after_scan() {
    // SHA-1("password") (the HIBP example): neither the password nor its
    // hash, in any case, may be in the store files.
    const SHA1_UPPER: &str = "5BAA61E4C9B93F3F0682250B6CF8331B7EE68FD8";
    const UNIQUE: &str = "zq9-unique-plaintext-7731";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mosaic.db");
    let (mut s, _) = LockedSession::open(Store::open(&path).unwrap())
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    let v = s.create_vault("V").unwrap();
    let a = login(&mut s, v, "https://bank.example", "password");
    let b = login(&mut s, v, "https://other.example", UNIQUE);
    s.begin_rotation(v, b, &secret("pending-plaintext-5512"), None)
        .unwrap();
    let results = [result_for(&s, v, a, true), result_for(&s, v, b, false)];
    s.record_breach_results(&results, scan()).unwrap();
    s.snooze_rotation(v, a, at(10_000)).unwrap();
    s.dismiss_rotation(v, a).unwrap();
    s.lock().into_store().close().unwrap();

    let needles = [
        SHA1_UPPER.to_owned(),
        SHA1_UPPER.to_lowercase(),
        SHA1_UPPER[..5].to_owned(),
        "password".to_owned(),
        UNIQUE.to_owned(),
        "pending-plaintext-5512".to_owned(),
    ];
    let mut scanned = 0;
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(path).unwrap();
        scanned += bytes.len();
        for needle in &needles {
            let n = needle.as_bytes();
            // "password" is also a word of the schema; the sealed items and
            // the rows must not add the long values.
            if n.len() < 10 {
                continue;
            }
            assert!(
                !bytes.windows(n.len()).any(|w| w == n),
                "{needle} found in a store file"
            );
        }
    }
    assert!(scanned > 0);
}
