//! Tests of the sharing session API (M6b): identity keys, contacts, invites,
//! the roster, shared rotation with attachment re-seal, the claim rule and
//! emergency seals.

#![expect(
    clippy::many_single_char_names,
    clippy::similar_names,
    reason = "accounts a, b, c (and their sessions) read best as one letter"
)]

use mosaic_core::model::{ContactState, ItemType, Role, VaultRoster};
use mosaic_core::{
    AccountId, Argon2Profile, Fingerprint, IdentityPublicKey, IdentitySecret, KeyGeneration,
    MasterPassword, SecretKey, VaultId, VaultKey, VaultRosterCiphertext, seal_vault_key,
};

use super::{
    AcceptedInvite, Error, IncomingInvite, InviteId, KeyOrigin, LockedSession, OpenedChain,
    PulledShareView, RevocationReason, RotationPlan, SharingFinding, SyncConfig, SyncKind,
    UnlockedSession,
};
use crate::storage::{Error as StorageError, Store};

fn pw() -> MasterPassword {
    MasterPassword::new("unit test password".to_owned())
}

fn g(n: u32) -> KeyGeneration {
    KeyGeneration::new(n).unwrap()
}

/// One account with an unlocked session and an identity key.
struct User {
    s: UnlockedSession,
    id: AccountId,
    ipk: IdentityPublicKey,
    secret: SecretKey,
}

fn user_in(store: Store, email: &str) -> User {
    let (mut s, created) = LockedSession::open(store)
        .create_account(&pw(), Argon2Profile::Baseline)
        .unwrap();
    s.set_sync_config(Some(&SyncConfig {
        kind: SyncKind::Hosted,
        server_url: Some("https://one.example".to_owned()),
        email: Some(email.to_owned()),
        folder_path: None,
        device_name: "Laptop".to_owned(),
    }))
    .unwrap();
    let ipk = s.ensure_identity_key().unwrap();
    User {
        id: s.account_id(),
        s,
        ipk,
        secret: created.secret_key,
    }
}

fn user(email: &str) -> User {
    user_in(Store::open_in_memory().unwrap(), email)
}

fn fpr(u: &User) -> Fingerprint {
    Fingerprint::of(&u.id, &u.ipk)
}

/// `a` pins `b` and attests the safety codes match.
fn verify(a: &mut User, b: &User, email: &str) {
    a.s.pin_contact(&b.id, &b.ipk, email).unwrap();
    a.s.verify_contact(&b.id, &fpr(b)).unwrap();
}

/// What the invitee's device fetched for an invite.
struct Sent {
    incoming: IncomingInvite,
    meta: mosaic_core::VaultMetaCiphertext,
    roster: VaultRosterCiphertext,
}

/// `owner` invites `other` to `vault` with `role`.
fn invite(owner: &mut User, vault: VaultId, other: &User, role: Role) -> Sent {
    verify(owner, other, "b@x");
    let p = owner
        .s
        .prepare_invite(vault, &other.id, &other.ipk, "b@x", role)
        .unwrap();
    let incoming = IncomingInvite {
        invite_id: p.invite_id(),
        vault_id: vault,
        sender: owner.id,
        sender_email: "a@x".to_owned(),
        role,
        generation: p.generation(),
        seal: p.seal().clone(),
    };
    let roster = p.roster().clone();
    owner.s.commit_invite_sent(p, 1).unwrap();
    let meta = owner.s.store.vault_meta(&vault).unwrap().unwrap();
    Sent {
        incoming,
        meta,
        roster,
    }
}

/// `other` previews, verifies the sender and accepts.
fn accept(other: &mut User, owner: &User, sent: &Sent, role: Role) {
    let preview = other
        .s
        .open_invite(&sent.incoming, &owner.ipk, &sent.meta, &sent.roster)
        .unwrap();
    assert_eq!(preview.role, role);
    assert_eq!(preview.sender_code, fpr(owner).safety_code());
    other.s.verify_contact(&owner.id, &fpr(owner)).unwrap();
    let a = other.s.accept_invite(&sent.incoming.invite_id).unwrap();
    other.s.commit_invite(a, role, &owner.id).unwrap();
}

/// The roster the owner's device holds now, as the server would serve it.
fn current_roster(owner: &User, vault: VaultId) -> VaultRosterCiphertext {
    owner
        .s
        .store
        .vault_roster(&vault)
        .unwrap()
        .unwrap()
        .ciphertext
}

/// A vault of `a` shared with `b` (write) and `c` (read), all in sync at
/// generation 1.
fn trio() -> (User, User, User, VaultId) {
    let (mut a, mut b, mut c) = (user("a@x"), user("b@x"), user("c@x"));
    let v = a.s.create_vault("Team").unwrap();
    let sent_b = invite(&mut a, v, &b, Role::Write);
    accept(&mut b, &a, &sent_b, Role::Write);
    let sent_c = invite(&mut a, v, &c, Role::Read);
    accept(&mut c, &a, &sent_c, Role::Read);
    let roster = current_roster(&a, v);
    b.s.apply_remote_roster(v, g(1), &roster, 2, &a.id).unwrap();
    c.s.apply_remote_roster(v, g(1), &roster, 2, &a.id).unwrap();
    (a, b, c, v)
}

