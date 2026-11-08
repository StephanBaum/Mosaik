//! A hostile server against sharing (M6b spec §8.5, CRYPTO_SPEC §8.2 to
//! §8.4, R71 to R79). The real server's database or its answers are changed
//! on their way to one device: the directory serves another identity key, an
//! invite goes to another recipient, a revoked member and the server forge a
//! rotation, the server adds a member, releases an emergency grant early,
//! rolls a vault back or withholds the contact book. A device must refuse
//! what does not authenticate, never seal a key to a key that does not match
//! the pin or the roster, and keep what it has.
//!
//! The server is hostile at two levels: SQL on its database (what a server
//! operator can do) and a `MaliciousTransport` that rewrites what one
//! device pulls.

use std::sync::{Arc, Mutex};

use mosaic_core::model::Timestamp;
use mosaic_core::model::{ContactState, Role, VaultRoster};
use mosaic_core::{
    AccountId, DeviceId, Fingerprint, IdentityPublicKey, IdentitySecret, KeyGeneration, VaultId,
    VaultKey, seal_emergency_vault_key, seal_vault_key,
};
use mosaic_sync::engine::{SyncEngine, SyncError, SyncReport, SyncWarning};
use mosaic_sync::http::HttpTransport;
use mosaic_sync::sharing::{GrantScope, NewInvite};
use mosaic_sync::testutil::MaliciousTransport;
use mosaic_sync::transport::{PulledRoster, PulledShare, Transport};
use mosaic_vault::session::{Error, SharingFinding};

use crate::scenarios::engine_for;
use crate::sharing_world::SWorld;
use crate::sim_sharing::{assert_agree, assert_no_findings, trio_with};

type Hostile = SyncEngine<MaliciousTransport<HttpTransport>>;

/// A device of account `a` reading through a transport the test controls.
/// A login ends the other sessions of the same device: the device's own
/// engine needs `SWorld::fresh` before it is used again.
fn hostile(w: &mut SWorld, a: usize, d: usize) -> Hostile {
    let acct = &w.accts[a];
    let opw = acct.devs[d]
        .s()
        .opaque_password(
            &crate::hosted::pw(crate::hosted::PASSWORD),
            &acct.secret_key,
        )
        .unwrap();
    let (email, id, device) = (acct.email.clone(), acct.id, acct.devs[d].device);
    let transport = w.login_transport(&email, id, device, "hostile path", &opw);
    engine_for(MaliciousTransport::new(transport))
}

fn run(w: &mut SWorld, a: usize, d: usize, engine: &mut Hostile) -> SyncReport {
    w.server.advance_ms(60_000);
    let clock = w.clock.clone();
    let dev = &mut w.accts[a].devs[d];
    engine.sync_once(&mut dev.access, &clock).unwrap()
}

/// A key nobody holds the private half of... except the attacker.
fn attacker() -> (IdentitySecret, IdentityPublicKey) {
    IdentitySecret::generate().unwrap()
}

/// The server serves `ipk` for the account from now on.
fn substitute(w: &SWorld, account: usize, ipk: &IdentityPublicKey) {
    let id = w.id(account);
    w.server
        .state
        .db()
        .run(|c| {
            c.execute(
                "UPDATE accounts SET ipk = ?1 WHERE account_id = ?2",
                [ipk.0.as_slice(), id.as_bytes().as_slice()],
            )?;
            Ok(())
        })
        .unwrap();
}

fn count(w: &SWorld, sql: &'static str) -> i64 {
    w.server
        .state
        .db()
        .run(|c| Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))?))
        .unwrap()
}

fn own_ipk(w: &SWorld, a: usize) -> IdentityPublicKey {
    w.accts[a].devs[0].s().identity().unwrap().unwrap().0
}

/// The identity secret of an account, opened the way its devices do.
fn isk_of(w: &SWorld, a: usize) -> IdentitySecret {
    let acct = &w.accts[a];
    let boot = acct.devs[0].s().account_bootstrap().unwrap();
    let ak = mosaic_core::unlock_with_password(
        &crate::hosted::pw(crate::hosted::PASSWORD),
        &acct.secret_key,
        &boot.params,
        &boot.wrapped_ak,
    )
    .unwrap();
    let wrapped = acct.devs[0].s().identity().unwrap().unwrap().1;
    wrapped.unwrap_identity(&ak, &acct.id).unwrap()
}

