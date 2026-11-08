//! Sync chaos with three accounts (M6b spec §8.6): A (owner, two devices),
//! B (manager) and C (reader) share vaults through the real server on
//! 127.0.0.1 and the sync engine. After every scenario every current member
//! holds the same key and records, a removed account holds no key above its
//! removal generation, and no record is lost.

use std::sync::atomic::Ordering;

use mosaic_core::model::Role;
use mosaic_core::{ItemId, VaultId};
use mosaic_sync::engine::{SyncError, SyncWarning};
use mosaic_sync::sharing::{GrantScope, InviteState};
use mosaic_sync::testutil::{ChaosConfig, MaliciousTransport, SimRng};
use mosaic_sync::transport::{Rejection, TransportError};
use mosaic_vault::session::RevocationReason;

use crate::hosted::FLAKY;
use crate::scenarios::engine_for;
use crate::sharing_world::SWorld;

/// A, B and C at rest around one vault with three items: A is the owner
/// (devices 0 and 1), B manages, C reads.
fn trio(seed: u64) -> (SWorld, VaultId, Vec<ItemId>) {
    trio_with(seed, Role::Manage, Role::Read)
}

/// [`trio`] with the roles of B and C chosen.
pub(crate) fn trio_with(seed: u64, role_b: Role, role_c: Role) -> (SWorld, VaultId, Vec<ItemId>) {
    let mut w = SWorld::with_accounts(seed, 3);
    let vault = w.new_vault(0, 0, "Shared");
    let items: Vec<ItemId> = (0..3)
        .map(|i| w.add_login(0, 0, vault, &format!("item {i}")))
        .collect();
    w.sync(0, 0);
    w.join(0);
    w.sync(0, 1);
    // Identity keys are made and published by the first run; an account
    // that never synced has none and the directory serves a fake key.
    w.sync(1, 0);
    w.sync(2, 0);
    w.invite(0, 0, vault, 1, role_b).unwrap();
    w.invite(0, 0, vault, 2, role_c).unwrap();
    w.accept(1, 0, vault);
    w.accept(2, 0, vault);
    w.converge();
    (w, vault, items)
}

/// What two replicas of an item must agree on.
fn shape(w: &SWorld, a: usize, d: usize, vault: VaultId, item: ItemId) -> String {
    let s = w.accts[a].devs[d].s();
    let it = s.item(vault, item).unwrap();
    format!(
        "{:?}|{:?}|{:?}",
        it.title(),
        it.secret(mosaic_core::model::SecretField::Password)
            .unwrap()
            .map(|p| p.expose_secret().to_owned()),
        it.version()
    )
}

/// Every device of the given accounts holds the same items at the same
/// generation, with no queue left.
pub(crate) fn assert_agree(w: &SWorld, vault: VaultId, members: &[usize], items: &[ItemId]) {
    let generation = w.generation(members[0], 0, vault);
    for &a in members {
        for d in 0..w.accts[a].devs.len() {
            assert_eq!(
                w.generation(a, d, vault),
                generation,
                "account {a} device {d} holds another generation"
            );
            for item in items {
                assert_eq!(
                    shape(w, members[0], 0, vault, *item),
                    shape(w, a, d, vault, *item),
                    "account {a} device {d} disagrees on {item:?}"
                );
            }
            assert!(
                w.record_generations(a, d, vault)
                    .iter()
                    .all(|(_, g)| *g == generation.get()),
                "account {a} device {d} holds a record under another key"
            );
            let dev = w.accts[a].devs[d].s();
            assert!(
                dev.pending_uploads_in(vault, 100).unwrap().is_empty()
                    || dev.vault_own_role(vault).unwrap() == Role::Read,
                "account {a} device {d} has a queue left"
            );
        }
    }
}

/// No device of the accounts carries a finding against the server or about
/// sharing.
pub(crate) fn assert_no_findings(w: &SWorld, accounts: &[usize]) {
    for &a in accounts {
        for (d, dev) in w.accts[a].devs.iter().enumerate() {
            let s = dev.s();
            assert!(
                s.sync_findings().unwrap().is_clear(),
                "account {a} device {d}: {:?}",
                s.sync_findings().unwrap()
            );
            assert!(
                s.sharing_findings().unwrap().is_empty(),
                "account {a} device {d}: {:?}",
                s.sharing_findings().unwrap()
            );
        }
    }
}

#[test]
fn the_trio_shares_one_vault() {
    let (w, vault, items) = trio(1);
    assert_agree(&w, vault, &[0, 1, 2], &items);
    assert_eq!(w.generation(0, 0, vault).get(), 1);
}