#[test]
fn identity_generated_once_and_survives_lock() {
    let mut u = user("a@x");
    let ipk = u.ipk;
    assert_eq!(u.s.ensure_identity_key().unwrap(), ipk);
    let (stored_ipk, wrapped, published) = u.s.identity().unwrap().unwrap();
    assert_eq!(stored_ipk, ipk);
    assert!(!published);
    assert_eq!(u.s.own_fingerprint().unwrap(), Some(fpr(&u)));
    // The wrapping opens to the key that matches the public key.
    let isk = wrapped.unwrap_identity(&u.s.account_key, &u.id).unwrap();
    assert_eq!(isk.public_key().unwrap(), ipk);
    u.s.mark_identity_published().unwrap();

    let mut s = u.s.lock().unlock(&pw(), &u.secret).unwrap();
    assert_eq!(s.ensure_identity_key().unwrap(), ipk);
    assert!(s.identity().unwrap().unwrap().2);
}

#[test]
fn adopt_identity_only_if_unpublished() {
    let mut u = user("a@x");
    let (isk2, ipk2) = IdentitySecret::generate().unwrap();
    let wrapped2 = isk2.wrap(&u.s.account_key, &u.id).unwrap();
    // Wrapped for another account: refused, nothing changes.
    let other = AccountId::new_random().unwrap();
    let foreign = isk2.wrap(&u.s.account_key, &other).unwrap();
    assert!(matches!(
        u.s.adopt_remote_identity(&foreign, &ipk2),
        Err(Error::IdentityMismatch)
    ));
    // A public key that is not the wrapped key's: refused.
    assert!(matches!(
        u.s.adopt_remote_identity(&wrapped2, &u.ipk),
        Err(Error::IdentityMismatch)
    ));
    assert_eq!(u.s.ensure_identity_key().unwrap(), u.ipk);
    // Unpublished: the server's key replaces it.
    u.s.adopt_remote_identity(&wrapped2, &ipk2).unwrap();
    assert_eq!(u.s.ensure_identity_key().unwrap(), ipk2);
    let (ipk, _, published) = u.s.identity().unwrap().unwrap();
    assert_eq!((ipk, published), (ipk2, true));
    // The same key again is fine; another one is not.
    u.s.adopt_remote_identity(&wrapped2, &ipk2).unwrap();
    let (isk3, ipk3) = IdentitySecret::generate().unwrap();
    let wrapped3 = isk3.wrap(&u.s.account_key, &u.id).unwrap();
    assert!(matches!(
        u.s.adopt_remote_identity(&wrapped3, &ipk3),
        Err(Error::IdentityMismatch)
    ));
    assert_eq!(u.s.identity().unwrap().unwrap().0, ipk2);
}

#[test]
fn contacts_pin_verify_and_key_change() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    assert_eq!(
        a.s.contact_state(&b.id, &b.ipk).unwrap(),
        ContactState::Unknown
    );
    assert_eq!(
        a.s.pin_contact(&b.id, &b.ipk, "b@x").unwrap(),
        ContactState::Pinned { verified: false }
    );
    a.s.verify_contact(&b.id, &fpr(&b)).unwrap();
    assert_eq!(
        a.s.contact_state(&b.id, &b.ipk).unwrap(),
        ContactState::Pinned { verified: true }
    );
    // The server serves another key: a key change, and the pin is not
    // overwritten by pinning again.
    let (_, other_ipk) = IdentitySecret::generate().unwrap();
    assert_eq!(
        a.s.pin_contact(&b.id, &other_ipk, "b@x").unwrap(),
        ContactState::Changed
    );
    assert_eq!(
        a.s.verify_contact(&b.id, &Fingerprint::of(&b.id, &other_ipk))
            .unwrap_err()
            .to_string(),
        Error::Core(mosaic_core::Error::ContactChanged).to_string()
    );
    // Accepting the change pins the new key, unverified.
    a.s.accept_key_change(&b.id, &other_ipk, "b@x").unwrap();
    assert_eq!(
        a.s.contact_state(&b.id, &other_ipk).unwrap(),
        ContactState::Pinned { verified: false }
    );
    assert_eq!(a.s.contacts().unwrap().len(), 1);
}

#[test]
fn contact_book_syncs_and_merges() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    assert!(a.s.pending_contacts_upload().unwrap().is_none());
    a.s.pin_contact(&b.id, &b.ipk, "b@x").unwrap();
    let (ct, base) = a.s.pending_contacts_upload().unwrap().unwrap();
    assert_eq!(base, None);
    a.s.mark_contacts_uploaded(5).unwrap();
    assert!(a.s.pending_contacts_upload().unwrap().is_none());
    // A pin made between the hand-out and the acknowledgement stays queued.
    let c = user("c@x");
    let (_, _) = (c.id, c.ipk);
    a.s.pin_contact(&c.id, &c.ipk, "c@x").unwrap();
    let _ = a.s.pending_contacts_upload().unwrap().unwrap();
    a.s.pin_contact(&b.id, &b.ipk, "b@x").unwrap(); // no change, nothing queued
    a.s.verify_contact(&b.id, &fpr(&b)).unwrap();
    a.s.mark_contacts_uploaded(6).unwrap();
    assert!(a.s.pending_contacts_upload().unwrap().is_some());
    // Another device of the account sees the first version and merges.
    let mut a2 = LockedSession::open(Store::open_in_memory().unwrap());
    let _ = &mut a2;
    a.s.apply_remote_contacts(&ct, 7).unwrap();
    assert_eq!(a.s.contacts().unwrap().len(), 2);
    assert_eq!(
        a.s.contact_state(&b.id, &b.ipk).unwrap(),
        ContactState::Pinned { verified: true }
    );
}

