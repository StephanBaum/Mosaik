//! Rotation attempts (M7b, CRYPTO_SPEC §7.4, spec §9.1).
#![expect(
    clippy::assert_is_empty,
    clippy::many_single_char_names,
    clippy::cast_possible_wrap,
    reason = "tests"
)]

use proptest::prelude::*;

use super::*;
use crate::model::clock::Timestamp;

const DAY: i64 = 86_400_000;

fn dev(b: u8) -> DeviceId {
    DeviceId::from_bytes([b; 16]).unwrap()
}

fn at(ms: i64) -> Timestamp {
    Timestamp::from_unix_millis(ms)
}

fn login() -> Item {
    let mut item = Item::new(&ItemType::Login, dev(1), at(100)).unwrap();
    let mut e = item.edit(dev(1), at(100));
    e.set_title("GitHub").unwrap();
    e.set_secret(SecretField::Password, "pw-1").unwrap();
    item
}

fn copy(item: &Item) -> Item {
    Item::from_json(item.to_json().unwrap().expose_secret()).unwrap()
}

fn password(item: &Item) -> String {
    item.secret(SecretField::Password)
        .unwrap()
        .unwrap()
        .expose_secret()
        .to_owned()
}

fn history(item: &Item) -> Vec<(String, HistoryReason)> {
    item.password_history()
        .into_iter()
        .map(|h| (h.value.expose_secret().to_owned(), h.reason))
        .collect()
}

fn pendings(item: &Item) -> Vec<String> {
    item.rotation_attempts()
        .into_iter()
        .map(|a| a.pending.expose_secret().to_owned())
        .collect()
}

fn begin(item: &mut Item, d: u8, t: i64, value: &str) -> RotationAttemptId {
    item.edit(dev(d), at(t))
        .begin_rotation(value, Some(RulesSource::Page))
        .unwrap()
}

/// `a` merges `b` as device `d` (the repairing merge every store runs).
fn merge_into(a: &mut Item, d: u8, b: &Item) {
    *a = a.merge_and_repair(b, dev(d)).unwrap().item;
}