#[test]
fn three_accounts_converge_concurrent_edits_role_change() {
    let (mut w, vault, items) = trio(11);
    // Concurrent offline edits on three devices of two accounts.
    w.set_password(0, 0, vault, items[0], "from-a0");
    w.set_password(0, 1, vault, items[1], "from-a1");
    w.set_password(1, 0, vault, items[2], "from-b0");
    let new_item = w.add_login(1, 0, vault, "added by b");
    // A lowers B from manager to writer while that happens.
    let b = w.id(1);
    {
        let dev = w.dev(0, 0);
        dev.engine
            .set_member_role(&mut dev.access, vault, &b, Role::Write)
            .unwrap();
    }
    w.converge();
    let mut all = items.clone();
    all.push(new_item);
    assert_agree(&w, vault, &[0, 1, 2], &all);
    assert_eq!(
        w.password(2, 0, vault, items[0]).as_deref(),
        Some("from-a0")
    );
    assert_eq!(
        w.password(2, 0, vault, items[1]).as_deref(),
        Some("from-a1")
    );
    assert_eq!(
        w.password(2, 0, vault, items[2]).as_deref(),
        Some("from-b0")
    );
    assert!(w.has(2, 0, vault, new_item));
    // The role change reached B through the roster.
    assert_eq!(w.dev(1, 0).s().vault_own_role(vault).unwrap(), Role::Write);
    // C only reads: its edit stays queued and never reaches the others.
    w.set_password(2, 0, vault, items[0], "from-c-reader");
    w.converge();
    assert_eq!(
        w.password(0, 0, vault, items[0]).as_deref(),
        Some("from-a0")
    );
    assert_eq!(w.generation(0, 0, vault).get(), 1, "no rotation needed");
    assert_no_findings(&w, &[0, 1, 2]);
}

#[test]
fn revoke_c_while_c_offline_edits() {
    let (mut w, vault, items) = trio_with(12, Role::Manage, Role::Write);
    w.set_password(2, 0, vault, items[0], "c-offline");
    w.remove(0, 0, vault, 2).unwrap();
    w.converge();
    assert_agree(&w, vault, &[0, 1], &items);
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    assert_eq!(w.generation(1, 0, vault).get(), 2);
    // C never gets the new key, loses access and its edit never arrives.
    assert_eq!(w.generation(2, 0, vault).get(), 1);
    assert!(w.dev(2, 0).s().vault_access_lost(vault).unwrap());
    assert_ne!(
        w.password(0, 0, vault, items[0]).as_deref(),
        Some("c-offline")
    );
    // New items stay with A and B.
    let secret = w.add_login(0, 0, vault, "after the removal");
    w.converge();
    assert!(w.has(1, 0, vault, secret));
    assert!(!w.has(2, 0, vault, secret));
    assert_eq!(w.dev(0, 0).s().pending_revocations().unwrap().len(), 0);
    assert_no_findings(&w, &[0, 1]);
}

#[test]
fn concurrent_rotations_two_owner_devices_no_adoption() {
    let (mut w, vault, items) = trio_with(13, Role::Manage, Role::Write);
    let c = w.id(2);
    // Both devices of the owner carry the same queued revocation.
    for d in 0..2 {
        w.dev(0, d)
            .s_mut()
            .request_revocation(vault, &c, RevocationReason::Removed)
            .unwrap();
    }
    w.sync(0, 0);
    w.sync(0, 1);
    w.converge();
    assert_agree(&w, vault, &[0, 1], &items);
    let g = w.generation(0, 0, vault);
    assert!(g.get() >= 2);
    // The second device took the first one's key from the server (a key row)
    // and never adopted anything.
    for d in 0..2 {
        assert!(
            w.dev(0, d)
                .s()
                .vault_key_origin(vault, g)
                .unwrap()
                .is_some()
        );
    }
    assert_no_findings(&w, &[0, 1]);
    assert!(w.dev(2, 0).s().vault_access_lost(vault).unwrap());
}