#[test]
fn invite_requires_verified_contact() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    let ask = |a: &User| a.s.prepare_invite(v, &b.id, &b.ipk, "b@x", Role::Write);
    assert!(matches!(ask(&a), Err(Error::ContactUnverified)));
    a.s.pin_contact(&b.id, &b.ipk, "b@x").unwrap();
    assert!(matches!(ask(&a), Err(Error::ContactUnverified)));
    a.s.verify_contact(&b.id, &fpr(&b)).unwrap();
    assert!(ask(&a).is_ok());
    // A key change blocks.
    let (_, other) = IdentitySecret::generate().unwrap();
    assert!(matches!(
        a.s.prepare_invite(v, &b.id, &other, "b@x", Role::Write),
        Err(Error::ContactKeyChanged)
    ));
    // Not owner-level roles, not to oneself.
    assert!(matches!(
        a.s.prepare_invite(v, &b.id, &b.ipk, "b@x", Role::Owner),
        Err(Error::RoleForbidden)
    ));
    assert!(matches!(
        a.s.prepare_invite(v, &a.id, &a.ipk, "a@x", Role::Read),
        Err(Error::RoleForbidden)
    ));
}

#[test]
fn invite_writes_roster_before_seal() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    verify(&mut a, &b, "b@x");
    let p =
        a.s.prepare_invite(v, &b.id, &b.ipk, "b@x", Role::Write)
            .unwrap();
    // Preparing stores nothing.
    assert!(!a.s.vault_shared(v).unwrap());
    assert!(a.s.vault_roster(v).unwrap().is_none());
    // The roster holds the owner and the invitee at the sealed generation,
    // and opens with the vault key.
    let vk = a.s.vault(v).unwrap().current_key().unwrap();
    let roster = VaultRoster::open(p.roster(), vk, &v, a.id).unwrap();
    roster.validate().unwrap();
    let me = roster.member(&a.id).unwrap();
    assert_eq!((me.role, me.fingerprint), (Role::Owner, fpr(&a)));
    let them = roster.member(&b.id).unwrap();
    assert_eq!(
        (them.role, them.since, them.fingerprint),
        (Role::Write, g(1), fpr(&b))
    );
    assert_eq!(p.generation(), g(1));
    // A lock drops it.
    let tag_stale = {
        let s2 = a.s.lock().unlock(&pw(), &a.secret).unwrap();
        a.s = s2;
        a.s.commit_invite_sent(p, 1)
    };
    assert!(matches!(tag_stale, Err(Error::PreparedStale)));
    // Committing the real one marks the vault shared with the roster.
    let sent = invite(&mut a, v, &b, Role::Write);
    assert!(a.s.vault_shared(v).unwrap());
    assert_eq!(a.s.vault_own_role(v).unwrap(), Role::Owner);
    assert_eq!(a.s.vault_roster(v).unwrap().unwrap().len(), 2);
    assert_eq!(a.s.roster_seq(v).unwrap(), Some(1));
    // Inviting a member again is refused.
    assert!(matches!(
        a.s.prepare_invite(v, &b.id, &b.ipk, "b@x", Role::Read),
        Err(Error::AlreadyMember)
    ));
    drop(sent);
}

#[test]
fn accept_creates_shared_remote_vault() {
    let (mut a, mut b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("Team").unwrap();
    let item =
        a.s.add_item(v, &ItemType::Login, |e| e.set_title("Shared"))
            .unwrap();
    let sent = invite(&mut a, v, &b, Role::Write);

    // Previewing pins the sender unverified; accepting needs a verified pin.
    let preview =
        b.s.open_invite(&sent.incoming, &a.ipk, &sent.meta, &sent.roster)
            .unwrap();
    assert_eq!(preview.vault_name.as_deref(), Some("Team"));
    assert_eq!(preview.owner, a.id);
    assert_eq!(
        preview.sender_state,
        ContactState::Pinned { verified: false }
    );
    assert!(matches!(
        b.s.accept_invite(&sent.incoming.invite_id),
        Err(Error::ContactUnverified)
    ));
    b.s.verify_contact(&a.id, &fpr(&a)).unwrap();
    // An invite that was not previewed cannot be accepted.
    assert!(matches!(
        b.s.accept_invite(&InviteId::new().unwrap()),
        Err(Error::InviteNotOpened)
    ));
    let accepted: AcceptedInvite = b.s.accept_invite(&sent.incoming.invite_id).unwrap();
    assert_eq!(accepted.generation(), g(1));
    // The server names another owner than the roster does.
    let stranger = AccountId::new_random().unwrap();
    let wrong = b.s.accept_invite(&sent.incoming.invite_id).unwrap();
    assert!(matches!(
        b.s.commit_invite(wrong, Role::Write, &stranger),
        Err(Error::OwnerMismatch)
    ));
    b.s.commit_invite(accepted, Role::Write, &a.id).unwrap();

    assert!(b.s.vault_shared(v).unwrap());
    assert_eq!(b.s.vault_own_role(v).unwrap(), Role::Write);
    assert_eq!(b.s.vault_owner(v).unwrap(), a.id);
    assert_eq!(
        b.s.vault_key_origin(v, g(1)).unwrap(),
        Some(KeyOrigin::Remote)
    );
    assert!(b.s.vaults().iter().any(|x| x.vault_id == v && x.available));
    // The vault arrives with the next pull; its record opens under the key.
    let rec = a.s.store.record(&v, &item).unwrap().unwrap();
    let remote = super::RemoteRecord {
        vault_id: v,
        record_id: item,
        body: rec.body,
        server_seq: 1,
    };
    b.s.apply_remote_record(&remote).unwrap();
    assert_eq!(b.s.item(v, item).unwrap().title(), Some("Shared"));
    // The roster of the preview is replaced by the pulled one.
    b.s.apply_remote_roster(v, g(1), &sent.roster, 1, &a.id)
        .unwrap();
    assert_eq!(b.s.vault_roster(v).unwrap().unwrap().len(), 2);
    // Another invite for the vault B has joined already (a replay, here with
    // another role) hands it no key: the vault is known (H1).
    let mut tampered = sent.incoming.clone();
    tampered.role = Role::Manage;
    assert!(matches!(
        b.s.open_invite(&tampered, &a.ipk, &sent.meta, &sent.roster),
        Err(Error::VaultKnown)
    ));
}

#[test]
fn invite_for_another_recipient_does_not_open() {
    let (mut a, b, mut c) = (user("a@x"), user("b@x"), user("c@x"));
    let v = a.s.create_vault("V").unwrap();
    let sent = invite(&mut a, v, &b, Role::Write);
    // The server hands B's invite to C: the seal is addressed to B.
    assert!(matches!(
        c.s.open_invite(&sent.incoming, &a.ipk, &sent.meta, &sent.roster),
        Err(Error::InviteUnreadable)
    ));
}

#[test]
fn adoption_forbidden_for_shared_vault() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    let wrapped = a.s.store.vault_key(&v, g(1)).unwrap().unwrap();
    invite(&mut a, v, &b, Role::Write);
    assert!(matches!(
        a.s.adopt_remote_vault_key(v, g(1), &wrapped),
        Err(Error::AdoptionForbidden)
    ));
    assert!(matches!(
        a.s.prepare_vault_rotation(v),
        Err(Error::VaultShared)
    ));
    assert!(matches!(a.s.rotate_vault_key(v), Err(Error::VaultShared)));
}

