//! M10-G5: the v11 boot marker and the re-authentication counter as a model
//! test, plus persistence across a reopened store.

use mosaic_vault::storage::{MAX_BOOT_ID_LEN, Store, StoredBootMarker};
use proptest::prelude::*;

#[derive(Clone, Debug)]
enum Op {
    Put(Vec<u8>, i64),
    Delete,
    Clear,
    Touch,
    Fail,
    Reset,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (prop::collection::vec(any::<u8>(), 0..40), any::<i64>()).prop_map(|(b, t)| Op::Put(b, t)),
        Just(Op::Delete),
        Just(Op::Clear),
        Just(Op::Touch),
        Just(Op::Fail),
        Just(Op::Reset),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    /// The store follows a simple model: a marker holds the last accepted
    /// 1..=32 byte boot ID, a rejected ID leaves it unchanged, `clear` never
    /// leaves a usable marker behind, and the failure count saturates at 255
    /// and survives marker changes.
    #[test]
    fn marker_and_failure_counter_follow_the_model(ops in prop::collection::vec(op(), 0..40)) {
        let mut s = Store::open_in_memory().unwrap();
        let mut marker: Option<Vec<u8>> = None;
        let mut failures: u8 = 0;
        for op in ops {
            match op {
                Op::Put(id, at) => {
                    let ok = (1..=MAX_BOOT_ID_LEN).contains(&id.len());
                    prop_assert_eq!(s.put_boot_marker(&id, at).is_ok(), ok);
                    if ok {
                        marker = Some(id);
                    }
                }
                Op::Delete => {
                    prop_assert_eq!(s.delete_boot_marker().unwrap(), marker.take().is_some());
                }
                Op::Clear => {
                    s.clear_boot_marker().unwrap();
                    marker = None;
                }
                Op::Touch => {
                    prop_assert_eq!(s.touch_boot_marker().unwrap(), marker.is_some());
                }
                Op::Fail => {
                    failures = failures.saturating_add(1);
                    prop_assert_eq!(s.note_reauth_failure().unwrap(), failures);
                }
                Op::Reset => {
                    s.reset_reauth_failures().unwrap();
                    failures = 0;
                }
            }
            let stored = s.boot_marker().unwrap().map(|m: StoredBootMarker| m.boot_id);
            prop_assert_eq!(&stored, &marker);
            prop_assert_eq!(s.reauth_failures().unwrap(), failures);
            prop_assert_eq!(s.boot_marker_is_neutral().unwrap(), marker.is_none());
        }
    }
}

#[test]
fn marker_and_failures_survive_a_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store.sqlite");
    {
        let mut s = Store::open(&path).unwrap();
        s.put_boot_marker(&[1, 2, 3], 42).unwrap();
        for _ in 0..3 {
            s.note_reauth_failure().unwrap();
        }
    }
    let s = Store::open(&path).unwrap();
    assert_eq!(
        s.boot_marker().unwrap(),
        Some(StoredBootMarker {
            boot_id: vec![1, 2, 3],
            recorded_at_ms: 42
        })
    );
    assert_eq!(s.reauth_failures().unwrap(), 3);
}