#[test]
fn revocation_under_partition_duplicate_delivery() {
    let (mut w, vault, items) = trio_with(14, Role::Manage, Role::Write);
    let c = w.id(2);
    w.server.faults.outage.store(true, Ordering::SeqCst);
    assert!(w.remove(0, 0, vault, 2).is_err());
    // The revocation is queued for good.
    assert_eq!(
        w.dev(0, 0)
            .s()
            .pending_revocations()
            .unwrap()
            .iter()
            .map(|(_, a, _)| *a)
            .collect::<Vec<_>>(),
        vec![c]
    );
    w.server.faults.outage.store(false, Ordering::SeqCst);
    // The outage may have put the engines into backoff.
    w.clock.advance_ms(20 * 60_000);
    // Failing, lossy and duplicating calls on every device of A and B.
    w.chaos = FLAKY;
    for a in 0..2 {
        for d in 0..w.accts[a].devs.len() {
            w.dev(a, d).engine.transport_mut().config = FLAKY;
        }
    }
    w.converge();
    assert_agree(&w, vault, &[0, 1], &items);
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    assert!(w.dev(2, 0).s().vault_access_lost(vault).unwrap());
    assert_eq!(w.dev(0, 0).s().pending_revocations().unwrap().len(), 0);
}

#[test]
fn member_offline_across_three_rotations_walks_chain() {
    let (mut w, vault, items) = trio(15);
    for _ in 0..3 {
        w.rotate(0, 0, vault).unwrap();
    }
    let latest = w.add_login(0, 0, vault, "after three rotations");
    w.sync(0, 0);
    w.sync(0, 1);
    // B and C were offline for all of it: each claims the last key through
    // the chain of the three shares.
    w.converge();
    let mut all = items.clone();
    all.push(latest);
    assert_agree(&w, vault, &[0, 1, 2], &all);
    assert_eq!(w.generation(1, 0, vault).get(), 4);
    assert_eq!(w.generation(2, 0, vault).get(), 4);
    assert_no_findings(&w, &[0, 1, 2]);
}

#[test]
fn invite_accept_races_rotation() {
    let mut w = SWorld::with_accounts(16, 3);
    let vault = w.new_vault(0, 0, "Shared");
    let item = w.add_login(0, 0, vault, "item");
    w.sync_all();
    w.invite(0, 0, vault, 2, Role::Write).unwrap();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    // C previews at generation 1 ...
    let invite = w.incoming(2, 0, vault);
    {
        let dev = w.dev(2, 0);
        let preview = dev.engine.preview_invite(&mut dev.access, &invite).unwrap();
        let sender = dev
            .s()
            .contacts()
            .unwrap()
            .into_iter()
            .find(|c| c.account_id == preview.sender)
            .unwrap();
        dev.s_mut()
            .verify_contact(&preview.sender, &sender.fingerprint)
            .unwrap();
    }
    // ... the vault rotates (the pending invites get fresh seals) ...
    w.rotate(0, 0, vault).unwrap();
    // ... and C's accept of the old preview is refused: ask again.
    {
        let dev = w.dev(2, 0);
        let stale = dev.engine.accept_invite(&mut dev.access, &invite);
        assert!(
            matches!(
                stale,
                Err(SyncError::Transport(TransportError::Rejected(
                    Rejection::InviteStale {
                        current_generation: 2
                    }
                )))
            ),
            "{stale:?}"
        );
    }
    // B never looked at its invite until now: it opens at generation 2.
    w.accept(1, 0, vault);
    w.accept(2, 0, vault);
    w.converge();
    assert_agree(&w, vault, &[0, 1, 2], &[item]);
    assert_eq!(w.generation(2, 0, vault).get(), 2);
    assert_no_findings(&w, &[0, 1, 2]);
}

#[test]
fn decline_after_delivery_triggers_rotation() {
    let mut w = SWorld::with_accounts(17, 3);
    let vault = w.new_vault(0, 0, "Shared");
    let item = w.add_login(0, 0, vault, "item");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    w.invite(0, 0, vault, 2, Role::Write).unwrap();
    w.accept(2, 0, vault);
    // B lists the invite (the seal is delivered) and declines.
    let invite = w.incoming(1, 0, vault);
    {
        let dev = w.dev(1, 0);
        dev.engine.preview_invite(&mut dev.access, &invite).unwrap();
        dev.engine
            .decline_invite(&mut dev.access, &invite, false)
            .unwrap();
    }
    w.converge();
    // B may hold the generation-1 key, so the vault moved to generation 2
    // without it.
    assert_agree(&w, vault, &[0, 2], &[item]);
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    assert!(!w.has_vault(1, 0, vault));
    let roster = w.dev(0, 0).s().vault_roster(vault).unwrap().unwrap();
    let b = w.id(1);
    assert!(
        roster
            .iter()
            .any(|m| m.account_id == b && m.role == Role::Removed)
    );
    // The finished invite left A's list.
    let dev = w.dev(0, 0);
    let (_, outgoing) = dev.engine.list_invites(&mut dev.access).unwrap();
    assert!(
        !outgoing
            .iter()
            .any(|o| o.state == InviteState::Declined && o.delivered)
    );
    assert_no_findings(&w, &[0, 2]);
}

