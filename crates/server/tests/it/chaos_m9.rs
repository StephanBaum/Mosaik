//! M9 T12 (sync-chaos), hosted: a device that was revoked while offline, and a
//! vault-key rotation with attachment re-seal that is interrupted at every
//! call while other devices edit. Deterministic: seeded worlds, manual
//! clocks, no sleeps (`chaos_net::Faulty` places the faults).

use mosaic_core::{ItemId, VaultId};
use mosaic_sync::TransportError;
use mosaic_sync::engine::{RekeyError, RotationOutcome, SyncError};
use mosaic_sync::http::hosted as client;
use mosaic_sync::transport::{Rejection, RevokedReason};
use mosaic_vault::session::RotationPhase;

use crate::chaos_net::{
    DAY_MS, FEngine, Fault, MINUTE_MS, Op, anywhere, assert_converged, faulty, frun, fsync, settle,
};
use crate::hosted::{PASSWORD, World, everything, is_idle, pw};
use crate::scenarios::{add_login, generation, has_pw, set_password, three_devices, two_devices};

const ALL: [usize; 3] = [0, 1, 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cause {
    PasswordChange,
    DeviceRemoved,
}

const NEW_PASSWORD: &str = "a new password for the revocation chaos test";

/// Device 0 ends the sessions of device 1 for `cause` and goes on with a
/// session of its own. Returns the reason the server records.
fn revoke(w: &mut World, cause: Cause) -> RevokedReason {
    let device0 = w.devs[0].device;
    match cause {
        Cause::PasswordChange => {
            let prepared = w.devs[0]
                .s()
                .prepare_master_password_change(&pw(PASSWORD), &pw(NEW_PASSWORD), &w.secret_key)
                .unwrap();
            w.server.advance_ms(MINUTE_MS);
            let changed = client::password_change(
                &w.server.base,
                &w.email,
                &prepared,
                device0,
                "device 0",
                None,
            )
            .unwrap();
            let transport = crate::hosted::connect(&w.server, &changed.grant, device0);
            w.devs[0].engine = w.engine_over(transport, 0);
            w.devs[0]
                .s_mut()
                .commit_master_password_change(prepared)
                .unwrap();
            RevokedReason::PasswordChange
        }
        Cause::DeviceRemoved => {
            w.server.advance_ms(MINUTE_MS);
            let grant = client::login(
                &w.server.base,
                &w.email,
                w.account,
                &w.opw(0),
                device0,
                "device 0",
            )
            .unwrap();
            let victim = w.devs[1].device;
            client::device_remove(&w.server.base, &grant.token, victim).unwrap();
            let transport = crate::hosted::connect(&w.server, &grant, device0);
            w.devs[0].engine = w.engine_over(transport, 0);
            RevokedReason::DeviceRemoved
        }
    }
}

/// Signs device `d` in again with `password` and gives it a fresh engine.
fn sign_in_again(w: &mut World, d: usize, password: &str) {
    let opw = w.devs[0]
        .s()
        .opaque_password(&pw(password), &w.secret_key)
        .unwrap();
    let device = w.devs[d].device;
    w.server.advance_ms(MINUTE_MS);
    let grant = client::login(&w.server.base, &w.email, w.account, &opw, device, "device").unwrap();
    let transport = crate::hosted::connect(&w.server, &grant, device);
    w.devs[d].engine = w.engine_over(transport, d);
}

fn w_requests(w: &World) -> u64 {
    w.server
        .faults
        .requests
        .load(std::sync::atomic::Ordering::SeqCst)
}

/// A device is revoked while it is offline with unsent edits. When it comes
/// back, within the 30 days the server remembers the reason for or after
/// the purge, it gets a sane error (the reason, then a plain 401), makes one
/// request per run (no loop), keeps its edits, and, where the account still
/// accepts the device, signs in again and everything converges.
fn revoked_while_offline(cause: Cause, purged: bool, seed: u64) {
    let (mut w, vault, item) = two_devices(seed);
    // Device 1 has a session of its own that it uses through a scripted
    // transport, so its calls can be counted.
    let mut e: FEngine = faulty(&w, 1);
    fsync(&mut w, &mut e, 1);

    // Offline: unsent edits on device 1.
    set_password(&mut w, 1, vault, item, "device 1, offline");
    let extra = add_login(&mut w, 1, vault, "added offline by device 1");
    assert_ne!(w.devs[1].s().pending_uploads(10).unwrap().len(), 0);

    let reason = revoke(&mut w, cause);
    set_password(&mut w, 0, vault, item, "device 0, after the revocation");
    w.sync(0);

    // The device stays away for a while.
    let away = if purged { 31 * DAY_MS } else { 3 * DAY_MS };
    w.server.advance_ms(away);
    w.clock.advance_ms(away);
    w.server.state.maintain().unwrap();
    // The other device is online all along: it signs in again (after 31 days
    // its own session idled out too).
    let password = match cause {
        Cause::PasswordChange => NEW_PASSWORD,
        Cause::DeviceRemoved => PASSWORD,
    };
    sign_in_again(&mut w, 0, password);

    let before = w_requests(&w);
    for run in 0..4 {
        match frun(&mut w, &mut e, 1) {
            Err(SyncError::Transport(TransportError::Revoked(got))) if !purged => {
                assert_eq!(got, reason, "run {run}");
            }
            Err(SyncError::Transport(TransportError::Unauthorized)) if purged => {}
            other => panic!("{cause:?}, purged {purged}, run {run}: {other:?}"),
        }
    }
    let calls = w_requests(&w) - before;
    assert!(
        (4..=8).contains(&calls),
        "{calls} requests in 4 runs of a revoked device"
    );
    // Nothing of the device's own work was touched.
    assert_ne!(w.devs[1].s().pending_uploads(10).unwrap().len(), 0);
    assert!(has_pw(&w, 1, vault, item, "device 1, offline"));

    let device1 = w.devs[1].device;
    w.server.advance_ms(MINUTE_MS);
    match cause {
        Cause::PasswordChange => {
            // It signs in with the new password and its offline work
            // reaches everyone.
            let opw = w.devs[0]
                .s()
                .opaque_password(&pw(NEW_PASSWORD), &w.secret_key)
                .unwrap();
            let grant = client::login(
                &w.server.base,
                &w.email,
                w.account,
                &opw,
                device1,
                "device 1",
            )
            .unwrap();
            let transport = crate::hosted::connect(&w.server, &grant, device1);
            w.devs[1].engine = w.engine_over(transport, 1);
            drop(e);
            for _ in 0..3 {
                w.sync(0);
                w.sync(1);
            }
            for d in 0..2 {
                assert!(
                    has_pw(&w, d, vault, item, "device 1, offline"),
                    "device {d}"
                );
                assert!(
                    has_pw(&w, d, vault, item, "device 0, after the revocation"),
                    "device {d}"
                );
                assert!(w.devs[d].s().item(vault, extra).is_ok(), "device {d}");
            }
        }
        Cause::DeviceRemoved => {
            // A removed device cannot sign in again (it joins as a new
            // install); it says so, and its unsent work stays on it.
            let err = client::login(
                &w.server.base,
                &w.email,
                w.account,
                &w.opw(1),
                device1,
                "device 1",
            )
            .err();
            assert_eq!(
                err,
                Some(TransportError::Rejected(Rejection::DeviceRevoked))
            );
            assert_ne!(w.devs[1].s().pending_uploads(10).unwrap().len(), 0);
            // A new install joins and sees what device 0 holds.
            let d = w.join();
            for _ in 0..3 {
                w.sync(0);
                w.sync(d);
            }
            assert!(has_pw(&w, d, vault, item, "device 0, after the revocation"));
        }
    }
}

#[test]
fn password_change_while_offline_gives_the_reason_and_recovers() {
    revoked_while_offline(Cause::PasswordChange, false, 6001);
}

#[test]
fn password_change_after_the_30_day_purge_is_a_plain_401_and_recovers() {
    revoked_while_offline(Cause::PasswordChange, true, 6002);
}

#[test]
fn device_removal_while_offline_gives_the_reason() {
    revoked_while_offline(Cause::DeviceRemoved, false, 6003);
}

#[test]
fn device_removal_after_the_30_day_purge_is_a_plain_401() {
    revoked_while_offline(Cause::DeviceRemoved, true, 6004);
}

// ---- a vault-key rotation with re-seal, interrupted at every call ----------------

/// The app's retry loop for a rotation with re-seal: resume a job past the
/// key commit ("Continue"), otherwise rotate again (a retry settles a key
/// offer whose outcome is not known, M9-F1), until it reports finished.
fn drive_rotation(w: &mut World, e: &mut FEngine, vault: VaultId) {
    for _ in 0..12 {
        w.server.advance_ms(MINUTE_MS);
        let clock = w.clock.clone();
        let job = w.devs[0].s().rotation_job().unwrap();
        let result = if job.is_some_and(|j| j.phase >= RotationPhase::LocalCommitted) {
            e.vault_rotate_continue(&mut w.devs[0].access, &clock)
                .map(|p| p.finished)
        } else {
            e.rotate_vault_key(&mut w.devs[0].access, &clock, vault, true)
                .map(|r| r.progress.is_none_or(|p| p.finished))
        };
        match result {
            Ok(true) => return,
            Err(RekeyError::Sync(SyncError::Backoff { retry_at_ms })) => {
                let now = mosaic_sync::clock::Clock::now_ms(&w.clock);
                w.clock.advance_ms(retry_at_ms.saturating_sub(now) + 1);
            }
            // Not finished, or a call failed: the app retries.
            Ok(false)
            | Err(
                RekeyError::Sync(SyncError::Transport(_))
                | RekeyError::Transport(_)
                | RekeyError::OutcomeUnknown,
            ) => {}
            Err(other) => panic!("the rotation failed for good: {other:?}"),
        }
    }
    panic!("the rotation never finished");
}

fn settle_with(w: &mut World, e: &mut FEngine) {
    for _ in 0..10 {
        let mut idle = true;
        idle &= is_idle(&fsync(w, e, 0));
        idle &= is_idle(&w.sync(1));
        idle &= is_idle(&w.sync(2));
        if idle {
            return;
        }
    }
    panic!("no rest after 10 passes");
}

fn title_exists(w: &World, d: usize, title: &str) -> bool {
    w.devs[d]
        .s()
        .items(&everything())
        .iter()
        .any(|r| r.item.title() == Some(title))
}

/// Device 0 rotates the vault key and re-seals its attachments, with a
/// fault (a failed call, or a lost response after the effect) at each call
/// of the run in turn, while device 1 edits before and during the
/// interruption and device 2 edits offline. After the app's retries and full
/// delivery: nothing is lost, every attachment opens with its content, no
/// queue is left, and all devices share one higher generation.
#[test]
fn rotation_with_reseal_interrupted_in_every_phase_loses_nothing() {
    rotation_interrupted(7001, false);
}

/// The same with a fault at every single call of the run.
#[test]
#[ignore = "long-running; run with --release -- --ignored"]
fn rotation_with_reseal_interrupted_at_every_call_loses_nothing() {
    rotation_interrupted(7002, true);
}

/// Mutating calls: a lost response after the effect differs from a failure
/// before it.
const MUTATING: [Op; 6] = [
    Op::PushKey,
    Op::PushRecords,
    Op::PushMeta,
    Op::PutAttachment,
    Op::DeleteAttachment,
    Op::PutCheckpoint,
];

fn rotation_interrupted(seed: u64, exhaustive: bool) {
    let (mut w, vault, items) = three_devices(seed);
    let bodies: Vec<Vec<u8>> = (0..2)
        .map(|i| format!("attachment body {i}").into_bytes())
        .collect();
    for (i, body) in bodies.iter().enumerate() {
        w.devs[0]
            .s_mut()
            .add_attachment(vault, items[i], "a.txt", "text/plain", body)
            .unwrap();
    }
    settle(&mut w, &ALL);
    // Device 0 works through the scripted transport from here on.
    let mut e: FEngine = faulty(&w, 0);
    let log = e.transport_mut().log.clone();

    // Dry run: the calls of a clean rotation with re-seal.
    log.lock().unwrap().clear();
    drive_rotation(&mut w, &mut e, vault);
    settle_with(&mut w, &mut e);
    let ops: Vec<Op> = log.lock().unwrap().clone();
    assert!(ops.contains(&Op::PushKey), "{ops:?}");
    assert!(ops.len() > 5, "{ops:?}");

    // The first call of every kind, the second of every mutating kind and
    // the last call: every phase of the run (pull, key push, record push,
    // metadata, attachment upload and removal, checkpoint) is interrupted, at
    // its start and again in its repeat. A lost response only differs from a
    // failed call where the call changes something.
    let picked: Vec<usize> = (0..ops.len())
        .filter(|&k| {
            let same_before = ops[..k].iter().filter(|o| **o == ops[k]).count();
            exhaustive
                || k + 1 == ops.len()
                || same_before == 0
                || (same_before == 1 && MUTATING.contains(&ops[k]))
        })
        .collect();
    let mut generation_before = generation(&w, 0, vault);
    let mut n = 0usize;
    for k in picked {
        let op = &ops[k];
        for fault in [Fault::FailBefore, Fault::LoseResponse] {
            if fault == Fault::LoseResponse && !MUTATING.contains(op) {
                continue;
            }
            n += 1;
            let label = format!("call {k} ({op:?}), {fault:?}, of {ops:?}");
            let (a, b, c) = (format!("a{n}"), format!("b{n}"), format!("c{n}"));
            // Edits under the old key, offline on devices 1 and 2 and on 0.
            set_password(&mut w, 1, vault, items[2], &a);
            set_password(&mut w, 2, vault, items[1], &b);
            w.devs[0]
                .s_mut()
                .update_item(vault, items[0], |it| {
                    it.set_secret(mosaic_core::model::SecretField::Password, &c)
                })
                .unwrap();

            let same_before = ops[..k].iter().filter(|o| *o == op).count();
            let at = e.transport_mut().count(*op) + same_before;
            e.transport_mut().faults.push((*op, at, fault));
            let first = {
                let clock = w.clock.clone();
                w.server.advance_ms(MINUTE_MS);
                e.rotate_vault_key(&mut w.devs[0].access, &clock, vault, true)
            };
            // While device 0 is interrupted, device 1 edits and syncs.
            let during = format!("during {n}");
            add_login(&mut w, 1, vault, &during);
            w.sync(1);
            // The app retries.
            let _ = first;
            drive_rotation(&mut w, &mut e, vault);
            e.transport_mut().faults.clear();
            settle_with(&mut w, &mut e);
            settle_with(&mut w, &mut e);

            assert_converged(&mut w, &ALL);
            let g = generation(&w, 0, vault);
            assert!(g > generation_before, "{label}: no new generation");
            for d in ALL {
                assert_eq!(generation(&w, d, vault), g, "{label}: device {d}");
                for value in [&a, &b, &c] {
                    assert!(anywhere(&w, d, value), "{label}: {value} lost on {d}");
                }
                assert!(title_exists(&w, d, &during), "{label}: item lost on {d}");
                for (i, body) in bodies.iter().enumerate() {
                    check_attachment(&w, d, vault, items[i], body, &label);
                }
            }
            generation_before = g;
        }
    }
}

/// M9-F1 follow-up, hosted: the key offer fails before it reaches the
/// server (a phase-2 job is left) and a plain retry of `rotate_vault_key`
/// rotates once and re-seals; after a lost answer to the offer the retry
/// finishes the landed key without a second rotation.
#[test]
fn a_retry_after_a_failed_key_offer_rotates_once() {
    let (mut w, vault, items) = three_devices(7003);
    let body = b"attachment body".to_vec();
    w.devs[0]
        .s_mut()
        .add_attachment(vault, items[0], "a.txt", "text/plain", &body)
        .unwrap();
    settle(&mut w, &ALL);
    let mut e: FEngine = faulty(&w, 0);
    for fault in [Fault::FailBefore, Fault::LoseResponse] {
        let next = generation(&w, 0, vault).next().unwrap();
        e.transport_mut().next(Op::PushKey, fault);
        let clock = w.clock.clone();
        w.server.advance_ms(MINUTE_MS);
        let first = e.rotate_vault_key(&mut w.devs[0].access, &clock, vault, true);
        assert!(first.is_err(), "{fault:?}: {first:?}");
        let job = w.devs[0].s().rotation_job().unwrap().unwrap();
        assert_eq!(job.phase, RotationPhase::BackendStarted, "{fault:?}");

        w.server.advance_ms(MINUTE_MS);
        let done = e
            .rotate_vault_key(&mut w.devs[0].access, &clock, vault, true)
            .unwrap();
        assert_eq!(done.outcome, RotationOutcome::Rotated(next), "{fault:?}");
        assert!(done.progress.is_some_and(|p| p.finished), "{fault:?}");
        assert!(w.devs[0].s().rotation_job().unwrap().is_none());
        settle_with(&mut w, &mut e);
        let label = format!("{fault:?}");
        for d in ALL {
            assert_eq!(generation(&w, d, vault), next, "{label}: device {d}");
            check_attachment(&w, d, vault, items[0], &body, &label);
        }
    }
}

fn check_attachment(w: &World, d: usize, vault: VaultId, item: ItemId, body: &[u8], label: &str) {
    let s = w.devs[d].s();
    let refs = s.item(vault, item).unwrap().attachments();
    assert_eq!(refs.len(), 1, "{label}: device {d} attachments of {item:?}");
    let plain = s
        .attachment(vault, item, refs[0].attachment_id)
        .unwrap_or_else(|e| panic!("{label}: device {d} cannot open the attachment: {e:?}"));
    assert!(
        plain.expose_secret() == body,
        "{label}: device {d} attachment differs"
    );
}