/// A share of a new key from `forger` to `target`, valid in every way except
/// that the forger is not the owner, with the roster that goes with it.
fn forge_share(
    w: &SWorld,
    forger: usize,
    target: usize,
    owner: usize,
    vault: VaultId,
    generation: KeyGeneration,
) -> (PulledShare, PulledRoster) {
    let vk = VaultKey::generate(generation).unwrap();
    let (forger_id, target_id, owner_id) = (w.id(forger), w.id(target), w.id(owner));
    let seal = seal_vault_key(
        &vk,
        &vault,
        &forger_id,
        &isk_of(w, forger),
        &target_id,
        &own_ipk(w, target),
    )
    .unwrap();
    let mut roster = VaultRoster::new(generation, owner_id);
    {
        let mut edit = roster.edit(DeviceId::new_v7().unwrap(), Timestamp::from_unix_millis(1));
        for (account, role) in [
            (owner, Role::Owner),
            (target, Role::Write),
            (forger, Role::Manage),
        ] {
            edit.set_member(
                &w.id(account),
                role,
                KeyGeneration::FIRST,
                &Fingerprint::of(&w.id(account), &own_ipk(w, account)),
                "x@example.com",
            )
            .unwrap();
        }
    }
    let ciphertext = roster.seal(&vk, &vault).unwrap();
    (
        PulledShare {
            generation,
            sender: forger_id,
            seal,
            seq: Some(1),
        },
        PulledRoster {
            generation,
            ciphertext,
            seq: Some(1),
        },
    )
}

fn untrusted_findings(w: &SWorld, a: usize, d: usize) -> Vec<SharingFinding> {
    w.accts[a].devs[d]
        .s()
        .sharing_findings()
        .unwrap()
        .into_iter()
        .filter(|f| matches!(f, SharingFinding::ShareUntrusted { .. }))
        .collect()
}

#[test]
fn key_substitution_first_contact_shows_different_code() {
    let mut w = SWorld::with_accounts(30, 2);
    w.sync_all();
    let real = w
        .dev(1, 0)
        .s()
        .own_fingerprint()
        .unwrap()
        .unwrap()
        .safety_code();
    substitute(&w, 1, &attacker().1);
    let email = w.email(1);
    let dev = w.dev(0, 0);
    let found = dev.engine.lookup_contact(&mut dev.access, &email).unwrap();
    // The first contact is pinned unverified, and the code the user would
    // compare with the real person is not the real person's.
    assert_eq!(found.state, ContactState::Pinned { verified: false });
    assert_ne!(found.safety_code, real);
}

#[test]
fn key_substitution_after_pin_blocks_seal() {
    let mut w = SWorld::with_accounts(31, 2);
    let vault = w.new_vault(0, 0, "Shared");
    w.sync_all();
    w.verify(0, 0, 1);
    substitute(&w, 1, &attacker().1);
    let (id, email) = (w.id(1), w.email(1));
    let dev = w.dev(0, 0);
    let refused = dev
        .engine
        .invite_member(&mut dev.access, vault, &id, &email, Role::Write);
    assert!(
        matches!(refused, Err(SyncError::Session(Error::ContactKeyChanged))),
        "{refused:?}"
    );
    // No invite, no seal.
    assert_eq!(count(&w, "SELECT COUNT(*) FROM share_invites"), 0);
}

#[test]
fn roster_fpr_mismatch_blocks_rotation_seal() {
    let (mut w, vault, _items) = trio_with(32, Role::Manage, Role::Write);
    let real = own_ipk(&w, 1);
    // The server serves another key for B to its co-members.
    substitute(&w, 1, &attacker().1);
    w.remove(0, 0, vault, 2).unwrap();
    let report = w.sync(0, 0);
    assert!(
        report
            .warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::RotationBlocked { vault: v } if *v == vault)),
        "{:?}",
        report.warnings
    );
    // No rotation, no share sealed to the attacker's key.
    assert_eq!(w.generation(0, 0, vault).get(), 1);
    assert_eq!(count(&w, "SELECT COUNT(*) FROM key_shares"), 0);
    // With the honest key back, the revocation goes through.
    substitute(&w, 1, &real);
    w.converge();
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    assert_eq!(w.generation(1, 0, vault).get(), 2);
}