#[test]
fn backend_change_refused_with_shared_vaults() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    let hosted = |url: &str, name: &str| SyncConfig {
        kind: SyncKind::Hosted,
        server_url: Some(url.to_owned()),
        email: Some("a@x".to_owned()),
        folder_path: None,
        device_name: name.to_owned(),
    };
    // Not shared yet: any change is fine.
    a.s.set_sync_config(Some(&hosted("https://two.example", "L")))
        .unwrap();
    a.s.set_sync_config(Some(&hosted("https://one.example", "L")))
        .unwrap();
    invite(&mut a, v, &b, Role::Write);
    for bad in [
        a.s.set_sync_config(Some(&hosted("https://two.example", "L"))),
        a.s.set_sync_config(None),
    ] {
        assert!(matches!(
            bad,
            Err(Error::Store(StorageError::HasSharedVaults))
        ));
    }
    // The same server under another device name is not a backend change.
    a.s.set_sync_config(Some(&hosted("https://one.example", "Desk")))
        .unwrap();
    // After losing access the backend can change.
    a.s.mark_access_lost(v).unwrap();
    assert!(a.s.vault_access_lost(v).unwrap());
    a.s.set_sync_config(Some(&hosted("https://two.example", "L")))
        .unwrap();
}

#[test]
fn shared_flag_survives_reset_sync_state() {
    let (mut a, b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    let sent = invite(&mut a, v, &b, Role::Write);
    a.s.mark_identity_published().unwrap();
    a.s.pin_contact(&b.id, &b.ipk, "b@x").unwrap();
    a.s.reset_sync_state().unwrap();
    assert!(a.s.vault_shared(v).unwrap());
    assert_eq!(a.s.vault_own_role(v).unwrap(), Role::Owner);
    assert!(a.s.identity().unwrap().unwrap().2, "published stays");
    // The sequence numbers are gone and the documents are queued again.
    assert_eq!(a.s.roster_seq(v).unwrap(), None);
    assert!(a.s.pending_roster_upload(v).unwrap().is_some());
    let (_, base) = a.s.pending_contacts_upload().unwrap().unwrap();
    assert_eq!(base, None);
    drop(sent);
}

/// A vault of `a` with one item and one attachment, uploaded.
fn vault_with_attachment(
    a: &mut User,
) -> (VaultId, mosaic_core::ItemId, mosaic_core::AttachmentId) {
    let v = a.s.create_vault("Files").unwrap();
    let item =
        a.s.add_item(v, &ItemType::Login, |e| e.set_title("With file"))
            .unwrap();
    let att =
        a.s.add_attachment(v, item, "f.txt", "text/plain", b"secret file")
            .unwrap();
    a.s.mark_attachment_uploaded(att).unwrap();
    (v, item, att)
}

#[test]
fn shared_rotation_reseals_attachments_with_fresh_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = user_in(Store::open(dir.path().join("a.db")).unwrap(), "a@x");
    let b = user("b@x");
    let (v, item, old) = vault_with_attachment(&mut a);
    invite(&mut a, v, &b, Role::Write);
    a.s.request_revocation(v, &b.id, RevocationReason::Removed)
        .unwrap();
    assert_eq!(a.s.pending_revocations().unwrap().len(), 1);

    let plan = RotationPlan {
        revoked: vec![b.id],
        ..RotationPlan::default()
    };
    let prepared = a.s.prepare_shared_rotation(v, &plan).unwrap();
    assert_eq!(prepared.generation(), g(2));
    assert_eq!(prepared.base_generation(), g(1));
    assert_eq!(prepared.unreachable_attachments().len(), 0);
    assert!(prepared.seals().is_empty() && prepared.revoke() == [b.id]);
    // Preparing changed nothing.
    assert_eq!(a.s.vaults()[0].key_generation, g(1));
    assert!(a.s.store.attachment_info(&old).unwrap().is_some());
    let roster_ct = prepared.roster().clone();

    let outcome = a.s.commit_shared_rotation(prepared).unwrap();
    assert_eq!(outcome.generation, g(2));
    assert_eq!(outcome.unreachable_attachments.len(), 0);
    // The attachment has a fresh ID (and key); the old one is gone and its
    // deletion is recorded for the server.
    let refs = a.s.item(v, item).unwrap().attachments();
    assert_eq!(refs.len(), 1);
    let new = refs[0].attachment_id;
    assert_ne!(new, old);
    assert!(a.s.store.attachment_info(&old).unwrap().is_none());
    assert_ne!(
        a.s.attachment_file_state(new).unwrap(),
        super::AttachmentFileState::Missing
    );
    assert_eq!(a.s.pending_attachment_deletions().unwrap(), vec![(v, old)]);
    assert_eq!(
        a.s.attachment(v, item, new).unwrap().expose_secret(),
        b"secret file"
    );
    assert_eq!(a.s.pending_attachment_uploads().unwrap().len(), 1);
    // Every record and the metadata are under the new key, queued; the
    // roster marks B removed at the new generation; the revocation is done.
    assert_eq!(
        a.s.store.records_below_generation(&v, g(2)).unwrap().len(),
        0
    );
    assert_eq!(
        a.s.store.vault_meta(&v).unwrap().unwrap().key_generation(),
        g(2)
    );
    assert_eq!(
        a.s.store.vault_roster(&v).unwrap().unwrap().ciphertext,
        roster_ct
    );
    let members = a.s.vault_roster(v).unwrap().unwrap();
    let removed = members.iter().find(|m| m.account_id == b.id).unwrap();
    assert_eq!((removed.role, removed.since), (Role::Removed, g(2)));
    assert_eq!(a.s.pending_revocations().unwrap().len(), 0);
    assert_eq!(a.s.vaults()[0].key_generation, g(2));
    // It survives a lock.
    let s = a.s.lock().unlock(&pw(), &a.secret).unwrap();
    assert_eq!(s.vaults()[0].key_generation, g(2));
    assert_eq!(
        s.attachment(v, item, new).unwrap().expose_secret(),
        b"secret file"
    );
}

