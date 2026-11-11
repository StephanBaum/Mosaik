//! M9 T12 (sync-chaos): oversized vault-metadata merges across three devices
//! in every delivery order (CRYPTO_SPEC §7.1, ADR-0085, M2-R16).
//!
//! Three devices write long concurrent values (as peers that ignore the
//! write caps would), then receive each other's records in every order, with
//! duplicates, applying exactly what `UnlockedSession::apply_remote_vault_meta`
//! applies: skip a dominated or equal record, `merge_and_repair`, then
//! `reduce_oversized`. Checked:
//!
//! - every stored state fits the 64 KiB limit and dominates what it merged;
//! - after the three records were delivered, all devices hold the same
//!   canonical bytes and version vector, and the dropped-field reports agree
//!   (the spec's required test);
//! - after devices also exchange their stored states, they converge, with no
//!   device-ID repair (`stamp_collision`) and no silent loss: a field that is
//!   absent at the end was reported as dropped;
//! - a replay of any record changes nothing;
//! - the outcome never depends on the writers' wall-clock stamps (skew of
//!   days either way).

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "integration tests"
)]

use mosaic_core::envelope::MAX_VAULT_META_PLAINTEXT_LEN;
use mosaic_core::model::{Causality, DroppedMeta, Timestamp, VaultMeta};
use mosaic_core::{DeviceId, Error};
use proptest::prelude::*;

const DAY_MS: i64 = 86_400_000;

fn dev(i: u8) -> DeviceId {
    DeviceId::from_bytes([i; 16]).unwrap()
}

fn base() -> VaultMeta {
    VaultMeta::new("base", dev(1), Timestamp::from_unix_millis(1)).unwrap()
}

fn copy(m: &VaultMeta) -> VaultMeta {
    VaultMeta::from_json(m.to_json().unwrap().expose_secret()).unwrap()
}

fn bytes(m: &VaultMeta) -> Vec<u8> {
    m.to_json().unwrap().expose_secret().to_vec()
}

/// A version written by device `d` on top of the shared base, with a name,
/// icon and description of the given lengths (0 for icon or description: not
/// set). The text bypasses the write caps (a peer that ignores them) by
/// swapping placeholders in the canonical plaintext, which `from_json`
/// accepts at any length.
fn long(d: u8, at_ms: i64, name: usize, icon: usize, desc: usize) -> VaultMeta {
    let mut m = copy(&base());
    {
        let mut e = m.edit(dev(d), Timestamp::from_unix_millis(at_ms));
        e.set_name("@N@").unwrap();
        if icon > 0 {
            e.set_icon(Some("@I@")).unwrap();
        }
        if desc > 0 {
            e.set_description(Some("@D@")).unwrap();
        }
    }
    let mut text = String::from_utf8(bytes(&m)).unwrap();
    let fill = |c: char, n: usize| format!("{d}{}", c.to_string().repeat(n));
    text = text.replace("@N@", &fill('n', name));
    text = text.replace("@I@", &fill('i', icon));
    text = text.replace("@D@", &fill('d', desc));
    VaultMeta::from_json(text.as_bytes()).unwrap()
}

fn assert_same(a: &VaultMeta, b: &VaultMeta, context: &str) {
    assert!(
        bytes(a) == bytes(b),
        "{context}: other canonical bytes ({} vs {} bytes, same version vector: {})",
        bytes(a).len(),
        bytes(b).len(),
        a.version() == b.version()
    );
}

fn or_report(a: &mut DroppedMeta, b: DroppedMeta) {
    a.name |= b.name;
    a.icon |= b.icon;
    a.description |= b.description;
}

/// Three devices (writers `dev(2)` to `dev(4)`) and what happened to them.
struct Run {
    states: [VaultMeta; 3],
    /// The device IDs now in use (the engine rotates one on `DeviceIdReused`).
    ids: [DeviceId; 3],
    retired: [Vec<DeviceId>; 3],
    /// What each device reported as dropped while receiving.
    reports: [DroppedMeta; 3],
    /// Repairs of the kind `merge_and_repair` flags as a device-ID collision.
    collisions: usize,
    /// Device-ID rotations the engine would have run (`DeviceIdReused`).
    rotations: usize,
}

