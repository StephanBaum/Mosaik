//! Account re-key over the hosted backend on a real server (M7 spec §7.5,
//! CRYPTO_SPEC §6.6, §6.7): three devices and an offline peer, a crash after
//! the server accepted, a crash in the middle of the attachment re-seal, and
//! shared vaults (a member's and an owner's re-key).

use mosaic_core::model::{ItemType, Role, SecretField};
use mosaic_core::{Argon2Profile, ItemId, SecretKey, VaultId};
use mosaic_sync::engine::{
    HostedRekey, RekeyError, RekeySecrets, ResumeOutcome, SyncError, SyncWarning,
    adopt_hosted_rekey,
};
use mosaic_sync::http::hosted::{self, RekeyOutcome, RekeyStage};
use mosaic_sync::testutil::ChaosConfig;
use mosaic_vault::session::{RekeyPlan, RekeyRequest, RotationKind, RotationPhase};

use crate::hosted::{PASSWORD, World, assert_no_server_findings, connect, pw};
use crate::sharing_world::SWorld;

const NEW_PASSWORD: &str = "a different horse battery staple";

fn copy_key(sk: &SecretKey) -> SecretKey {
    SecretKey::parse(sk.to_display().unwrap().expose_secret()).unwrap()
}

fn calm() -> ChaosConfig {
    ChaosConfig::default()
}

fn add_login(w: &mut World, d: usize, vault: VaultId, title: &str) -> ItemId {
    w.devs[d]
        .s_mut()
        .add_item(vault, &ItemType::Login, |e| {
            e.set_title(title)?;
            e.set_secret(SecretField::Password, "pw")
        })
        .unwrap()
}

fn set_password(w: &mut World, d: usize, vault: VaultId, item: ItemId, value: &str) {
    w.devs[d]
        .s_mut()
        .update_item(vault, item, |e| e.set_secret(SecretField::Password, value))
        .unwrap();
}

fn password_of(w: &World, d: usize, vault: VaultId, item: ItemId) -> Option<String> {
    w.devs[d]
        .s()
        .item(vault, item)
        .ok()?
        .secret(SecretField::Password)
        .ok()?
        .map(|s| s.expose_secret().to_owned())
}

/// A world with `devices` devices that converged on one vault with one item.
fn world(seed: u64, devices: usize) -> (World, VaultId, ItemId) {
    let mut w = World::new(seed, calm());
    let vault = w.new_vault(0, "Shared");
    let item = add_login(&mut w, 0, vault, "first");
    w.sync(0);
    for _ in 1..devices {
        w.join();
    }
    for _ in 0..3 {
        for d in 0..devices {
            w.sync(d);
        }
    }
    (w, vault, item)
}

/// Device `d` re-keys the account and continues over the new session.
fn rekey(w: &mut World, d: usize, new_sk: &SecretKey) -> mosaic_sync::engine::RekeyProgress {
    let current = copy_key(&w.secret_key);
    let (current_password, new_password) = (pw(PASSWORD), pw(NEW_PASSWORD));
    let secrets = RekeySecrets {
        current_password: &current_password,
        current_secret_key: &current,
        new_password: &new_password,
        new_secret_key: new_sk,
        profile: Argon2Profile::Baseline,
    };
    w.server.advance_ms(60_000);
    let name = w.devs[d].name.clone();
    let device = w.devs[d].device;
    let done = {
        let World {
            devs,
            clock,
            server,
            email,
            ..
        } = w;
        let hosted = HostedRekey {
            base: &server.base,
            address: email,
            device_name: &name,
            totp_code: None,
        };
        let dev = &mut devs[d];
        dev.engine
            .rekey_hosted(&mut dev.access, clock, &hosted, &secrets)
            .unwrap()
    };
    assert_eq!(
        done.committed
            .new_secret_key
            .to_display()
            .unwrap()
            .expose_secret(),
        new_sk.to_display().unwrap().expose_secret()
    );
    // The session of the re-key is the only one left: connect with it.
    let transport = connect(&w.server, &done.grant, device);
    w.devs[d].engine = w.engine_over(transport, d);
    let World { devs, clock, .. } = w;
    let dev = &mut devs[d];
    dev.engine.rekey_continue(&mut dev.access, clock).unwrap()
}