#[test]
fn replayed_invite_to_other_recipient_fails_open() {
    let mut w = SWorld::with_accounts(33, 3);
    let vault = w.new_vault(0, 0, "Shared");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    // The server hands B's invite to C.
    let (b, c) = (w.id(1), w.id(2));
    w.server
        .state
        .db()
        .run(|conn| {
            conn.execute(
                "UPDATE share_invites SET recipient = ?1 WHERE recipient = ?2",
                [c.as_bytes().as_slice(), b.as_bytes().as_slice()],
            )?;
            Ok(())
        })
        .unwrap();
    let invite = w.incoming(2, 0, vault);
    let dev = w.dev(2, 0);
    let opened = dev.engine.preview_invite(&mut dev.access, &invite);
    assert!(
        matches!(opened, Err(SyncError::Session(Error::InviteUnreadable))),
        "{opened:?}"
    );
    assert!(!w.has_vault(2, 0, vault));
}

#[test]
fn replayed_invite_not_in_roster_refused() {
    let mut w = SWorld::with_accounts(34, 3);
    let vault = w.new_vault(0, 0, "Shared");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Manage).unwrap();
    w.accept(1, 0, vault);
    w.converge();
    // B, a manager, sends an invite to C without writing C into the roster.
    w.verify(1, 0, 2);
    let (c, c_email) = (w.id(2), w.email(2));
    let dev = w.dev(1, 0);
    let found = dev
        .engine
        .lookup_contact(&mut dev.access, &c_email)
        .unwrap();
    let prepared = dev
        .s()
        .prepare_invite(vault, &c, &found.ipk, &c_email, Role::Write)
        .unwrap();
    dev.engine
        .transport_mut()
        .sharing()
        .unwrap()
        .create_invite(
            vault,
            &NewInvite {
                invite_id: prepared.invite_id(),
                recipient: c,
                role: Role::Write,
                generation: prepared.generation(),
                seal: prepared.seal(),
            },
        )
        .unwrap();
    let invite = w.incoming(2, 0, vault);
    let dev = w.dev(2, 0);
    let opened = dev.engine.preview_invite(&mut dev.access, &invite);
    assert!(
        matches!(opened, Err(SyncError::Session(Error::InviteNotInRoster))),
        "{opened:?}"
    );
}

#[test]
fn replayed_old_invite_after_revocation_gains_nothing() {
    let mut w = SWorld::with_accounts(35, 3);
    let vault = w.new_vault(0, 0, "Shared");
    let item = w.add_login(0, 0, vault, "item");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    w.invite(0, 0, vault, 2, Role::Write).unwrap();
    w.accept(2, 0, vault);
    // B fetches its invite (and keeps it), then declines: the vault moves to
    // generation 2.
    let kept = {
        let dev = w.dev(1, 0);
        let (incoming, _) = dev.engine.list_invites(&mut dev.access).unwrap();
        let invite = incoming[0].invite.clone();
        dev.engine
            .decline_invite(&mut dev.access, &invite.invite_id, false)
            .unwrap();
        invite
    };
    w.converge();
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    // The server serves B the old invite again.
    let id = *kept.invite_id.as_bytes();
    let (vault_bytes, sender, recipient) = (
        *kept.vault_id.as_bytes(),
        *kept.sender.as_bytes(),
        *w.id(1).as_bytes(),
    );
    let seal = kept.seal.as_bytes().to_vec();
    w.server
        .state
        .db()
        .run(|c| {
            c.execute(
                "INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, \
                 generation, seal, created_at, expires_at, delivered_at, state, hidden) \
                 VALUES (?1, ?2, ?3, ?4, 3, 1, ?5, 0, 9999999999999, 0, 0, 0)",
                rusqlite::params![
                    id.as_slice(),
                    vault_bytes.as_slice(),
                    sender.as_slice(),
                    recipient.as_slice(),
                    seal
                ],
            )?;
            Ok(())
        })
        .unwrap();
    let dev = w.dev(1, 0);
    let replayed = dev.engine.preview_invite(&mut dev.access, &kept.invite_id);
    // The seal still opens to the generation-1 key, but the vault's metadata
    // and roster are sealed under generation 2: nothing opens, and nothing
    // can be accepted.
    assert!(
        matches!(replayed, Err(SyncError::Session(Error::InviteUnreadable))),
        "{replayed:?}"
    );
    let accepted = dev.engine.accept_invite(&mut dev.access, &kept.invite_id);
    assert!(matches!(
        accepted,
        Err(SyncError::Session(Error::InviteNotOpened))
    ));
    assert!(!w.has_vault(1, 0, vault));
    assert!(w.has(0, 0, vault, item));
}