impl Run {
    fn new(versions: &[VaultMeta; 3]) -> Self {
        Self {
            states: [copy(&versions[0]), copy(&versions[1]), copy(&versions[2])],
            ids: [dev(2), dev(3), dev(4)],
            retired: [Vec::new(), Vec::new(), Vec::new()],
            reports: [DroppedMeta::default(); 3],
            collisions: 0,
            rotations: 0,
        }
    }

    /// What `UnlockedSession::apply_remote_vault_meta` does on device `i`
    /// (with the engine's one retry after `DeviceIdReused`); whether the
    /// record changed the device.
    fn receive(&mut self, i: usize, received: &VaultMeta) -> bool {
        let local = &self.states[i];
        match local.compare(received) {
            Causality::Dominates => return false,
            Causality::Equal if local == received => return false,
            Causality::Equal | Causality::DominatedBy | Causality::Concurrent => {}
        }
        let first = local.merge_and_repair_after_reuse(received, self.ids[i], &self.retired[i]);
        let (m, collision) = match first {
            Ok(x) => x,
            Err(Error::DeviceIdReused) => {
                self.rotations += 1;
                self.retired[i].push(self.ids[i]);
                self.ids[i] = dev(100 + u8::try_from(self.rotations).unwrap());
                local
                    .merge_and_repair_after_reuse(received, self.ids[i], &self.retired[i])
                    .unwrap_or_else(|e| panic!("refused after the rotation: {e:?}"))
            }
            Err(e) => panic!("merge_and_repair refused a record: {e:?}"),
        };
        let (m, dropped) = m.reduce_oversized().unwrap();
        assert!(
            m.to_json().is_ok(),
            "a stored state must fit the metadata limit"
        );
        assert_ne!(m.compare(received), Causality::DominatedBy);
        self.collisions += usize::from(collision);
        or_report(&mut self.reports[i], dropped);
        self.states[i] = m;
        true
    }
}

/// One run: device `i` starts from its own version and receives the other
/// two in `orders[i]`; `dup` delivers every record twice.
fn deliver(versions: &[VaultMeta; 3], orders: [[usize; 2]; 3], dup: bool) -> Run {
    let mut run = Run::new(versions);
    for (i, order) in orders.iter().enumerate() {
        for &j in order {
            assert_ne!(j, i);
            run.receive(i, &versions[j]);
            if dup {
                assert!(
                    !run.receive(i, &versions[j]),
                    "a replayed record changed device {i}"
                );
            }
        }
    }
    run
}

/// Every device uploads its stored state; the others receive it, until a
/// whole round changes nothing. Returns the rounds it took.
fn exchange(run: &mut Run) -> usize {
    for round in 1..=8 {
        let mut changed = false;
        for from in 0..3 {
            for to in 0..3 {
                if from != to {
                    let sent = copy(&run.states[from]);
                    changed |= run.receive(to, &sent);
                }
            }
        }
        if !changed {
            return round;
        }
    }
    panic!("the devices still change after 8 exchange rounds");
}

const PERMS: [[[usize; 2]; 3]; 8] = [
    [[1, 2], [0, 2], [0, 1]],
    [[1, 2], [0, 2], [1, 0]],
    [[1, 2], [2, 0], [0, 1]],
    [[1, 2], [2, 0], [1, 0]],
    [[2, 1], [0, 2], [0, 1]],
    [[2, 1], [0, 2], [1, 0]],
    [[2, 1], [2, 0], [0, 1]],
    [[2, 1], [2, 0], [1, 0]],
];