#[test]
fn unreachable_attachment_listed_and_kept() {
    let dir = tempfile::tempdir().unwrap();
    let mut a = user_in(Store::open(dir.path().join("a.db")).unwrap(), "a@x");
    let b = user("b@x");
    let (v, item, att) = vault_with_attachment(&mut a);
    invite(&mut a, v, &b, Role::Write);
    // This device never downloaded the file.
    a.s.store.delete_attachment(&att).unwrap();
    let plan = RotationPlan {
        revoked: vec![b.id],
        ..RotationPlan::default()
    };
    let prepared = a.s.prepare_shared_rotation(v, &plan).unwrap();
    assert_eq!(prepared.unreachable_attachments(), [(item, att)]);
    let outcome = a.s.commit_shared_rotation(prepared).unwrap();
    assert_eq!(outcome.unreachable_attachments, vec![(item, att)]);
    // It keeps its ID and key in the item.
    let refs = a.s.item(v, item).unwrap().attachments();
    assert_eq!(refs[0].attachment_id, att);
}

#[test]
fn member_key_mismatch_blocks_rotation() {
    let (mut a, b, c, v) = trio();
    let (_, evil) = IdentitySecret::generate().unwrap();
    // The server serves B another key than the roster's fingerprint.
    let plan = RotationPlan {
        remaining: vec![(b.id, evil)],
        revoked: vec![c.id],
        ..RotationPlan::default()
    };
    assert!(matches!(
        a.s.prepare_shared_rotation(v, &plan),
        Err(Error::MemberKeyMismatch)
    ));
    // A pin that moved blocks too, even if the roster agrees.
    a.s.accept_key_change(&b.id, &evil, "b@x").unwrap();
    let ok = RotationPlan {
        remaining: vec![(b.id, b.ipk)],
        revoked: vec![c.id],
        ..RotationPlan::default()
    };
    assert!(matches!(
        a.s.prepare_shared_rotation(v, &ok),
        Err(Error::MemberKeyMismatch)
    ));
    // A plan that leaves a member out is refused.
    let short = RotationPlan {
        revoked: vec![c.id],
        ..RotationPlan::default()
    };
    assert!(matches!(
        a.s.prepare_shared_rotation(v, &short),
        Err(Error::PlanMismatch)
    ));
    // The owner cannot be revoked.
    assert!(matches!(
        a.s.request_revocation(v, &a.id, RevocationReason::Removed),
        Err(Error::RoleForbidden)
    ));
}