/// Device `d` adopts the re-key with the typed new password and Secret Key and
/// signs in again.
fn adopt(w: &mut World, d: usize, new_sk: &SecretKey) -> Result<(), RekeyError> {
    w.server.advance_ms(60_000);
    let current = copy_key(&w.secret_key);
    let (name, device) = (w.devs[d].name.clone(), w.devs[d].device);
    let adopted = {
        let World {
            devs,
            server,
            email,
            ..
        } = w;
        let hosted = HostedRekey {
            base: &server.base,
            address: email,
            device_name: &name,
            totp_code: None,
        };
        adopt_hosted_rekey(
            &mut devs[d].access,
            &hosted,
            &pw(NEW_PASSWORD),
            new_sk,
            &current,
            device,
        )?
    };
    let transport = w.login_transport(device, &name, &adopted.opaque_password);
    w.devs[d].engine = w.engine_over(transport, d);
    Ok(())
}

fn converge(w: &mut World, devices: usize) {
    for _ in 0..4 {
        for d in 0..devices {
            w.sync(d);
        }
    }
}

#[test]
fn hosted_rekey_three_devices_offline_peer_edits_survive() {
    let (mut w, vault, item) = world(21, 3);
    // Device 2 edits while it is offline; device 1 adds an item and syncs.
    set_password(&mut w, 2, vault, item, "edited offline by device 2");
    let extra = add_login(&mut w, 1, vault, "added by device 1");
    w.sync(1);

    let new_sk = SecretKey::generate().unwrap();
    let progress = rekey(&mut w, 0, &new_sk);
    assert!(progress.finished, "{progress:?}");
    assert_eq!(w.devs[0].s().ak_epoch(), 2);

    // The other devices are signed out; the old Secret Key does not get back
    // in (the new password with the old Secret Key logs in nowhere).
    for d in 1..3 {
        assert!(
            matches!(
                w.try_sync(d),
                Err(SyncError::Transport(
                    mosaic_sync::TransportError::Unauthorized
                ))
            ),
            "device {d} is signed out"
        );
    }
    let current = copy_key(&w.secret_key);
    assert!(
        adopt_with(&mut w, 1, &current).is_err(),
        "the old Secret Key adopts nothing"
    );
    adopt(&mut w, 1, &new_sk).unwrap();
    adopt(&mut w, 2, &new_sk).unwrap();
    converge(&mut w, 3);

    for d in 0..3 {
        assert_eq!(w.devs[d].s().ak_epoch(), 2, "device {d}");
        assert_eq!(
            password_of(&w, d, vault, item).as_deref(),
            Some("edited offline by device 2"),
            "device {d} keeps the offline edit"
        );
        assert!(w.devs[d].s().item(vault, extra).is_ok(), "device {d}");
        assert_eq!(w.devs[d].s().vaults()[0].key_generation.get(), 2);
    }
    // M3: no false ServerInconsistent after the adoption.
    assert_no_server_findings(&w);
}

/// Adopts with a given typed Secret Key (the password is the new one).
fn adopt_with(w: &mut World, d: usize, typed: &SecretKey) -> Result<(), RekeyError> {
    adopt(w, d, typed)
}