#[test]
fn rotation_attempt_round_trips_through_encoding() {
    let mut item = login();
    let id = begin(&mut item, 1, 200, "new-pw");
    let back = copy(&item);
    let attempts = back.rotation_attempts();
    assert_eq!(attempts.len(), 1);
    let a = &attempts[0];
    assert_eq!(a.id, id);
    assert_eq!(a.pending.expose_secret(), "new-pw");
    assert_eq!(a.started_at, at(200));
    assert_eq!(a.stage, RotationStage::Prepared);
    assert_eq!(a.rules_source, Some(RulesSource::Page));
    assert_eq!(RotationAttemptId::from_stored(&id.to_string()).unwrap(), id);
    assert!(id.to_string().len() <= 64);
    assert!(back.unreadable_fields().is_empty());
    // The attempt is not a user-visible conflict and does not touch the
    // password.
    assert!(back.conflicts().is_empty());
    assert_eq!(password(&back), "pw-1");
    // The doc layout of the spec.
    let json = String::from_utf8(back.to_json().unwrap().expose_secret().to_vec()).unwrap();
    assert!(json.contains(r#""stage":"prepared""#));
    assert!(json.contains(&format!(r#""rotation:{id}""#)));
    // Stage and policy round trip too.
    let mut e = item.edit(dev(1), at(300));
    assert!(e.mark_rotation_delivered(id).unwrap());
    assert!(!e.mark_rotation_delivered(id).unwrap());
    e.set_rotation_policy(Some(90)).unwrap();
    let back = copy(&item);
    assert_eq!(back.rotation_attempts()[0].stage, RotationStage::Delivered);
    assert_eq!(back.rotation_policy(), Some(90));
    item.edit(dev(1), at(400))
        .set_rotation_policy(Some(91))
        .unwrap_err();
    item.edit(dev(1), at(400))
        .set_rotation_policy(None)
        .unwrap();
    assert_eq!(item.rotation_policy(), None);
}

#[test]
fn concurrent_attempts_on_two_devices_both_survive_merge() {
    let base = login();
    let mut a = copy(&base);
    let mut b = copy(&base);
    let ia = begin(&mut a, 1, 200, "from-a");
    let ib = begin(&mut b, 2, 210, "from-b");
    assert_ne!(ia, ib);
    let ab = a.merge(&b);
    let ba = b.merge(&a);
    assert_eq!(ab, ba);
    assert_eq!(pendings(&ab), ["from-a", "from-b"]);
    assert!(ab.conflicts().is_empty());
    // Confirming one moves the other to the history.
    let mut m = ab;
    m.edit(dev(1), at(300)).confirm_rotation(ib).unwrap();
    assert_eq!(password(&m), "from-b");
    assert!(m.rotation_attempts().is_empty());
    assert!(history(&m).contains(&("from-a".to_owned(), HistoryReason::Unconfirmed)));
}

#[test]
fn confirm_moves_current_to_history_and_closes_other_attempts() {
    let mut item = login();
    let x = begin(&mut item, 1, 200, "x-new");
    let y = begin(&mut item, 1, 201, "y-new");
    let z = begin(&mut item, 1, 202, "z-new");
    let before = item.version().get(&dev(1));
    item.edit(dev(1), at(300)).confirm_rotation(y).unwrap();
    assert_eq!(password(&item), "y-new");
    assert!(item.rotation_attempts().is_empty());
    let h = history(&item);
    assert!(h.contains(&("pw-1".to_owned(), HistoryReason::Replaced)));
    assert!(h.contains(&("x-new".to_owned(), HistoryReason::Unconfirmed)));
    assert!(h.contains(&("z-new".to_owned(), HistoryReason::Unconfirmed)));
    assert_eq!(h.len(), 3);
    // One edit, one stamp.
    assert_eq!(item.version().get(&dev(1)), before + 1);
    // A closed attempt cannot be closed again.
    assert_eq!(
        item.edit(dev(1), at(301)).confirm_rotation(x),
        Err(Error::RotationNotFound)
    );
    assert_eq!(
        item.edit(dev(1), at(301)).fail_rotation(z),
        Err(Error::RotationNotFound)
    );
    assert_eq!(
        item.edit(dev(1), at(301)).mark_rotation_delivered(z),
        Err(Error::RotationNotFound)
    );
}

#[test]
fn fail_moves_pending_to_history_unconfirmed() {
    let mut item = login();
    let id = begin(&mut item, 1, 200, "new-pw");
    item.edit(dev(1), at(300)).fail_rotation(id).unwrap();
    assert_eq!(password(&item), "pw-1");
    assert!(item.rotation_attempts().is_empty());
    assert_eq!(
        history(&item),
        [("new-pw".to_owned(), HistoryReason::Unconfirmed)]
    );
    // The reason is stored in the entry.
    let json = String::from_utf8(item.to_json().unwrap().expose_secret().to_vec()).unwrap();
    assert!(json.contains(r#""reason":"unconfirmed""#));
}

#[test]
fn delivered_beats_concurrent_close_and_value_is_repaired_into_history() {
    let mut base = login();
    let x = begin(&mut base, 1, 200, "x-new");
    // Device 1 hands the value to a page; device 2 confirms it, changes the
    // password again and clears the history: the value is nowhere on 2.
    let mut a = copy(&base);
    a.edit(dev(1), at(300)).mark_rotation_delivered(x).unwrap();
    let mut b = copy(&base);
    b.edit(dev(2), at(310)).confirm_rotation(x).unwrap();
    b.edit(dev(2), at(311))
        .set_secret(SecretField::Password, "later")
        .unwrap();
    b.edit(dev(2), at(312)).clear_password_history().unwrap();
    assert!(history(&b).is_empty());
    // The attempt stays open (delivered wins) and its value is in the
    // history again, as unconfirmed.
    let mut m = copy(&a);
    merge_into(&mut m, 1, &b);
    let attempts = m.rotation_attempts();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].stage, RotationStage::Delivered);
    assert_eq!(
        history(&m),
        [("x-new".to_owned(), HistoryReason::Unconfirmed)]
    );
    // The other direction reaches the same set of values.
    let mut n = copy(&b);
    merge_into(&mut n, 2, &a);
    assert_eq!(pendings(&n), ["x-new"]);
    assert!(history(&n).contains(&("x-new".to_owned(), HistoryReason::Unconfirmed)));
}

#[test]
fn a_confirmed_attempt_is_closed_when_a_concurrent_delivery_arrives() {
    let mut base = login();
    let x = begin(&mut base, 1, 200, "x-new");
    let mut a = copy(&base);
    a.edit(dev(1), at(300)).mark_rotation_delivered(x).unwrap();
    let mut b = copy(&base);
    b.edit(dev(2), at(310)).confirm_rotation(x).unwrap();
    merge_into(&mut a, 1, &b);
    assert!(a.rotation_attempts().is_empty());
    assert_eq!(password(&a), "x-new");
    assert_eq!(history(&a), [("pw-1".to_owned(), HistoryReason::Replaced)]);
}

#[test]
fn restore_history_swaps_values() {
    let mut item = login();
    let id = begin(&mut item, 1, 200, "new-pw");
    item.edit(dev(1), at(300)).fail_rotation(id).unwrap();
    let entry = item.password_history().remove(0);
    assert_eq!(entry.value.expose_secret(), "new-pw");
    item.edit(dev(1), at(400))
        .restore_history(&entry.id)
        .unwrap();
    assert_eq!(password(&item), "new-pw");
    assert_eq!(
        history(&item),
        [("pw-1".to_owned(), HistoryReason::Replaced)]
    );
    // Restoring again swaps back; an unknown ID is refused.
    let back = item.password_history().remove(0);
    item.edit(dev(1), at(500))
        .restore_history(&back.id)
        .unwrap();
    assert_eq!(password(&item), "pw-1");
    assert_eq!(
        item.edit(dev(1), at(600)).restore_history(&back.id),
        Err(Error::HistoryNotFound)
    );
    assert!(HistoryId::from_stored("x").is_ok());
}

#[test]
fn clear_history_refused_with_open_attempt() {
    let mut item = login();
    item.edit(dev(1), at(150))
        .set_secret(SecretField::Password, "pw-2")
        .unwrap();
    let id = begin(&mut item, 1, 200, "new-pw");
    assert_eq!(
        item.edit(dev(1), at(300)).clear_password_history(),
        Err(Error::RotationOpen)
    );
    assert_eq!(item.password_history().len(), 1);
    item.edit(dev(1), at(310)).fail_rotation(id).unwrap();
    item.edit(dev(1), at(320)).clear_password_history().unwrap();
}

#[test]
fn clear_history_keeps_recent_unconfirmed() {
    let mut item = login();
    item.edit(dev(1), at(150))
        .set_secret(SecretField::Password, "pw-2")
        .unwrap();
    let id = begin(&mut item, 1, 200, "new-pw");
    item.edit(dev(1), at(300)).fail_rotation(id).unwrap();
    item.edit(dev(1), at(300 + DAY))
        .clear_password_history()
        .unwrap();
    assert_eq!(
        history(&item),
        [("new-pw".to_owned(), HistoryReason::Unconfirmed)]
    );
    item.edit(dev(1), at(300 + 31 * DAY))
        .clear_password_history()
        .unwrap();
    assert!(history(&item).is_empty());
}

#[test]
fn history_reason_absent_reads_as_replaced() {
    let mut item = login();
    item.edit(dev(1), at(200))
        .set_secret(SecretField::Password, "pw-2")
        .unwrap();
    let h = item.password_history();
    assert_eq!(h[0].reason, HistoryReason::Replaced);
    let json = String::from_utf8(item.to_json().unwrap().expose_secret().to_vec()).unwrap();
    assert!(!json.contains("reason"));
}

/// An item written by an M7 build has history entries without `reason`;
/// an M7 reader ignores the member in entries M7b wrote. Both directions
/// through the golden M7 plaintext.
#[test]
fn v1_parser_ignores_reason_key() {
    let golden = include_str!("../../../tests/golden/item_plaintext.json");
    let item = Item::from_json(golden.as_bytes()).unwrap();
    let h = item.password_history();
    assert_eq!(h.len(), 2);
    assert!(h.iter().all(|e| e.reason == HistoryReason::Replaced));
    assert!(item.unreadable_fields().is_empty());
    // The same document with a reason on one entry, and a rotation key a
    // newer build wrote.
    let with = golden
        .replacen(
            r#""field":"login.password","value":"correct horse"#,
            r#""field":"login.password","reason":"unconfirmed","value":"correct horse"#,
            1,
        )
        .replacen(
            r#""ssh.comment":"#,
            r#""rotation_policy":[{"at":1,"c":2,"d":"01010101-0101-0101-0101-010101010101","v":{"every_days":90}}],"ssh.comment":"#,
            1,
        );
    let item = Item::from_json(with.as_bytes()).unwrap();
    let h = item.password_history();
    assert_eq!(
        h.iter()
            .filter(|e| e.reason == HistoryReason::Unconfirmed)
            .count(),
        1
    );
    assert_eq!(item.rotation_policy(), Some(90));
    assert!(item.unreadable_fields().is_empty());
    let again = copy(&item);
    assert_eq!(again, item);
}

#[test]
fn secret_mark_changes_on_every_password_write() {
    let mut item = login();
    assert!(item.secret_mark(SecretField::WifiPassword).is_none());
    let m1 = item.secret_mark(SecretField::Password).unwrap();
    assert_eq!(item.secret_mark(SecretField::Password), Some(m1));
    item.edit(dev(1), at(200))
        .set_secret(SecretField::Password, "pw-2")
        .unwrap();
    let m2 = item.secret_mark(SecretField::Password).unwrap();
    assert_ne!(m1, m2);
    item.edit(dev(2), at(300))
        .set_secret(SecretField::Password, "pw-3")
        .unwrap();
    let m3 = item.secret_mark(SecretField::Password).unwrap();
    assert!(m3 != m2 && m3 != m1);
    // Setting the same text again is no write.
    item.edit(dev(2), at(301))
        .set_secret(SecretField::Password, "pw-3")
        .unwrap();
    assert_eq!(item.secret_mark(SecretField::Password), Some(m3));
    // A rotation that is confirmed writes the password.
    let id = begin(&mut item, 1, 400, "pw-4");
    assert_eq!(item.secret_mark(SecretField::Password), Some(m3));
    item.edit(dev(1), at(500)).confirm_rotation(id).unwrap();
    let m4 = item.secret_mark(SecretField::Password).unwrap();
    assert_ne!(m3, m4);
    // The mark is bytes of the stamp, not of the value, and round trips.
    assert_eq!(ValueMark::from_bytes(m4.as_bytes()), Some(m4));
    assert_eq!(ValueMark::from_bytes(&[0; 5]), None);
    assert!(!m4.as_bytes().windows(4).any(|w| w == b"pw-4"));
}

#[test]
fn begin_rotation_limit_four() {
    let mut item = login();
    for i in 0..MAX_OPEN_ATTEMPTS {
        begin(&mut item, 1, 200 + i as i64, &format!("p{i}"));
    }
    assert_eq!(
        item.edit(dev(1), at(300)).begin_rotation("p9", None),
        Err(Error::TooManyAttempts)
    );
    let first = item.rotation_attempts()[0].id;
    item.edit(dev(1), at(301)).fail_rotation(first).unwrap();
    item.edit(dev(1), at(302))
        .begin_rotation("p9", None)
        .unwrap();
    // Value rules and item type.
    assert_eq!(
        item.edit(dev(1), at(303)).begin_rotation("", None),
        Err(Error::Malformed)
    );
    let long = "x".repeat(MAX_PENDING_CHARS + 1);
    let mut item2 = login();
    assert_eq!(
        item2.edit(dev(1), at(303)).begin_rotation(&long, None),
        Err(Error::Malformed)
    );
    let ok = "ä".repeat(MAX_PENDING_CHARS);
    item2
        .edit(dev(1), at(304))
        .begin_rotation(&ok, None)
        .unwrap();
    let mut note = Item::new(&ItemType::SecureNote, dev(1), at(100)).unwrap();
    assert_eq!(
        note.edit(dev(1), at(200)).begin_rotation("x", None),
        Err(Error::NotALogin)
    );
}

#[test]
fn debug_redacts_pending() {
    let mut item = login();
    begin(&mut item, 1, 200, "very-secret-pending");
    let a = item.rotation_attempts().remove(0);
    assert!(!format!("{a:?}").contains("very-secret-pending"));
    assert!(!format!("{item:?}").contains("very-secret-pending"));
    item.edit(dev(1), at(300)).fail_rotation(a.id).unwrap();
    let h = item.password_history().remove(0);
    assert!(!format!("{h:?}").contains("very-secret-pending"));
}

#[derive(Clone, Debug)]
enum Op {
    Begin(usize),
    Deliver(usize, usize),
    Confirm(usize, usize),
    Fail(usize, usize),
    Restore(usize, usize),
    Sync(usize, usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0..3usize).prop_map(Op::Begin),
        (0..3usize, 0..4usize).prop_map(|(r, k)| Op::Deliver(r, k)),
        (0..3usize, 0..4usize).prop_map(|(r, k)| Op::Confirm(r, k)),
        (0..3usize, 0..4usize).prop_map(|(r, k)| Op::Fail(r, k)),
        (0..3usize, 0..4usize).prop_map(|(r, k)| Op::Restore(r, k)),
        (0..3usize, 0..3usize).prop_map(|(a, b)| Op::Sync(a, b)),
    ]
}

/// Whether `v` is the password (any live value), an open attempt or a
/// history entry of `item`.
fn holds(item: &Item, v: &str) -> bool {
    item.entries(SecretField::Password.key())
        .any(|e| e.value().as_str() == Some(v))
        || pendings(item).iter().any(|p| p == v)
        || history(item).iter().any(|(h, _)| h == v)
}

fn apply(replicas: &mut [Item], op: &Op, t: i64, ever: &mut Vec<String>) {
    let dev_of = |r: usize| u8::try_from(r + 1).unwrap();
    match *op {
        Op::Begin(r) => {
            let v = format!("pend-{}", ever.len());
            if replicas[r]
                .edit(dev(dev_of(r)), at(t))
                .begin_rotation(&v, None)
                .is_ok()
            {
                ever.push(v);
            }
        }
        Op::Deliver(r, k) | Op::Confirm(r, k) | Op::Fail(r, k) => {
            let attempts = replicas[r].rotation_attempts();
            let Some(a) = attempts.get(k % attempts.len().max(1)) else {
                return;
            };
            let mut e = replicas[r].edit(dev(dev_of(r)), at(t));
            let done = match *op {
                Op::Deliver(..) => e.mark_rotation_delivered(a.id).map(|_| ()),
                Op::Confirm(..) => e.confirm_rotation(a.id),
                _ => e.fail_rotation(a.id),
            };
            done.unwrap();
        }
        Op::Restore(r, k) => {
            let h = replicas[r].password_history();
            let Some(entry) = h.get(k % h.len().max(1)) else {
                return;
            };
            replicas[r]
                .edit(dev(dev_of(r)), at(t))
                .restore_history(&entry.id)
                .unwrap();
        }
        Op::Sync(a, b) => {
            if a != b {
                let other = copy(&replicas[b]);
                merge_into(&mut replicas[a], dev_of(a), &other);
            }
        }
    }
}

proptest! {
    /// Every value that was ever an attempt's new password is the password,
    /// an open attempt or a history entry on every replica after a full
    /// sync, until the history's count limit trims it.
    #[test]
    fn no_pending_value_is_ever_lost(ops in proptest::collection::vec(op(), 1..14)) {
        let base = login();
        let mut replicas = vec![copy(&base), copy(&base), copy(&base)];
        let mut ever = Vec::new();
        for (i, o) in ops.iter().enumerate() {
            apply(&mut replicas, o, 1_000 + i as i64, &mut ever);
        }
        for _ in 0..2 {
            for a in 0..3 {
                for b in 0..3 {
                    apply(&mut replicas, &Op::Sync(a, b), 9_000, &mut ever);
                }
            }
        }
        for r in &replicas {
            if r.password_history().len() >= MAX_HISTORY {
                continue;
            }
            for v in &ever {
                prop_assert!(holds(r, v), "{v} lost");
            }
        }
    }
}