/// Rotation by A that removes C and keeps B; what B's page would show.
fn rotate_remove_c(
    a: &mut User,
    b: &User,
    c: &User,
    v: VaultId,
) -> (PulledShareView, (KeyGeneration, VaultRosterCiphertext)) {
    let plan = RotationPlan {
        remaining: vec![(b.id, b.ipk)],
        revoked: vec![c.id],
        ..RotationPlan::default()
    };
    let p = a.s.prepare_shared_rotation(v, &plan).unwrap();
    let share = PulledShareView {
        generation: p.generation(),
        sender: a.id,
        seal: p.seals()[0].1.clone(),
    };
    let roster = (p.generation(), p.roster().clone());
    a.s.commit_shared_rotation(p).unwrap();
    (share, roster)
}

#[test]
fn owner_rotation_is_claimed_by_members() {
    let (mut a, mut b, c, v) = trio();
    let (share, roster) = rotate_remove_c(&mut a, &b, &c, v);
    let ipks = |x: &AccountId| (*x == a.id).then_some(a.ipk);
    let chain: OpenedChain =
        b.s.open_share_chain(
            v,
            std::slice::from_ref(&share),
            std::slice::from_ref(&roster),
            &ipks,
        )
        .unwrap();
    assert_eq!(chain.generation(), g(2));
    let wrapped = b.s.claim_wrap(&chain).unwrap();
    b.s.commit_claimed_key(chain, &wrapped).unwrap();
    assert_eq!(b.s.vaults()[0].key_generation, g(2));
    assert_eq!(b.s.vault_roster(v).unwrap().unwrap().len(), 3);
    assert_eq!(b.s.vault_own_role(v).unwrap(), Role::Write);
    // Nothing is left to claim.
    assert!(matches!(
        b.s.open_share_chain(v, &[share], &[roster], &ipks),
        Err(Error::NothingToClaim)
    ));
}

#[test]
fn chain_claim_requires_owner_pin() {
    let (mut a, mut b, c, v) = trio();
    let (share, roster) = rotate_remove_c(&mut a, &b, &c, v);
    let ipks = |x: &AccountId| (*x == a.id).then_some(a.ipk);
    // The served key is not the owner's pin: untrusted.
    let (_, evil) = IdentitySecret::generate().unwrap();
    let evil_ipks = |x: &AccountId| (*x == a.id).then_some(evil);
    assert!(matches!(
        b.s.open_share_chain(
            v,
            std::slice::from_ref(&share),
            std::slice::from_ref(&roster),
            &evil_ipks
        ),
        Err(Error::ShareUntrusted { generation: 2 })
    ));
    // The server withholds the key.
    assert!(matches!(
        b.s.open_share_chain(
            v,
            std::slice::from_ref(&share),
            std::slice::from_ref(&roster),
            &|_| None
        ),
        Err(Error::ShareUntrusted { .. })
    ));
    // The pin of the owner moved: untrusted even for the real key.
    b.s.accept_key_change(&a.id, &evil, "a@x").unwrap();
    assert!(matches!(
        b.s.open_share_chain(v, &[share], &[roster], &ipks),
        Err(Error::ShareUntrusted { .. })
    ));
}

#[test]
fn chain_claim_refuses_non_owner_sender() {
    // C was removed (or is a promoted member) and colludes with the server:
    // they seal a key of their choosing for B at the next generation and
    // write a roster for it that lists B. C is not the owner, so B refuses,
    // whatever the roster says.
    let (a, mut b, c, v) = trio();
    let vk2 = VaultKey::generate(g(2)).unwrap();
    let forged = {
        let id = c.s.identity_key.as_ref().unwrap();
        seal_vault_key(&vk2, &v, &c.id, &id.isk, &b.id, &b.ipk).unwrap()
    };
    let mut roster = VaultRoster::new(g(2), a.id);
    {
        let mut e = roster.edit(c.s.device_id(), super::now());
        e.set_member(&a.id, Role::Owner, g(1), &fpr(&a), "a@x")
            .unwrap();
        e.set_member(&b.id, Role::Write, g(1), &fpr(&b), "b@x")
            .unwrap();
        e.set_member(&c.id, Role::Manage, g(1), &fpr(&c), "c@x")
            .unwrap();
    }
    let roster_ct = roster.seal(&vk2, &v).unwrap();
    let share = PulledShareView {
        generation: g(2),
        sender: c.id,
        seal: forged,
    };
    let ipks = |x: &AccountId| {
        if *x == c.id {
            Some(c.ipk)
        } else if *x == a.id {
            Some(a.ipk)
        } else {
            None
        }
    };
    b.s.pin_contact(&c.id, &c.ipk, "c@x").unwrap();
    b.s.verify_contact(&c.id, &fpr(&c)).unwrap();
    let err =
        b.s.open_share_chain(v, &[share], &[(g(2), roster_ct)], &ipks)
            .unwrap_err();
    assert!(matches!(err, Error::ShareUntrusted { generation: 2 }));
    b.s.record_sharing_finding(SharingFinding::ShareUntrusted {
        vault: v,
        generation: g(2),
        sender: c.id,
    })
    .unwrap();
    assert_eq!(b.s.sharing_findings().unwrap().len(), 1);
    b.s.clear_sharing_findings().unwrap();
    assert_eq!(b.s.sharing_findings().unwrap().len(), 0);
    assert_eq!(b.s.vaults()[0].key_generation, g(1));
}