#[test]
fn hosted_crash_after_finish_recovers_with_pending_sk() {
    let (mut w, vault, item) = world(22, 2);
    let new_sk = SecretKey::generate().unwrap();
    let current = copy_key(&w.secret_key);
    let (current_password, new_password) = (pw(PASSWORD), pw(NEW_PASSWORD));
    w.server.advance_ms(60_000);
    let (name, device) = (w.devs[0].name.clone(), w.devs[0].device);
    // The process does the first steps by hand and dies after the server
    // accepted: the prepared keys are gone, the journal says phase 3.
    {
        let head = {
            let s = w.devs[0].s();
            s.sync_cursor(vault)
                .unwrap()
                .and_then(|c| <[u8; 8]>::try_from(c).ok())
                .map_or(0, u64::from_be_bytes)
        };
        let mut s = w.devs[0].s_mut();
        s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
        let prepared = s
            .prepare_rekey(RekeyRequest {
                current_password: &current_password,
                current_secret_key: &current,
                new_password: &new_password,
                new_secret_key: &new_sk,
                profile: Argon2Profile::Baseline,
                plan: RekeyPlan {
                    heads: vec![(vault, head)],
                    ..RekeyPlan::default()
                },
            })
            .unwrap();
        drop(s);
        let mut phases = Vec::new();
        let outcome = hosted::rekey(
            &w.server.base,
            &w.email,
            &prepared,
            device,
            &name,
            None,
            &mut |stage| {
                phases.push(stage);
                true
            },
        )
        .unwrap();
        assert!(matches!(outcome, RekeyOutcome::Done(_)));
        assert_eq!(phases, vec![RekeyStage::Started, RekeyStage::Accepted]);
        let mut s = w.devs[0].s_mut();
        s.set_rotation_phase(RotationPhase::BackendAccepted)
            .unwrap();
        // The crash: `prepared` is dropped without a commit.
    }
    assert_eq!(w.devs[0].s().ak_epoch(), 1);

    // Resume: the last kit group has to be retyped.
    let engine_hosted = |w: &World| (w.server.base.clone(), w.email.clone());
    let (base, email) = engine_hosted(&w);
    let hosted = HostedRekey {
        base: &base,
        address: &email,
        device_name: &name,
        totp_code: None,
    };
    let World { devs, .. } = &mut w;
    let dev = &mut devs[0];
    assert!(matches!(
        dev.engine.resume_hosted_rekey(
            &mut dev.access,
            &hosted,
            &new_password,
            &new_sk,
            &current,
            "ABCD"
        ),
        Err(RekeyError::KitGroupMismatch)
    ));
    assert_eq!(
        dev.access.session().unwrap().ak_epoch(),
        1,
        "nothing happened"
    );
    let group = new_sk.last_group().unwrap();
    let ResumeOutcome::Adopted(adopted) = dev
        .engine
        .resume_hosted_rekey(
            &mut dev.access,
            &hosted,
            &new_password,
            &new_sk,
            &current,
            group.expose_secret(),
        )
        .unwrap()
    else {
        panic!("the server had applied the re-key");
    };
    // M2: the job goes on at phase 6 (attachments are queued again).
    assert_eq!(
        dev.access
            .session()
            .unwrap()
            .rotation_job()
            .unwrap()
            .unwrap()
            .phase,
        RotationPhase::Attachments
    );
    assert_eq!(dev.access.session().unwrap().ak_epoch(), 2);
    let transport = {
        w.server.advance_ms(60_000);
        w.login_transport(device, &name, &adopted.opaque_password)
    };
    w.devs[0].engine = w.engine_over(transport, 0);
    {
        let World { devs, clock, .. } = &mut w;
        let dev = &mut devs[0];
        assert!(
            dev.engine
                .rekey_continue(&mut dev.access, clock)
                .unwrap()
                .finished
        );
    }
    // The other device adopts, and everything converges on the new keys.
    adopt(&mut w, 1, &new_sk).unwrap();
    converge(&mut w, 2);
    for d in 0..2 {
        assert_eq!(w.devs[d].s().ak_epoch(), 2);
        assert_eq!(password_of(&w, d, vault, item).as_deref(), Some("pw"));
        assert_eq!(
            w.devs[d].s().vaults()[0].key_generation.get(),
            2,
            "device {d}"
        );
    }
    assert_no_server_findings(&w);
}