#[test]
fn leave_triggers_rotation() {
    let (mut w, vault, items) = trio_with(18, Role::Manage, Role::Write);
    {
        let dev = w.dev(1, 0);
        dev.engine.leave_vault(&mut dev.access, vault).unwrap();
    }
    w.converge();
    assert_agree(&w, vault, &[0, 2], &items);
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    assert_eq!(w.generation(2, 0, vault).get(), 2);
    assert_eq!(w.generation(1, 0, vault).get(), 1);
    assert!(w.dev(1, 0).s().vault_access_lost(vault).unwrap());
    // The owner cannot leave.
    let dev = w.dev(0, 0);
    let refused = dev.engine.leave_vault(&mut dev.access, vault);
    assert!(
        matches!(
            refused,
            Err(SyncError::Transport(TransportError::Rejected(
                Rejection::OwnerCannotLeave
            )))
        ),
        "{refused:?}"
    );
}

#[test]
fn emergency_full_cycle_decline_then_release() {
    const DAY: u64 = 24 * 60 * 60 * 1000;
    let mut w = SWorld::with_accounts(19, 2);
    let vault = w.new_vault(0, 0, "Estate");
    let item = w.add_login(0, 0, vault, "will");
    w.sync_all();
    w.verify(0, 0, 1);
    let (a, b) = (w.id(0), w.id(1));
    let b_email = w.email(1);
    w.fresh(0, 0);
    {
        let dev = w.dev(0, 0);
        dev.engine
            .grant_emergency(&mut dev.access, &b, &b_email, GrantScope::All, 1, &[])
            .unwrap();
    }
    // The vault rotates after the grant: the next run seals the new key.
    w.rotate(0, 0, vault).unwrap();
    w.sync(0, 0);
    // B asks, A refuses.
    {
        let dev = w.dev(1, 0);
        dev.engine.request_emergency(&mut dev.access, &a).unwrap();
    }
    let report = w.sync(0, 0);
    assert_eq!(report.emergency_requests, 1);
    {
        let dev = w.dev(0, 0);
        dev.engine
            .respond_emergency(&mut dev.access, &b, false)
            .unwrap();
    }
    // A refusal starts a cooldown of 24 hours.
    {
        let dev = w.dev(1, 0);
        let again = dev.engine.request_emergency(&mut dev.access, &a);
        assert!(
            matches!(
                again,
                Err(SyncError::Transport(TransportError::Rejected(
                    Rejection::EmergencyCooldown { .. }
                )))
            ),
            "{again:?}"
        );
    }
    w.server.advance_ms(DAY + 60_000);
    {
        let dev = w.dev(1, 0);
        let release_at = dev.engine.request_emergency(&mut dev.access, &a).unwrap();
        assert!(release_at > 0);
        // Not before the wait is over.
        let early = dev.engine.claim_emergency(&mut dev.access, &a);
        assert!(
            matches!(
                early,
                Err(SyncError::Transport(TransportError::Rejected(
                    Rejection::EmergencyWaiting { .. }
                )))
            ),
            "{early:?}"
        );
    }
    w.server.advance_ms(DAY + 60_000);
    let claimed = {
        let dev = w.dev(1, 0);
        dev.engine.claim_emergency(&mut dev.access, &a).unwrap()
    };
    assert_eq!(claimed, 1);
    w.converge();
    // B reads the vault, read-only, at the current generation.
    assert!(w.has(1, 0, vault, item));
    assert_eq!(w.generation(1, 0, vault).get(), 2);
    assert_eq!(w.dev(1, 0).s().vault_own_role(vault).unwrap(), Role::Read);
    assert_no_findings(&w, &[0, 1]);
}

#[test]
fn view_digest_detects_meta_split() {
    // R70: the server withholds a vault metadata change from one device while
    // the head and the records are what the other device sees. The digest
    // covers the metadata, so the two devices' views differ.
    let (mut w, vault, _items) = trio(20);
    w.dev(0, 0).s_mut().rename_vault(vault, "Renamed").unwrap();
    w.sync(0, 0);
    // Device 1 of A reads through a path that drops the metadata.
    let (opw, device) = {
        let acct = &w.accts[0];
        (
            acct.devs[1]
                .s()
                .opaque_password(
                    &crate::hosted::pw(crate::hosted::PASSWORD),
                    &acct.secret_key,
                )
                .unwrap(),
            acct.devs[1].device,
        )
    };
    let (email, id) = (w.email(0), w.id(0));
    let transport = w.login_transport(&email, id, device, "device 1, hostile path", &opw);
    let mut hostile = engine_for(MaliciousTransport::new(transport));
    hostile.transport_mut().tamper_pull = Some(Box::new(|page| page.metas.clear()));
    w.server.advance_ms(60_000);
    let clock = w.clock.clone();
    {
        let dev = &mut w.accts[0].devs[1];
        hostile.sync_once(&mut dev.access, &clock).unwrap();
    }
    // Device 0 meets device 1's checkpoint at the same head and the digests
    // differ.
    let report = w.sync(0, 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::ServerInconsistent { vault: v } if *v == vault)),
        "{:?}",
        report.warnings
    );
}