/// The spec's required test: three devices write long concurrent values and
/// every delivery order ends with the same canonical bytes and version vector
/// on all three, and the same dropped-field report.
#[test]
fn three_devices_every_delivery_order_store_identical_bytes() {
    let versions = [
        long(2, 20, 20_000, 0, 20_000),
        long(3, 21, 21_000, 0, 19_000),
        long(4, 22, 22_000, 0, 18_000),
    ];
    for dup in [false, true] {
        for orders in PERMS {
            let run = deliver(&versions, orders, dup);
            assert_eq!((run.collisions, run.rotations), (0, 0));
            for i in 1..3 {
                assert_same(&run.states[i], &run.states[0], &format!("{orders:?}"));
                assert_eq!(run.states[i].version(), run.states[0].version());
                assert_eq!(run.reports[i], run.reports[0], "{orders:?}");
            }
            assert!(!run.reports[0].is_empty());
            for v in &versions {
                assert_eq!(run.states[0].compare(v), Causality::Dominates);
            }
            assert!(run.states[0].conflicts().is_empty());
        }
    }
}

/// Mixed sizes: some pairs fit together, the third record overflows. After
/// the three records everything the devices hold must still be identical
/// (the order a record arrives in must not decide the stored bytes).
///
/// M9-F50: a device that reduced the first two records (40 000 + 40 000)
/// used to keep two names after the small third, while a device that met the
/// small one first kept one. The weight counts the husks' lengths, so both
/// reduce at the third record.
#[test]
fn mixed_sizes_every_delivery_order_store_identical_bytes() {
    // 40 000 + 40 000 overflows; 40 000 + 5 000 fits.
    let versions = [
        long(2, 20, 40_000, 0, 0),
        long(3, 21, 40_000, 0, 0),
        long(4, 22, 5_000, 0, 0),
    ];
    for orders in PERMS {
        let run = deliver(&versions, orders, false);
        for i in 1..3 {
            assert_same(
                &run.states[i],
                &run.states[0],
                &format!("{orders:?}: device {i}"),
            );
        }
    }
}

/// With mixed sizes the devices may differ for a moment, but exchanging
/// their stored states must converge them, with no silent loss.
#[test]
fn mixed_sizes_converge_after_the_devices_exchange_states() {
    let versions = mixed();
    for orders in PERMS {
        let mut run = deliver(&versions, orders, false);
        exchange(&mut run);
        for i in 1..3 {
            assert_same(&run.states[i], &run.states[0], &format!("{orders:?}"));
        }
    }
}

fn mixed() -> [VaultMeta; 3] {
    // 40 000 + 40 000 overflows; 40 000 + 5 000 fits.
    [
        long(2, 20, 40_000, 0, 0),
        long(3, 21, 40_000, 0, 0),
        long(4, 22, 5_000, 0, 0),
    ]
}

/// M9-F51: two devices used to end with equal version vectors and different
/// contents, which `merge_and_repair` took for a device-ID reuse (values
/// written back by the receiver, ordered by the writers' wall clocks).
/// Nobody reused an ID: husk-only differences merge without a repair.
#[test]
fn exchange_never_runs_a_device_id_repair() {
    let versions = mixed();
    for orders in PERMS {
        let mut run = deliver(&versions, orders, false);
        exchange(&mut run);
        assert_eq!(
            (run.collisions, run.rotations),
            (0, 0),
            "{orders:?}: a device-ID repair or rotation ran though nobody reused an ID"
        );
    }
}

/// The same schedule as [`mixed_sizes_converge_after_the_devices_exchange_states`]
/// with the writers' wall clocks skewed by days: the shown name must not
/// change (the repair above orders re-added values by wall clock).
#[test]
fn mixed_sizes_outcome_does_not_depend_on_wall_clocks() {
    let outcome = |skew: [i64; 3]| {
        let versions = [
            long(2, 1_000 + skew[0], 40_000, 0, 0),
            long(3, 1_000 + skew[1], 40_000, 0, 0),
            long(4, 1_000 + skew[2], 5_000, 0, 0),
        ];
        let mut shown = Vec::new();
        for orders in PERMS {
            let mut run = deliver(&versions, orders, false);
            exchange(&mut run);
            shown.push(run.states[0].name().and_then(|n| n.chars().next()));
        }
        shown
    };
    let reference = outcome([0, 0, 0]);
    for skew in [
        [-30 * DAY_MS, 0, 30 * DAY_MS],
        [30 * DAY_MS, 0, -30 * DAY_MS],
        [0, 30 * DAY_MS, -30 * DAY_MS],
    ] {
        assert!(
            outcome(skew) == reference,
            "skew {skew:?} changed the shown name"
        );
    }
}