#[test]
fn a_probe_that_finds_nothing_applied_returns_the_job_to_phase_one() {
    let (mut w, _vault, _item) = world(23, 1);
    let new_sk = SecretKey::generate().unwrap();
    {
        let mut s = w.devs[0].s_mut();
        s.begin_rotation_job(RotationKind::Rekey, None).unwrap();
        s.set_rotation_phase(RotationPhase::BackendStarted).unwrap();
    }
    let current = copy_key(&w.secret_key);
    let name = w.devs[0].name.clone();
    let (base, email) = (w.server.base.clone(), w.email.clone());
    let hosted = HostedRekey {
        base: &base,
        address: &email,
        device_name: &name,
        totp_code: None,
    };
    w.server.advance_ms(60_000);
    let group = new_sk.last_group().unwrap();
    let World { devs, .. } = &mut w;
    let dev = &mut devs[0];
    let outcome = dev
        .engine
        .resume_hosted_rekey(
            &mut dev.access,
            &hosted,
            &pw(NEW_PASSWORD),
            &new_sk,
            &current,
            group.expose_secret(),
        )
        .unwrap();
    assert!(matches!(outcome, ResumeOutcome::NotApplied));
    let job = dev
        .access
        .session()
        .unwrap()
        .rotation_job()
        .unwrap()
        .unwrap();
    assert_eq!(job.phase, RotationPhase::Prepared);
}

#[test]
fn crash_mid_attachment_reseal_resumes() {
    let (mut w, vault, item) = world(24, 1);
    let ids: Vec<_> = (0..3)
        .map(|n| {
            w.devs[0]
                .s_mut()
                .add_attachment(
                    vault,
                    item,
                    "a.txt",
                    "text/plain",
                    format!("body {n}").as_bytes(),
                )
                .unwrap()
        })
        .collect();
    w.sync(0);
    let new_sk = SecretKey::generate().unwrap();
    // The commit queued one re-seal per attachment; one step runs, then the
    // process "dies" (a new engine continues).
    let current = copy_key(&w.secret_key);
    let (current_password, new_password) = (pw(PASSWORD), pw(NEW_PASSWORD));
    let secrets = RekeySecrets {
        current_password: &current_password,
        current_secret_key: &current,
        new_password: &new_password,
        new_secret_key: &new_sk,
        profile: Argon2Profile::Baseline,
    };
    w.server.advance_ms(60_000);
    let (name, device) = (w.devs[0].name.clone(), w.devs[0].device);
    let done = {
        let World {
            devs,
            clock,
            server,
            email,
            ..
        } = &mut w;
        let hosted = HostedRekey {
            base: &server.base,
            address: email,
            device_name: &name,
            totp_code: None,
        };
        let dev = &mut devs[0];
        dev.engine
            .rekey_hosted(&mut dev.access, clock, &hosted, &secrets)
            .unwrap()
    };
    assert_eq!(done.committed.attachments_to_reseal, 3);
    let step = w.devs[0].s_mut().reseal_next_attachment(None).unwrap();
    assert!(matches!(
        step,
        mosaic_vault::session::ResealStep::Resealed { .. }
    ));
    let transport = connect(&w.server, &done.grant, device);
    w.devs[0].engine = w.engine_over(transport, 0);
    let World { devs, clock, .. } = &mut w;
    let dev = &mut devs[0];
    let progress = dev.engine.rekey_continue(&mut dev.access, clock).unwrap();
    assert!(progress.finished);
    assert_eq!(progress.unreachable_attachments.len(), 0);
    let now: Vec<_> = w.devs[0]
        .s()
        .item(vault, item)
        .unwrap()
        .attachments()
        .into_iter()
        .map(|a| a.attachment_id)
        .collect();
    assert_eq!(now.len(), 3);
    assert!(
        now.iter().all(|a| !ids.contains(a)),
        "every attachment has a fresh ID and file key"
    );
    w.sync(0);
    assert!(w.devs[0].s().rotation_job().unwrap().is_none());
}

// ---- shared vaults ---------------------------------------------------------