#[test]
fn replayed_lower_generation_share_rejected() {
    let (mut w, vault, items) = trio_with(36, Role::Manage, Role::Write);
    w.rotate(0, 0, vault).unwrap();
    // C reads the page that carries its generation-2 share, and claims it.
    let captured: Arc<Mutex<Vec<PulledShare>>> = Arc::default();
    let mut engine = hostile(&mut w, 2, 0);
    {
        let sink = Arc::clone(&captured);
        engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
            sink.lock().unwrap().extend(page.shares.iter().cloned());
        }));
    }
    run(&mut w, 2, 0, &mut engine);
    assert_eq!(w.generation(2, 0, vault).get(), 2);
    let old = captured.lock().unwrap().clone();
    assert!(!old.is_empty(), "the share of generation 2 was served");
    // The server serves the old share again; C holds generation 2 already.
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        page.shares.extend(old.iter().cloned());
    }));
    run(&mut w, 2, 0, &mut engine);
    assert_eq!(w.generation(2, 0, vault).get(), 2);
    assert_eq!(untrusted_findings(&w, 2, 0).len(), 0);
    w.fresh(2, 0);
    w.converge();
    assert_agree(&w, vault, &[0, 1, 2], &items);
}

#[test]
fn revocation_rollback_rejected() {
    let (mut w, vault, items) = trio_with(37, Role::Manage, Role::Write);
    // A second device of B learns the vault at generation 1; what the server
    // serves it is kept.
    let b2 = w.join(1);
    let kept: Arc<Mutex<(Vec<mosaic_sync::transport::PulledRecord>, Vec<PulledRoster>)>> =
        Arc::default();
    let mut engine = hostile(&mut w, 1, b2);
    {
        let sink = Arc::clone(&kept);
        engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
            let mut k = sink.lock().unwrap();
            k.0.extend(page.records.iter().cloned());
            k.1.extend(page.rosters.iter().cloned());
        }));
    }
    run(&mut w, 1, b2, &mut engine);
    assert_eq!(w.generation(1, b2, vault).get(), 1);
    // C is removed: the vault is at generation 2.
    w.remove(0, 0, vault, 2).unwrap();
    w.sync(0, 0);
    // The server serves B's second device the generation-1 records and
    // roster again, newer sequence numbers on top of the real page.
    let (records, rosters) = kept.lock().unwrap().clone();
    assert!(!records.is_empty() && !rosters.is_empty());
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        let top = u64::from_be_bytes(<[u8; 8]>::try_from(page.next.0.as_slice()).unwrap());
        for (i, r) in records.iter().enumerate() {
            let mut r = r.clone();
            r.seq = Some(top + 1 + i as u64);
            page.records.push(r);
        }
        page.rosters.extend(rosters.iter().cloned());
    }));
    run(&mut w, 1, b2, &mut engine);
    assert_eq!(w.generation(1, b2, vault).get(), 2);
    w.fresh(1, b2);
    w.converge();
    assert_agree(&w, vault, &[0, 1], &items);
    assert_no_findings(&w, &[0]);
}

#[test]
fn revoked_member_and_server_forge_next_rotation_rejected() {
    let (mut w, vault, _items) = trio_with(38, Role::Manage, Role::Write);
    w.remove(0, 0, vault, 2).unwrap();
    w.converge();
    assert_eq!(w.generation(1, 0, vault).get(), 2);
    // C, removed, holds the generation-1 key and its own identity key. With
    // the server it forges the share for generation 3 that B should take.
    let (share, roster) = forge_share(&w, 2, 1, 0, vault, KeyGeneration::new(3).unwrap());
    let mut engine = hostile(&mut w, 1, 0);
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        page.shares.push(share.clone());
        page.rosters.push(roster.clone());
    }));
    run(&mut w, 1, 0, &mut engine);
    assert_eq!(
        w.generation(1, 0, vault).get(),
        2,
        "the forged key was not taken"
    );
    let c = w.id(2);
    // The share is named as untrusted, whether the sender fails the owner
    // check or the directory no longer knows C (it shares nothing with B).
    let findings = untrusted_findings(&w, 1, 0);
    assert!(
        findings.iter().any(|f| matches!(
            f,
            SharingFinding::ShareUntrusted { sender, vault: v, .. } if *sender == c && *v == vault
        )),
        "{findings:?}"
    );
    assert_eq!(w.generation(0, 0, vault).get(), 2);
}