/// The reduction is a function of the merge, not of the writers' wall
/// clocks.
#[test]
fn clock_skew_of_days_does_not_change_the_outcome() {
    let shape = |skew: [i64; 3]| {
        let versions = [
            long(2, 1_000 + skew[0], 20_000, 0, 20_000),
            long(3, 1_000 + skew[1], 21_000, 0, 19_000),
            long(4, 1_000 + skew[2], 22_000, 0, 18_000),
        ];
        let mut run = deliver(&versions, PERMS[3], false);
        exchange(&mut run);
        (
            run.states[0].name().map(str::to_owned),
            run.states[0].description().map(str::to_owned),
            run.states[0].version().clone(),
            run.reports,
        )
    };
    let reference = shape([0, 0, 0]);
    for skew in [
        [-30 * DAY_MS, 0, 30 * DAY_MS],
        [30 * DAY_MS, -30 * DAY_MS, 0],
        [3 * DAY_MS, 3 * DAY_MS, -3 * DAY_MS],
        [-365 * DAY_MS, -365 * DAY_MS, -365 * DAY_MS],
    ] {
        assert!(
            shape(skew) == reference,
            "skew {skew:?} changed the outcome"
        );
    }
}

fn arb_version() -> impl Strategy<Value = (usize, usize, usize)> {
    // Each version alone fits (at most 56 000 bytes of text).
    (1usize..=30_000, 0usize..=13_000, 0usize..=13_000)
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 96, ..ProptestConfig::default() })]

    /// Random sizes, orders and duplicates: every stored state fits, replays
    /// change nothing, the devices converge after exchanging their states and
    /// no field is lost without a report.
    #[test]
    fn random_metadata_schedules_converge(
        sizes in proptest::array::uniform3(arb_version()),
        order_pick in proptest::array::uniform3(0usize..2),
        dup in any::<bool>(),
    ) {
        let versions = versions_of(&sizes, [0; 3]);
        let mut run = deliver(&versions, orders_of(order_pick), dup);
        exchange(&mut run);
        for i in 1..3 {
            prop_assert!(bytes(&run.states[i]) == bytes(&run.states[0]));
        }
        prop_assert!(bytes(&run.states[0]).len() <= MAX_VAULT_META_PLAINTEXT_LEN);
        // No silent loss: a field that was written and is absent must have
        // been reported as dropped by some device.
        let mut any = DroppedMeta::default();
        for r in run.reports {
            or_report(&mut any, r);
        }
        let kept = &run.states[0];
        prop_assert!(kept.name().is_some() || any.name);
        let wrote_icon = sizes.iter().any(|s| s.1 > 0);
        let wrote_desc = sizes.iter().any(|s| s.2 > 0);
        prop_assert!(kept.icon().is_some() || any.icon || !wrote_icon);
        prop_assert!(kept.description().is_some() || any.description || !wrote_desc);
    }

    /// The spec's strict statement under random sizes: once all three
    /// records were delivered, before any state exchange, the stored bytes
    /// are already the same on every device.
    #[test]
    fn random_sizes_identical_after_delivery(
        sizes in proptest::array::uniform3(arb_version()),
        order_pick in proptest::array::uniform3(0usize..2),
    ) {
        let versions = versions_of(&sizes, [0; 3]);
        let run = deliver(&versions, orders_of(order_pick), false);
        for i in 1..3 {
            prop_assert!(bytes(&run.states[i]) == bytes(&run.states[0]));
        }
    }

    /// Exchanging states never runs a device-ID repair, and the shown values
    /// do not depend on the writers' wall clocks (skew of up to 30 days).
    #[test]
    fn random_schedules_need_no_repair_and_ignore_wall_clocks(
        sizes in proptest::array::uniform3(arb_version()),
        order_pick in proptest::array::uniform3(0usize..2),
        skew in proptest::array::uniform3(-30i64..=30),
    ) {
        let plain = {
            let mut run = deliver(&versions_of(&sizes, [0; 3]), orders_of(order_pick), false);
            exchange(&mut run);
            run
        };
        let skewed = {
            let mut run = deliver(&versions_of(&sizes, skew), orders_of(order_pick), false);
            exchange(&mut run);
            run
        };
        prop_assert_eq!((plain.collisions, plain.rotations), (0, 0));
        prop_assert_eq!((skewed.collisions, skewed.rotations), (0, 0));
        prop_assert_eq!(skewed.states[0].name(), plain.states[0].name());
        prop_assert_eq!(skewed.states[0].icon(), plain.states[0].icon());
        prop_assert_eq!(skewed.states[0].description(), plain.states[0].description());
        prop_assert_eq!(skewed.states[0].version(), plain.states[0].version());
    }
}