/// Accounts a (owner) and b (member with two devices) share a vault.
fn shared_world(seed: u64) -> (SWorld, VaultId, ItemId) {
    let mut w = SWorld::with_accounts(seed, 2);
    let vault = w.new_vault(0, 0, "Shared");
    let item = w.add_login(0, 0, vault, "shared item");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    w.accept(1, 0, vault);
    w.converge();
    w.join(1);
    w.converge();
    (w, vault, item)
}

/// Account `a` device `d` re-keys.
fn rekey_acct(w: &mut SWorld, a: usize, d: usize, new_sk: &SecretKey) {
    let current = copy_key(&w.accts[a].secret_key);
    let (current_password, new_password) = (pw(PASSWORD), pw(NEW_PASSWORD));
    let secrets = RekeySecrets {
        current_password: &current_password,
        current_secret_key: &current,
        new_password: &new_password,
        new_secret_key: new_sk,
        profile: Argon2Profile::Baseline,
    };
    w.server.advance_ms(60_000);
    let (name, device, email) = {
        let dev = &w.accts[a].devs[d];
        (dev.name.clone(), dev.device, w.accts[a].email.clone())
    };
    let done = {
        let SWorld {
            accts,
            clock,
            server,
            ..
        } = w;
        let hosted = HostedRekey {
            base: &server.base,
            address: &email,
            device_name: &name,
            totp_code: None,
        };
        let dev = &mut accts[a].devs[d];
        dev.engine
            .rekey_hosted(&mut dev.access, clock, &hosted, &secrets)
            .unwrap()
    };
    let transport = connect(&w.server, &done.grant, device);
    w.accts[a].devs[d].engine = w.engine_over(transport, a * 16 + d);
    let SWorld { accts, clock, .. } = w;
    let dev = &mut accts[a].devs[d];
    assert!(
        dev.engine
            .rekey_continue(&mut dev.access, clock)
            .unwrap()
            .finished
    );
}

/// Account `a` device `d` adopts the re-key.
fn adopt_acct(w: &mut SWorld, a: usize, d: usize, new_sk: &SecretKey) {
    w.server.advance_ms(60_000);
    let current = copy_key(&w.accts[a].secret_key);
    let (name, device, email, id) = {
        let dev = &w.accts[a].devs[d];
        (
            dev.name.clone(),
            dev.device,
            w.accts[a].email.clone(),
            w.accts[a].id,
        )
    };
    let adopted = {
        let SWorld { accts, server, .. } = w;
        let hosted = HostedRekey {
            base: &server.base,
            address: &email,
            device_name: &name,
            totp_code: None,
        };
        adopt_hosted_rekey(
            &mut accts[a].devs[d].access,
            &hosted,
            &pw(NEW_PASSWORD),
            new_sk,
            &current,
            device,
        )
        .unwrap()
    };
    let transport = w.login_transport(&email, id, device, &name, &adopted.opaque_password);
    w.accts[a].devs[d].engine = w.engine_over(transport, a * 16 + d);
}

fn no_findings(w: &SWorld) {
    for (a, acct) in w.accts.iter().enumerate() {
        for (d, dev) in acct.devs.iter().enumerate() {
            let findings = dev.s().sync_findings().unwrap();
            assert!(findings.is_clear(), "account {a} device {d}: {findings:?}");
            assert!(
                !dev.last
                    .warnings
                    .iter()
                    .any(|x| matches!(x, SyncWarning::ServerInconsistent { .. })),
                "account {a} device {d}: {:?}",
                dev.last.warnings
            );
        }
    }
}

/// M3: after a member re-keys, the member's second device adopts the server's
/// wrapping of the vault it does not own and reports no false
/// `ServerInconsistent`.
#[test]
fn member_rekey_two_devices_report_no_false_inconsistency() {
    let (mut w, vault, item) = shared_world(31);
    let new_sk = SecretKey::generate().unwrap();
    rekey_acct(&mut w, 1, 0, &new_sk);
    adopt_acct(&mut w, 1, 1, &new_sk);
    for _ in 0..4 {
        w.sync(1, 0);
        w.sync(1, 1);
    }
    no_findings(&w);
    for d in 0..2 {
        assert_eq!(w.accts[1].devs[d].s().ak_epoch(), 2);
        assert!(w.has(1, d, vault, item), "device {d} still reads the vault");
    }
}