#[test]
fn chain_claim_refuses_a_roster_that_drops_the_member() {
    let (mut a, b, c, v) = trio();
    // B is removed at generation 2 yet a share for B arrives (a replay of a
    // seal made before): the roster of generation 2 does not list B.
    let plan = RotationPlan {
        remaining: vec![(c.id, c.ipk)],
        revoked: vec![b.id],
        ..RotationPlan::default()
    };
    let p = a.s.prepare_shared_rotation(v, &plan).unwrap();
    let roster = (p.generation(), p.roster().clone());
    let for_c = PulledShareView {
        generation: p.generation(),
        sender: a.id,
        seal: p.seals()[0].1.clone(),
    };
    a.s.commit_shared_rotation(p).unwrap();
    let ipks = |x: &AccountId| (*x == a.id).then_some(a.ipk);
    // A share addressed to C does not open for B at all.
    assert!(matches!(
        b.s.open_share_chain(v, &[for_c], &[roster], &ipks),
        Err(Error::ShareUntrusted { .. })
    ));
}

#[test]
fn manager_may_invite_but_not_rotate() {
    let (mut a, mut b, _c, v) = {
        let (mut a, mut b, mut c) = (user("a@x"), user("b@x"), user("c@x"));
        let v = a.s.create_vault("Team").unwrap();
        let sent = invite(&mut a, v, &b, Role::Manage);
        accept(&mut b, &a, &sent, Role::Manage);
        b.s.apply_remote_roster(v, g(1), &current_roster(&a, v), 2, &a.id)
            .unwrap();
        let _ = &mut c;
        (a, b, c, v)
    };
    let d = user("d@x");
    // The manager invites with the current key, at a role up to its own.
    verify(&mut b, &d, "d@x");
    let p =
        b.s.prepare_invite(v, &d.id, &d.ipk, "d@x", Role::Write)
            .unwrap();
    assert_eq!(p.generation(), g(1));
    assert!(matches!(
        b.s.prepare_invite(v, &d.id, &d.ipk, "d@x", Role::Owner),
        Err(Error::RoleForbidden)
    ));
    // But only the owner rotates, and a manager's removal waits for it.
    let plan = RotationPlan {
        revoked: vec![d.id],
        ..RotationPlan::default()
    };
    assert!(matches!(
        b.s.prepare_shared_rotation(v, &plan),
        Err(Error::NotOwner)
    ));
    b.s.request_revocation(v, &d.id, RevocationReason::Removed)
        .unwrap();
    assert_eq!(b.s.pending_revocations().unwrap().len(), 1);
    assert!(matches!(
        b.s.request_revocation(v, &a.id, RevocationReason::Removed),
        Err(Error::OwnerCannotBeRevoked)
    ));
    // A member without a manage role cannot invite.
    let e = user("e@x");
    let mut w = user("w@x");
    let sent = invite(&mut a, v, &w, Role::Write);
    accept(&mut w, &a, &sent, Role::Write);
    w.s.apply_remote_roster(v, g(1), &current_roster(&a, v), 3, &a.id)
        .unwrap();
    verify(&mut w, &e, "e@x");
    assert!(matches!(
        w.s.prepare_invite(v, &e.id, &e.ipk, "e@x", Role::Read),
        Err(Error::RoleForbidden)
    ));
}

#[test]
fn role_change_is_queued_in_the_roster() {
    let (mut a, mut b, _c, v) = trio();
    a.s.set_member_role(v, &b.id, Role::Manage).unwrap();
    let (ct, base) = a.s.pending_roster_upload(v).unwrap().unwrap();
    assert_eq!(base, Some(1));
    a.s.mark_roster_uploaded(v, &ct, 9).unwrap();
    assert!(a.s.pending_roster_upload(v).unwrap().is_none());
    let members = a.s.vault_roster(v).unwrap().unwrap();
    assert_eq!(
        members.iter().find(|m| m.account_id == b.id).unwrap().role,
        Role::Manage
    );
    // The member's device learns its new role from the roster.
    b.s.apply_remote_roster(v, g(1), &ct, 9, &a.id).unwrap();
    assert_eq!(b.s.vault_own_role(v).unwrap(), Role::Manage);
    // The owner's role cannot be changed, a write member cannot change roles.
    assert!(matches!(
        a.s.set_member_role(v, &a.id, Role::Read),
        Err(Error::RoleForbidden)
    ));
    assert!(matches!(
        a.s.set_member_role(v, &b.id, Role::Owner),
        Err(Error::RoleForbidden)
    ));
}

#[test]
fn roster_of_another_owner_or_generation_is_refused() {
    let (a, mut b, _c, v) = trio();
    let stranger = AccountId::new_random().unwrap();
    let roster = current_roster(&a, v);
    assert!(matches!(
        b.s.apply_remote_roster(v, g(1), &roster, 3, &stranger),
        Err(Error::OwnerMismatch)
    ));
    assert!(matches!(
        b.s.apply_remote_roster(v, g(2), &roster, 3, &a.id),
        Err(Error::KeyGenerationNotAccepted { .. })
    ));
}

#[test]
fn roster_that_names_a_second_owner_raises_a_finding() {
    let (a, mut b, _c, v) = trio();
    let vk = b.s.vault(v).unwrap().current_key().unwrap();
    let mut forged = VaultRoster::new(g(1), a.id);
    {
        let mut e = forged.edit(b.s.device_id(), super::now());
        e.set_member(&a.id, Role::Owner, g(1), &fpr(&a), "a@x")
            .unwrap();
        e.set_member(&b.id, Role::Owner, g(1), &fpr(&b), "b@x")
            .unwrap();
    }
    let ct = forged.seal(vk, &v).unwrap();
    assert!(matches!(
        b.s.apply_remote_roster(v, g(1), &ct, 5, &a.id),
        Err(Error::Core(mosaic_core::Error::RosterInvalid))
    ));
    assert_eq!(
        b.s.sharing_findings().unwrap(),
        vec![SharingFinding::RosterInvalid { vault: v }]
    );
}