/// Device `i` of [`versions_of`] after it received the record of device
/// `from` (or nothing, if `from == i`): what it stores, with husks when the
/// pair did not fit.
fn replica(versions: &[VaultMeta; 3], i: usize, from: usize) -> VaultMeta {
    if from == i {
        return copy(&versions[i]);
    }
    let own = dev(u8::try_from(i).unwrap() + 2);
    let (m, collision) = versions[i].merge_and_repair(&versions[from], own).unwrap();
    assert!(!collision);
    copy(&m.reduce_oversized().unwrap().0)
}

/// The six orders of three replicas.
const ORDERS: [[usize; 3]; 6] = [
    [0, 1, 2],
    [0, 2, 1],
    [1, 0, 2],
    [1, 2, 0],
    [2, 0, 1],
    [2, 1, 0],
];

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    /// M9-F50/F51: the plain metadata join stays a semilattice with husks
    /// (CRYPTO_SPEC §7.1). Three replicas that carry husks from their own
    /// oversized reductions (step 1, and step 2 when the shown values alone
    /// do not fit) merge to the same record in every order and grouping,
    /// that record reduces to identical bytes, and merging is idempotent and
    /// absorbs each replica.
    #[test]
    fn merge_order_of_three_oversized_replicas_does_not_matter(
        sizes in proptest::array::uniform3((1usize..=36_000, 0usize..=13_000, 0usize..=13_000)),
        from in proptest::array::uniform3(0usize..3),
    ) {
        let versions = versions_of(&sizes, [0; 3]);
        let r = [0, 1, 2].map(|i| replica(&versions, i, from[i]));
        let reference = r[0].merge(&r[1]).merge(&r[2]);
        let reference_bytes = bytes(&reference.reduce_oversized().unwrap().0);
        for [x, y, z] in ORDERS {
            let left = r[x].merge(&r[y]).merge(&r[z]);
            let right = r[x].merge(&r[y].merge(&r[z]));
            prop_assert!(left == reference, "order {:?}, left grouping", [x, y, z]);
            prop_assert!(right == reference, "order {:?}, right grouping", [x, y, z]);
            prop_assert!(bytes(&left.reduce_oversized().unwrap().0) == reference_bytes);
            prop_assert!(bytes(&right.reduce_oversized().unwrap().0) == reference_bytes);
        }
        for x in &r {
            prop_assert!(&x.merge(x) == x, "idempotent");
            prop_assert!(reference.merge(x) == reference, "absorbs a replica");
        }
        prop_assert!(reference.compare(&versions[0]) != Causality::DominatedBy);
    }
}

fn versions_of(sizes: &[(usize, usize, usize); 3], skew_days: [i64; 3]) -> [VaultMeta; 3] {
    [0, 1, 2].map(|i| {
        let d = u8::try_from(i).unwrap() + 2;
        long(
            d,
            1_000 + skew_days[i] * DAY_MS,
            sizes[i].0,
            sizes[i].1,
            sizes[i].2,
        )
    })
}

fn orders_of(pick: [usize; 3]) -> [[usize; 2]; 3] {
    [
        [[1, 2], [2, 1]][pick[0]],
        [[0, 2], [2, 0]][pick[1]],
        [[0, 1], [1, 0]][pick[2]],
    ]
}