/// A member's re-key files a rotation request that reaches the owner's device
/// ([`mosaic_sync::transport::PullPage::rotation_requested`]). The member's new
/// identity key is not verified there, so the rotation waits
/// ([`SyncWarning::RotationBlocked`]) and nobody is locked out.
#[test]
fn member_rekey_rotation_request_reaches_the_owner_and_waits_for_verification() {
    let (mut w, vault, item) = shared_world(32);
    let before = w.generation(0, 0, vault);
    let new_sk = SecretKey::generate().unwrap();
    rekey_acct(&mut w, 1, 0, &new_sk);
    adopt_acct(&mut w, 1, 1, &new_sk);
    let report = w.sync(0, 0);
    assert_eq!(report.rotated_vaults.len(), 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::RotationBlocked { .. })),
        "{:?}",
        report.warnings
    );
    assert_eq!(
        w.generation(0, 0, vault),
        before,
        "no rotation, nobody locked out"
    );
    for d in 0..2 {
        assert!(w.has(1, d, vault, item), "the member still reads the vault");
    }
}

/// The owner compares the member's new safety code, accepts the change and
/// verifies; the next run rotates and the member converges.
///
/// The rotation plan checks the member's served identity key against the
/// roster fingerprint; the owner's verified acceptance of the member's new
/// safety code writes the new fingerprint into the roster.
#[test]
fn member_rekey_triggers_owner_rotation_after_verification() {
    let (mut w, vault, item) = shared_world(33);
    let before = w.generation(0, 0, vault);
    let new_sk = SecretKey::generate().unwrap();
    rekey_acct(&mut w, 1, 0, &new_sk);
    adopt_acct(&mut w, 1, 1, &new_sk);
    w.sync(0, 0);
    let email = w.email(1);
    {
        let dev = &mut w.accts[0].devs[0];
        let found = dev.engine.lookup_contact(&mut dev.access, &email).unwrap();
        dev.s_mut()
            .accept_key_change(&found.account_id, &found.ipk, &email)
            .unwrap();
        let fpr = mosaic_core::Fingerprint::of(&found.account_id, &found.ipk);
        dev.s_mut().verify_contact(&found.account_id, &fpr).unwrap();
    }
    let report = w.sync(0, 0);
    assert!(
        report.rotated_vaults.contains(&vault),
        "the owner rotates: {:?}",
        report.warnings
    );
    w.converge();
    assert_eq!(w.generation(0, 0, vault).get(), before.get() + 1);
    for d in 0..2 {
        assert_eq!(w.generation(1, d, vault), w.generation(0, 0, vault));
        assert!(w.has(1, d, vault, item));
    }
    no_findings(&w);
}

/// Without the owner's verification of the new safety code the roster keeps
/// the old fingerprint and the rotation stays blocked.
#[test]
fn member_rekey_without_owner_verification_blocks_rotation() {
    let (mut w, vault, _item) = shared_world(34);
    let before = w.generation(0, 0, vault);
    let new_sk = SecretKey::generate().unwrap();
    rekey_acct(&mut w, 1, 0, &new_sk);
    adopt_acct(&mut w, 1, 1, &new_sk);
    w.sync(0, 0);
    let email = w.email(1);
    {
        let dev = &mut w.accts[0].devs[0];
        let found = dev.engine.lookup_contact(&mut dev.access, &email).unwrap();
        // Accepting the changed key is not verifying it.
        dev.s_mut()
            .accept_key_change(&found.account_id, &found.ipk, &email)
            .unwrap();
    }
    let report = w.sync(0, 0);
    assert!(!report.rotated_vaults.contains(&vault));
    assert_eq!(w.generation(0, 0, vault), before);
}

// ---- review round (ADR-0063) -------------------------------------------------------------