#[test]
fn emergency_seals_round_trip() {
    let (mut a, mut b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("Mine").unwrap();
    let item =
        a.s.add_item(v, &ItemType::Login, |e| e.set_title("Vault item"))
            .unwrap();
    // Needs a verified grantee.
    assert!(matches!(
        a.s.seal_emergency(&b.id, &b.ipk, &[v]),
        Err(Error::ContactUnverified)
    ));
    verify(&mut a, &b, "b@x");
    let seals = a.s.seal_emergency(&b.id, &b.ipk, &[v]).unwrap();
    assert_eq!(seals.len(), 1);
    assert_eq!(seals[0].1, g(1));

    // B opens the release; an unverified grantor is allowed.
    let opened = b.s.open_emergency_release(&a.id, &a.ipk, &seals).unwrap();
    assert_eq!(opened.len(), 1);
    assert_eq!(opened[0].owner(), a.id);
    assert_eq!(opened[0].role(), Role::Read);
    for o in opened {
        b.s.commit_invite(o, Role::Read, &a.id).unwrap();
    }
    assert_eq!(b.s.vault_own_role(v).unwrap(), Role::Read);
    let rec = a.s.store.record(&v, &item).unwrap().unwrap();
    b.s.apply_remote_record(&super::RemoteRecord {
        vault_id: v,
        record_id: item,
        body: rec.body,
        server_seq: 1,
    })
    .unwrap();
    assert_eq!(b.s.item(v, item).unwrap().title(), Some("Vault item"));
}

#[test]
fn emergency_seal_checks_owner_pin_and_forgery() {
    let (mut a, mut b, c, v) = trio();
    // Only owned vaults can be sealed: B does not own the shared vault.
    verify(&mut b, &c, "c@x");
    assert!(matches!(
        b.s.seal_emergency(&c.id, &c.ipk, &[v]),
        Err(Error::NotOwner)
    ));
    verify(&mut a, &c, "c@x");
    let mut seals = a.s.seal_emergency(&c.id, &c.ipk, &[v]).unwrap();
    // A seal under another vault ID (the server swaps it) fails, and records a
    // finding; nothing is stored.
    let other = VaultId::new_v7().unwrap();
    seals[0].0 = other;
    let mut c = c;
    assert!(matches!(
        c.s.open_emergency_release(&a.id, &a.ipk, &seals),
        Err(Error::ShareUntrusted { .. })
    ));
    assert_eq!(c.s.sharing_findings().unwrap().len(), 1);
    // A changed key of the grantor is refused before anything is opened.
    let (_, evil) = IdentitySecret::generate().unwrap();
    c.s.pin_contact(&a.id, &a.ipk, "a@x").unwrap();
    assert!(matches!(
        c.s.open_emergency_release(&a.id, &evil, &seals),
        Err(Error::ContactKeyChanged)
    ));
}

#[test]
fn opened_invite_keys_are_swept_after_five_minutes() {
    // M6b review L4: the key does not sit in memory after its time.
    let (mut a, mut b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    let sent = invite(&mut a, v, &b, Role::Write);
    b.s.open_invite(&sent.incoming, &a.ipk, &sent.meta, &sent.roster)
        .unwrap();
    assert_eq!(b.s.pending_invites.len(), 1);
    b.s.sweep_pending_invites();
    assert_eq!(
        b.s.pending_invites.len(),
        1,
        "still within its five minutes"
    );
    b.s.age_pending_invites(std::time::Duration::from_secs(301));
    b.s.sweep_pending_invites();
    assert!(b.s.pending_invites.is_empty());
}

#[test]
fn first_roster_with_another_owner_needs_a_verified_contact() {
    // M6b review H3: a vault this device never saw shared is not handed to
    // whoever holds its key and forges a roster.
    let (mut a, b) = (user("a@x"), user("b@x"));
    let v = a.s.create_vault("V").unwrap();
    let ct = {
        let vk = a.s.vaults.get(&v).unwrap().current_key().unwrap();
        let mut roster = VaultRoster::new(g(1), b.id);
        {
            let mut edit = roster.edit(
                mosaic_core::DeviceId::new_v7().unwrap(),
                mosaic_core::model::Timestamp::from_unix_millis(1),
            );
            edit.set_member(&b.id, Role::Owner, g(1), &fpr(&b), "b@x")
                .unwrap();
            edit.set_member(&a.id, Role::Manage, g(1), &fpr(&a), "a@x")
                .unwrap();
        }
        roster.seal(vk, &v).unwrap()
    };
    // B is a stranger, or only pinned: refused, and the vault stays unshared.
    assert!(matches!(
        a.s.apply_remote_roster(v, g(1), &ct, 1, &b.id),
        Err(Error::OwnerMismatch)
    ));
    a.s.pin_contact(&b.id, &b.ipk, "b@x").unwrap();
    assert!(matches!(
        a.s.apply_remote_roster(v, g(1), &ct, 1, &b.id),
        Err(Error::OwnerMismatch)
    ));
    assert!(!a.s.vault_shared(v).unwrap());
}