#[test]
fn manager_share_rejected_only_owner_rotates() {
    let (mut w, vault, _items) = trio_with(39, Role::Manage, Role::Write);
    // B holds the current key and a valid identity key, and is a manager:
    // its share for generation 2 to C is still not a rotation (only the
    // owner rotates, CRYPTO_SPEC §8.3).
    let (share, roster) = forge_share(&w, 1, 2, 0, vault, KeyGeneration::new(2).unwrap());
    let mut engine = hostile(&mut w, 2, 0);
    engine.transport_mut().tamper_pull = Some(Box::new(move |page| {
        page.shares.push(share.clone());
        page.rosters.push(roster.clone());
    }));
    run(&mut w, 2, 0, &mut engine);
    assert_eq!(w.generation(2, 0, vault).get(), 1);
    let b = w.id(1);
    let findings = untrusted_findings(&w, 2, 0);
    assert!(
        findings.iter().any(|f| matches!(
            f,
            SharingFinding::ShareUntrusted { sender, vault: v, .. } if *sender == b && *v == vault
        )),
        "{findings:?}"
    );
}

#[test]
fn server_injected_member_shown_unconfirmed_and_revoked() {
    let mut w = SWorld::with_accounts(40, 3);
    let vault = w.new_vault(0, 0, "Shared");
    let item = w.add_login(0, 0, vault, "item");
    w.sync_all();
    w.invite(0, 0, vault, 1, Role::Write).unwrap();
    w.accept(1, 0, vault);
    w.converge();
    // The server adds C to the vault's members.
    let (c, v) = (w.id(2), vault);
    w.server
        .state
        .db()
        .run(|conn| {
            conn.execute(
                "INSERT INTO memberships (account_id, vault_id, role, origin, added_at) \
                 VALUES (?1, ?2, 3, 1, 0)",
                [c.as_bytes().as_slice(), v.as_bytes().as_slice()],
            )?;
            Ok(())
        })
        .unwrap();
    let members = {
        let dev = w.dev(0, 0);
        dev.engine.vault_members(&mut dev.access, vault).unwrap()
    };
    let row = members
        .members
        .iter()
        .find(|m| m.account_id == c)
        .expect("C is listed");
    assert_eq!(row.state, mosaic_sync::engine::MemberState::Unconfirmed);
    // The owner revokes it; the rotation locks it out.
    w.remove(0, 0, vault, 2).unwrap();
    w.converge();
    assert_eq!(w.generation(0, 0, vault).get(), 2);
    assert_eq!(w.generation(1, 0, vault).get(), 2);
    assert_eq!(
        count(
            &w,
            "SELECT COUNT(*) FROM memberships WHERE role = 3 AND origin = 1 AND added_at = 0"
        ),
        0
    );
    assert!(w.has(1, 0, vault, item));
}

/// A grants B emergency access (one day), B requested; the wait has not
/// passed on the real clock.
fn granted(seed: u64) -> (SWorld, VaultId, mosaic_core::ItemId) {
    let mut w = SWorld::with_accounts(seed, 2);
    let vault = w.new_vault(0, 0, "Estate");
    let item = w.add_login(0, 0, vault, "will");
    w.sync_all();
    w.verify(0, 0, 1);
    let (a, b, b_email) = (w.id(0), w.id(1), w.email(1));
    w.fresh(0, 0);
    {
        let dev = w.dev(0, 0);
        dev.engine
            .grant_emergency(&mut dev.access, &b, &b_email, GrantScope::All, 7, &[])
            .unwrap();
    }
    let dev = w.dev(1, 0);
    dev.engine.request_emergency(&mut dev.access, &a).unwrap();
    (w, vault, item)
}