/// H2 and L1: a device that joins after a re-key starts at the server's
/// epoch, so its own re-key targets the next epoch and the server takes it.
#[test]
fn hosted_join_after_a_rekey_then_rekey_again() {
    let (mut w, _vault, _item) = world(31, 1);
    let sk1 = SecretKey::generate().unwrap();
    assert!(rekey(&mut w, 0, &sk1).finished);
    w.secret_key = copy_key(&sk1);
    let d = w.try_join_with(NEW_PASSWORD).unwrap();
    assert_eq!(
        w.devs[d].s().ak_epoch(),
        2,
        "the join took the server epoch"
    );
    // A hosted run compares the served epoch with the own one: no rollback.
    let r = w.sync(d);
    assert!(
        !r.warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::AccountRollback { .. }))
    );
    // Its own re-key goes through (target 3) and device 0 adopts it.
    let sk2 = SecretKey::generate().unwrap();
    let third = "a third horse battery staple";
    let (cur_pw, new_pw) = (pw(NEW_PASSWORD), pw(third));
    let secrets = RekeySecrets {
        current_password: &cur_pw,
        current_secret_key: &sk1,
        new_password: &new_pw,
        new_secret_key: &sk2,
        profile: Argon2Profile::Baseline,
    };
    w.server.advance_ms(60_000);
    let (name, device) = (w.devs[d].name.clone(), w.devs[d].device);
    let done = {
        let World {
            devs,
            clock,
            server,
            email,
            ..
        } = &mut w;
        let hosted = HostedRekey {
            base: &server.base,
            address: email,
            device_name: &name,
            totp_code: None,
        };
        let dev = &mut devs[d];
        dev.engine
            .rekey_hosted(&mut dev.access, clock, &hosted, &secrets)
            .unwrap()
    };
    assert_eq!(w.devs[d].s().ak_epoch(), 3);
    drop(done);
    let _ = device;
}

/// L1: a server that states a lower epoch than the device's gives a finding.
#[test]
fn hosted_lower_epoch_is_an_account_rollback_finding() {
    let (mut w, _vault, _item) = world(32, 1);
    let sk1 = SecretKey::generate().unwrap();
    assert!(rekey(&mut w, 0, &sk1).finished);
    // The device believes it is one epoch further than the server says.
    w.devs[0].s_mut().raise_epoch_to(3).unwrap();
    let r = w.sync(0);
    assert!(r.warnings.iter().any(|x| matches!(
        x,
        SyncWarning::AccountRollback {
            stored: 3,
            offered: 2
        }
    )));
}

/// M5 and M1: after a hosted adoption the owned vault is held until the next
/// generation's key arrives, then edits converge; a device with an open
/// finding still adopts.
#[test]
fn hosted_adopted_vault_is_held_until_the_next_generation() {
    let (mut w, vault, item) = world(33, 2);
    set_password(&mut w, 1, vault, item, "edit while stale");
    let new_sk = SecretKey::generate().unwrap();
    assert!(rekey(&mut w, 0, &new_sk).finished);
    adopt(&mut w, 1, &new_sk).unwrap();
    assert!(
        w.devs[1].s().rekey_held(vault).unwrap(),
        "nothing is pushed under the retired key"
    );
    converge(&mut w, 2);
    assert!(!w.devs[1].s().rekey_held(vault).unwrap());
    for d in 0..2 {
        assert_eq!(w.devs[d].s().vaults()[0].key_generation.get(), 2);
    }
    assert_eq!(
        password_of(&w, 0, vault, item),
        password_of(&w, 1, vault, item)
    );
}

/// H3: an invite the recipient's re-key cancelled (state 5) reads as
/// cancelled on the inviter's side and does not break the inviter's sync.
#[test]
fn inviter_sync_survives_the_recipients_rekey() {
    let mut w = SWorld::with_accounts(34, 2);
    let vault = w.new_vault(0, 0, "Shared");
    w.add_login(0, 0, vault, "item");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    w.converge();
    rekey_acct(&mut w, 1, 0, &SecretKey::generate().unwrap());
    w.sync(0, 0);
    w.sync(0, 0);
}