/// Random operations by three accounts for one seed.
#[expect(
    clippy::excessive_nesting,
    reason = "one match over the random operation, each arm a few checks and calls"
)]
fn chaos(seed: u64) {
    let (mut w, vault, initial) = trio_with(seed, Role::Manage, Role::Write);
    let mut rng = SimRng::new(seed.wrapping_mul(7919).wrapping_add(1));
    // Items A and B created: none of them may ever be lost.
    let mut must_exist = initial;
    let mut counter = 0u32;
    let mut c_in = true;
    for _ in 0..14 {
        for _ in 0..(2 + rng.below(3)) {
            counter += 1;
            let roll = rng.below(100);
            // The devices that can write now.
            let mut writers: Vec<(usize, usize)> = vec![(0, 0), (0, 1), (1, 0)];
            if c_in {
                writers.push((2, 0));
            }
            let (a, d) = writers[rng.below(writers.len())];
            match roll {
                0..38 => {
                    let known: Vec<ItemId> = must_exist
                        .iter()
                        .copied()
                        .filter(|i| w.has(a, d, vault, *i))
                        .collect();
                    if !known.is_empty()
                        && w.dev(a, d).s().vault_own_role(vault).unwrap() != Role::Read
                    {
                        let item = known[rng.below(known.len())];
                        w.set_password(a, d, vault, item, &format!("pw-{seed}-{counter}"));
                    }
                }
                38..55 => {
                    if w.has_vault(a, d, vault)
                        && w.dev(a, d).s().vault_own_role(vault).unwrap() != Role::Read
                        && !w.dev(a, d).s().vault_access_lost(vault).unwrap()
                    {
                        let item = w.add_login(a, d, vault, &format!("new-{seed}-{counter}"));
                        if a != 2 {
                            must_exist.push(item);
                        }
                    }
                }
                55..65 => {
                    w.rotate(0, 0, vault).unwrap();
                }
                65..72 => {
                    if c_in {
                        w.remove(0, 0, vault, 2).unwrap();
                        c_in = false;
                    }
                }
                72..80 => {
                    if !c_in {
                        // A re-invite at the generation of the removal is
                        // blocked: the vault rotates first.
                        w.rotate(0, 0, vault).unwrap();
                        w.sync(0, 0);
                        w.sync(2, 0);
                        w.invite(0, 0, vault, 2, Role::Write).unwrap();
                        w.accept(2, 0, vault);
                        c_in = true;
                    }
                }
                80..92 => {
                    let (sa, sd) = [(0, 0), (0, 1), (1, 0), (2, 0)][rng.below(4)];
                    w.chaos = FLAKY;
                    w.dev(sa, sd).engine.transport_mut().config = FLAKY;
                    w.sync(sa, sd);
                    w.chaos = ChaosConfig::default();
                    w.dev(sa, sd).engine.transport_mut().config = ChaosConfig::default();
                }
                _ => {
                    // A partition: one run fails, then the clock passes the
                    // backoff.
                    let (sa, sd) = [(0, 0), (0, 1), (1, 0), (2, 0)][rng.below(4)];
                    w.server.faults.outage.store(true, Ordering::SeqCst);
                    assert!(w.try_sync(sa, sd).is_err());
                    w.server.faults.outage.store(false, Ordering::SeqCst);
                    w.clock.advance_ms(20 * 60_000);
                }
            }
        }
    }
    w.converge();
    let members: Vec<usize> = if c_in { vec![0, 1, 2] } else { vec![0, 1] };
    assert_agree(&w, vault, &members, &must_exist);
    for item in &must_exist {
        assert!(w.has(0, 0, vault, *item), "seed {seed}: {item:?} was lost");
    }
    if !c_in {
        assert!(
            w.generation(2, 0, vault) < w.generation(0, 0, vault),
            "seed {seed}: a removed account holds the current key"
        );
    }
    assert_no_findings(&w, &[0, 1]);
}

#[test]
fn chaos_three_accounts_random_ops() {
    for seed in 1..=6 {
        chaos(seed);
    }
}