#[test]
fn emergency_early_release_documented() {
    let (mut w, vault, item) = granted(41);
    let a = w.id(0);
    // Honest: the wait of seven days is not over.
    {
        let dev = w.dev(1, 0);
        let early = dev.engine.claim_emergency(&mut dev.access, &a);
        assert!(early.is_err());
    }
    // A server that controls its clock (or its database) releases at once.
    // Residual R73: the client cannot tell. The release still needs the
    // grantor's real seal, and that is what B opens.
    w.server
        .state
        .db()
        .run(|c| {
            c.execute(
                "UPDATE emergency_grants SET requested_at = requested_at - 8 * 86400000",
                [],
            )?;
            Ok(())
        })
        .unwrap();
    let claimed = {
        let dev = w.dev(1, 0);
        dev.engine.claim_emergency(&mut dev.access, &a).unwrap()
    };
    assert_eq!(claimed, 1);
    w.converge();
    assert!(w.has(1, 0, vault, item));
    assert_eq!(w.dev(1, 0).s().vault_own_role(vault).unwrap(), Role::Read);
}

#[test]
fn emergency_forged_seal_from_server_key_fails() {
    let (mut w, vault, _item) = granted(42);
    let (a, b) = (w.id(0), w.id(1));
    let b_ipk = own_ipk(&w, 1);
    // The wait is over (the server says so). The seal it releases is one the
    // server made itself with a key of its own.
    let (server_isk, _) = attacker();
    let forged = seal_emergency_vault_key(
        &VaultKey::generate(KeyGeneration::FIRST).unwrap(),
        &vault,
        &a,
        &server_isk,
        &b,
        &b_ipk,
    )
    .unwrap();
    w.server
        .state
        .db()
        .run(|c| {
            c.execute(
                "UPDATE emergency_grants SET requested_at = requested_at - 8 * 86400000",
                [],
            )?;
            c.execute(
                "UPDATE emergency_seals SET seal = ?1",
                [forged.as_bytes().as_slice()],
            )?;
            Ok(())
        })
        .unwrap();
    let dev = w.dev(1, 0);
    let refused = dev.engine.claim_emergency(&mut dev.access, &a);
    assert!(
        matches!(
            refused,
            Err(SyncError::Session(Error::ShareUntrusted { .. }))
        ),
        "{refused:?}"
    );
    assert!(!w.has_vault(1, 0, vault));
    assert!(
        w.dev(1, 0)
            .s()
            .sharing_findings()
            .unwrap()
            .iter()
            .any(|f| matches!(f, SharingFinding::ShareUntrusted { vault: v, .. } if *v == vault))
    );
}

#[test]
fn identity_mismatch_served_to_self_raises_finding() {
    let mut w = SWorld::with_accounts(43, 2);
    w.sync_all();
    assert_eq!(w.dev(0, 0).s().sharing_findings().unwrap().len(), 0);
    // The server serves this account another identity key than the one it
    // published.
    substitute(&w, 0, &attacker().1);
    w.sync(0, 0);
    assert!(
        w.dev(0, 0)
            .s()
            .sharing_findings()
            .unwrap()
            .contains(&SharingFinding::IdentityMismatch)
    );
}

#[test]
fn contacts_book_withheld_new_device_tofu() {
    let (mut w, vault, _items) = trio_with(44, Role::Manage, Role::Write);
    // A new device of A joins; the server withholds the contact book (the
    // pins of B and C) and serves another key for B.
    let d = w.join(0);
    w.server
        .state
        .db()
        .run(|c| {
            c.execute("UPDATE accounts SET contacts_ct = NULL", [])?;
            Ok(())
        })
        .unwrap();
    substitute(&w, 1, &attacker().1);
    w.sync(0, d);
    assert!(
        w.dev(0, d).s().contacts().unwrap().is_empty(),
        "the new device has no pins"
    );
    // The roster, which the inviting manager wrote under the vault key,
    // still holds B's fingerprint: the new device refuses to seal to the
    // substituted key.
    w.remove(0, d, vault, 2).unwrap();
    let report = w.sync(0, d);
    assert!(
        report
            .warnings
            .iter()
            .any(|x| matches!(x, SyncWarning::RotationBlocked { vault: v } if *v == vault)),
        "{:?}",
        report.warnings
    );
    assert_eq!(w.generation(0, d, vault).get(), 1);
    assert_eq!(count(&w, "SELECT COUNT(*) FROM key_shares"), 0);
    let _ = AccountId::from_bytes([1; 16]);
}
