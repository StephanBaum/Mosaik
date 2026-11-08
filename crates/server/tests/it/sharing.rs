//! Identity keys, the directory, invites, roles, rosters, shared-vault
//! rotation, claims and the abuse limits (M6b spec §5, §8.4).

use axum::http::{Method, StatusCode};
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::{
    AccountId, AccountKey, AttachmentId, ContactsCiphertext, InstallId, ItemId, KeyGeneration,
    VaultKey, VaultRosterCiphertext,
};
use mosaic_server::ApiError;
use mosaic_sync::wire::json::RegistrationMode;
use serde_json::{Value, json};

use crate::common::{LoginResult, Resp, T0, TestApp, TestVault, User, wrapped_key};

const HOUR: u64 = 3_600_000;

// ---- helpers (shared with emergency.rs) -----------------------------------

pub struct Acct {
    pub u: User,
    pub t: [u8; 32],
    pub id: String,
}

static NEXT_IP: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(1);

/// Registers and logs in a user from an address of its own (registration is
/// limited to three per hour and address).
pub async fn acct(app: &TestApp, name: &str) -> Acct {
    let n = NEXT_IP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let ip = format!("203.0.113.{n}:4000");
    let u = User::new(&format!("{name}@example.com"));
    let r = app.register_from(&u, None, &ip).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    let LoginResult::Ok { token: t, .. } = app
        .login_with(&u.email, &u.opw, u.account_id, u.device, &ip)
        .await
    else {
        panic!("login failed");
    };
    let id = u.account_id.to_string();
    Acct { u, t, id }
}

pub fn b64(b: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(b)
}

pub fn uuid() -> String {
    ItemId::new_v7().unwrap().to_string()
}

/// A structurally valid seal (the server never opens one).
pub fn seal(fill: u8) -> String {
    let mut b = [fill; 82];
    b[0] = 1;
    b[1] = 3;
    b64(&b)
}

pub fn device(app: &TestApp, a: &Acct) -> String {
    app.device_of(&a.t).to_string()
}

pub fn vault_url(v: &TestVault, tail: &str) -> String {
    format!("/v1/vaults/{}{tail}", v.id)
}

pub async fn publish(app: &TestApp, a: &Acct, fill: u8) {
    let r = app
        .put(
            "/v1/account/identity",
            Some(&a.t),
            &json!({"ipk": b64(&[fill; 32]), "wrappedIsk": b64(wrapped_key(fill).as_bytes())}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT, "{}", text(&r));
}

pub fn text(r: &Resp) -> String {
    String::from_utf8_lossy(&r.body).into_owned()
}

pub async fn head(app: &TestApp, a: &Acct, v: &TestVault) -> u64 {
    let r = app
        .get(&vault_url(v, "/changes?since=0&limit=1"), Some(&a.t))
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    r.json::<Value>()["headSeq"].as_u64().unwrap()
}

pub async fn invite(
    app: &TestApp,
    from: &Acct,
    v: &TestVault,
    to: &str,
    role: u8,
    generation: u32,
) -> (String, Resp) {
    let id = uuid();
    let r = app
        .post(
            &vault_url(v, "/invites"),
            Some(&from.t),
            &json!({"inviteId": id, "recipient": to, "role": role, "generation": generation,
                    "seal": seal(7)}),
        )
        .await;
    (id, r)
}

pub async fn accept(app: &TestApp, who: &Acct, invite_id: &str) -> Resp {
    app.post(
        &format!("/v1/invites/{invite_id}/accept"),
        Some(&who.t),
        &json!({"wrapped": b64(wrapped_key(9).as_bytes()), "device": device(app, who)}),
    )
    .await
}

/// Invites `who` at generation 1 and accepts.
pub async fn join(app: &TestApp, owner: &Acct, v: &TestVault, who: &Acct, role: u8) {
    let (id, r) = invite(app, owner, v, &who.id, role, 1).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    // Listing delivers it.
    let list = app.get("/v1/invites", Some(&who.t)).await;
    assert_eq!(
        list.json::<Value>()["incoming"].as_array().unwrap().len(),
        1
    );
    let r = accept(app, who, &id).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
}

pub async fn vault_with(app: &TestApp, owner: &Acct) -> TestVault {
    let v = TestVault::new();
    assert_eq!(
        app.create_vault(&owner.t, &v).await.status,
        StatusCode::CREATED
    );
    v
}

pub fn roster(v: &TestVault, generation: u32) -> String {
    let vk = VaultKey::generate(KeyGeneration::new(generation).unwrap()).unwrap();
    b64(VaultRosterCiphertext::seal(&vk, &v.id, b"{}")
        .unwrap()
        .as_bytes())
}

pub struct Rot<'a> {
    pub generation: u32,
    pub seals: &'a [&'a str],
    pub invite_seals: &'a [&'a str],
    pub revoke: &'a [&'a str],
}

pub async fn rotate(app: &TestApp, owner: &Acct, v: &TestVault, r: &Rot<'_>) -> Resp {
    let head = head(app, owner, v).await;
    app.post(
        &vault_url(v, "/rotate"),
        Some(&owner.t),
        &json!({
            "generation": r.generation,
            "wrapped": b64(wrapped_key(5).as_bytes()),
            "device": device(app, owner),
            "expectedHead": head,
            "roster": roster(v, r.generation),
            "seals": r.seals.iter().map(|a| json!({"recipient": a, "seal": seal(3)})).collect::<Vec<_>>(),
            "inviteSeals": r.invite_seals.iter().map(|i| json!({"inviteId": i, "seal": seal(4)})).collect::<Vec<_>>(),
            "revoke": r.revoke,
        }),
    )
    .await
}

fn sql<T>(app: &TestApp, f: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T>) -> T {
    app.state
        .db()
        .run(|c| f(c).map_err(ApiError::from))
        .unwrap()
}

fn code(r: &Resp) -> String {
    r.error().error
}

// ---- identity and contacts -------------------------------------------------

#[tokio::test(start_paused = true)]
async fn identity_is_first_write_wins_and_served_on_request() {
    let app = TestApp::new(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    // Not served without the flag (older clients parse the page strictly).
    let plain: Value = app.get("/v1/account", Some(&a.t)).await.json();
    assert!(plain.get("identity").is_none());
    let none: Value = app.get("/v1/account?identity=1", Some(&a.t)).await.json();
    assert!(none["identity"].is_null());

    publish(&app, &a, 1).await;
    // The same pair again is fine; a different one is refused with the stored pair.
    publish(&app, &a, 1).await;
    let r = app
        .put(
            "/v1/account/identity",
            Some(&a.t),
            &json!({"ipk": b64(&[2; 32]), "wrappedIsk": b64(wrapped_key(2).as_bytes())}),
        )
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "identity_exists");
    let d = r.error().detail.unwrap();
    assert_eq!(d["ipk"], b64(&[1; 32]));
    assert_eq!(d["wrappedIsk"], b64(wrapped_key(1).as_bytes()));
    let got: Value = app.get("/v1/account?identity=1", Some(&a.t)).await.json();
    assert_eq!(got["identity"]["ipk"], b64(&[1; 32]));
    // Malformed pairs.
    let b = acct(&app, "bob").await;
    for body in [
        json!({"ipk": b64(&[0; 32]), "wrappedIsk": b64(wrapped_key(1).as_bytes())}),
        json!({"ipk": b64(&[1; 31]), "wrappedIsk": b64(wrapped_key(1).as_bytes())}),
        json!({"ipk": b64(&[1; 32]), "wrappedIsk": b64(&[0; 74])}),
    ] {
        let r = app.put("/v1/account/identity", Some(&b.t), &body).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{body}");
    }
}

#[tokio::test(start_paused = true)]
async fn contact_book_is_a_compare_and_swap() {
    let app = TestApp::new(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    let ck = AccountKey::generate().unwrap().contacts_key().unwrap();
    let ct = |text: &[u8]| {
        b64(ContactsCiphertext::seal(&ck, &a.u.account_id, text)
            .unwrap()
            .as_bytes())
    };
    let r: Value = app.get("/v1/account/contacts", Some(&a.t)).await.json();
    assert!(r["ciphertext"].is_null());
    assert_eq!(r["seq"], 0);
    let put = |c: String, base: u64| {
        let app = &app;
        let t = a.t;
        async move {
            app.put(
                "/v1/account/contacts",
                Some(&t),
                &json!({"ciphertext": c, "baseSeq": base}),
            )
            .await
        }
    };
    let r = put(ct(b"one"), 0).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json::<Value>()["seq"], 1);
    // A stale base is a conflict that names the stored sequence.
    let r = put(ct(b"two"), 0).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "conflict");
    assert_eq!(r.error().detail.unwrap()["currentSeq"], 1);
    assert_eq!(put(ct(b"two"), 1).await.json::<Value>()["seq"], 2);
    // Not a contacts ciphertext.
    assert_eq!(
        put(b64(&[1, 1, 0, 0]), 2).await.status,
        StatusCode::BAD_REQUEST
    );
    let got: Value = app.get("/v1/account/contacts", Some(&a.t)).await.json();
    assert_eq!(got["seq"], 2);
}

// ---- directory --------------------------------------------------------------

async fn lookup(app: &TestApp, t: &[u8; 32], email: &str) -> Resp {
    app.post("/v1/directory/lookup", Some(t), &json!({"email": email}))
        .await
}

#[tokio::test(start_paused = true)]
async fn directory_fake_matches_prelogin_and_is_stable() {
    let app = TestApp::new(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    let r = lookup(&app, &a.t, "Nobody@Example.com").await;
    assert_eq!(r.status, StatusCode::OK);
    let v: Value = r.json();
    // The same account ID the pre-login route gives for that address.
    let pre: Value = app.prelogin("nobody@example.com").await.json();
    assert_eq!(v["accountId"], pre["accountId"]);
    // Stable, and the key looks like an X25519 public key (top bit clear).
    let again: Value = lookup(&app, &a.t, "nobody@example.com").await.json();
    assert_eq!(v, again);
    let ipk = Base64UrlUnpadded::decode_vec(v["ipk"].as_str().unwrap()).unwrap();
    assert_eq!(ipk.len(), 32);
    assert!(ipk[31] < 0x80);
    let other: Value = lookup(&app, &a.t, "somebody@example.com").await.json();
    assert_ne!(v["accountId"], other["accountId"]);
    assert_ne!(v["ipk"], other["ipk"]);
    // A malformed address is the only refusal.
    let r = lookup(&app, &a.t, "not an email").await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(code(&r), "invalid_email");
}

#[tokio::test(start_paused = true)]
async fn directory_unknown_and_unpublished_indistinguishable() {
    let app = TestApp::new(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    let b = acct(&app, "bob").await;
    let unpublished = lookup(&app, &a.t, &b.u.email).await;
    let unknown = lookup(&app, &a.t, "ghost@example.com").await;
    assert_eq!(unpublished.status, unknown.status);
    let (x, y): (Value, Value) = (unpublished.json(), unknown.json());
    let keys = |v: &Value| v.as_object().unwrap().keys().cloned().collect::<Vec<_>>();
    assert_eq!(keys(&x), keys(&y));
    // The registered account's real ID, with a derived key; the unknown one
    // gets derived values for both.
    assert_eq!(x["accountId"], b.id);
    assert_eq!(
        x["ipk"].as_str().unwrap().len(),
        y["ipk"].as_str().unwrap().len()
    );
    assert_eq!(x["accountId"].as_str().unwrap().len(), 36);
    assert_eq!(y["accountId"].as_str().unwrap().len(), 36);
    // After publication the real key is served.
    publish(&app, &b, 8).await;
    let real: Value = lookup(&app, &a.t, &b.u.email).await.json();
    assert_eq!(real["ipk"], b64(&[8; 32]));
    assert_eq!(real["accountId"], b.id);
}

#[tokio::test(start_paused = true)]
async fn directory_by_id_serves_only_related_accounts() {
    let app = TestApp::new(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    let b = acct(&app, "bob").await;
    let c = acct(&app, "carol").await;
    for (acc, fill) in [(&a, 1), (&b, 2), (&c, 3)] {
        publish(&app, acc, fill).await;
    }
    let by_id = |who: &Acct, id: String| {
        let app = &app;
        let t = who.t;
        async move { app.get(&format!("/v1/directory/{id}"), Some(&t)).await }
    };
    assert_eq!(by_id(&a, b.id.clone()).await.status, StatusCode::NOT_FOUND);
    // An invite from A: B may look A up, A may not look B up by ID (the
    // answer would tell a real recipient from a fake one).
    let v = vault_with(&app, &a).await;
    let (_, r) = invite(&app, &a, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = by_id(&b, a.id.clone()).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json::<Value>()["ipk"], b64(&[1; 32]));
    assert_eq!(by_id(&a, b.id.clone()).await.status, StatusCode::NOT_FOUND);
    // Co-members.
    let id = sql(&app, |c| {
        c.query_row("SELECT invite_id FROM share_invites", [], |r| {
            r.get::<_, Vec<u8>>(0)
        })
    });
    let invite_id = mosaic_core::ItemId::from_slice(&id).unwrap().to_string();
    assert_eq!(accept(&app, &b, &invite_id).await.status, StatusCode::OK);
    assert_eq!(by_id(&a, b.id.clone()).await.status, StatusCode::OK);
    // C shares nothing with anyone.
    assert_eq!(by_id(&c, a.id.clone()).await.status, StatusCode::NOT_FOUND);
    assert_eq!(by_id(&a, c.id.clone()).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test(start_paused = true)]
async fn rate_limits_directory_and_invites() {
    let app = TestApp::new(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    for i in 0..30 {
        let r = lookup(&app, &a.t, &format!("u{i}@example.com")).await;
        assert_eq!(r.status, StatusCode::OK, "lookup {i}");
    }
    let r = lookup(&app, &a.t, "u31@example.com").await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(r.headers.contains_key("retry-after"));
    // Another session of the same account has its own bucket but shares the
    // 60 per hour of the account.
    let t2 = app.login_device(&a.u, InstallId::new_v7().unwrap()).await;
    for i in 0..30 {
        assert_eq!(
            lookup(&app, &t2, &format!("w{i}@example.com")).await.status,
            StatusCode::OK
        );
    }
    assert_eq!(
        lookup(&app, &t2, "w31@example.com").await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    // The bucket refills.
    app.clock.advance_ms(HOUR);
    assert_eq!(
        lookup(&app, &t2, "again@example.com").await.status,
        StatusCode::OK
    );

    // 50 invites per hour: two vaults of 25 (a vault holds at most 50
    // members and open invites together).
    let v1 = vault_with(&app, &a).await;
    let v2 = vault_with(&app, &a).await;
    for i in 0..50 {
        let v = if i % 2 == 0 { &v1 } else { &v2 };
        let (_, r) = invite(&app, &a, v, &uuid(), 3, 1).await;
        assert_eq!(r.status, StatusCode::CREATED, "invite {i}: {}", text(&r));
    }
    let (_, r) = invite(&app, &a, &v1, &uuid(), 3, 1).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(code(&r), "rate_limited");
}

// ---- roles ------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn role_matrix() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let manage = acct(&app, "manage").await;
    let write = acct(&app, "write").await;
    let read = acct(&app, "read").await;
    let v = vault_with(&app, &owner).await;
    join(&app, &owner, &v, &manage, 2).await;
    join(&app, &owner, &v, &write, 3).await;
    join(&app, &owner, &v, &read, 4).await;
    let forbidden = |r: &Resp, what: &str| {
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{what}: {}", text(r));
        assert_eq!(code(r), "role_forbidden", "{what}");
    };

    // Reads work for every role, and the page names the role.
    for (a, role) in [(&owner, 1), (&manage, 2), (&write, 3), (&read, 4)] {
        let r = app
            .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&a.t))
            .await;
        assert_eq!(r.status, StatusCode::OK);
        assert_eq!(r.json::<Value>()["role"], role);
        let r = app.get(&vault_url(&v, "/members"), Some(&a.t)).await;
        assert_eq!(r.status, StatusCode::OK);
        assert_eq!(r.json::<Value>()["members"].as_array().unwrap().len(), 4);
        assert_eq!(
            app.get("/v1/vaults", Some(&a.t)).await.status,
            StatusCode::OK
        );
    }

    // Records, metadata and attachments: owner, manage and write; not read.
    for (a, may) in [
        (&owner, true),
        (&manage, true),
        (&write, true),
        (&read, false),
    ] {
        let rec = v.item_record(ItemId::new_v7().unwrap(), "x", None);
        let r = app.push(&a.t, &v, vec![rec]).await;
        if may {
            assert_eq!(r.status, StatusCode::OK, "push {}", text(&r));
        } else {
            forbidden(&r, "push");
        }
        let r = app
            .put(
                &vault_url(&v, "/meta"),
                Some(&a.t),
                &json!({"ciphertext": b64(&v.meta()), "baseSeq": null}),
            )
            .await;
        if may {
            assert_eq!(r.status, StatusCode::OK, "meta");
        } else {
            forbidden(&r, "meta");
        }
        let att = AttachmentId::new_v7().unwrap();
        let body = v.attachment(ItemId::new_v7().unwrap(), att, 100);
        let r = app
            .send(
                Method::PUT,
                &vault_url(&v, &format!("/attachments/{att}")),
                Some(&a.t),
                crate::common::IP,
                body,
            )
            .await;
        let del = app
            .delete(&vault_url(&v, &format!("/attachments/{att}")), Some(&a.t))
            .await;
        let restore = app
            .post(
                &vault_url(&v, &format!("/attachments/{att}/restore")),
                Some(&a.t),
                &json!({}),
            )
            .await;
        if may {
            assert_eq!(r.status, StatusCode::CREATED, "attachment {}", text(&r));
            assert_eq!(del.status, StatusCode::OK);
            assert_eq!(restore.status, StatusCode::OK);
        } else {
            forbidden(&r, "attachment put");
            forbidden(&del, "attachment delete");
            forbidden(&restore, "attachment restore");
        }
    }

    // The roster and invites: manage and owner.
    let mut base: Option<u64> = None;
    for (a, may) in [
        (&owner, true),
        (&manage, true),
        (&write, false),
        (&read, false),
    ] {
        let r = app
            .put(
                &vault_url(&v, "/roster"),
                Some(&a.t),
                &json!({"generation": 1, "ciphertext": roster(&v, 1), "baseSeq": base}),
            )
            .await;
        if may {
            assert_eq!(r.status, StatusCode::OK, "roster {}", text(&r));
            base = r.json::<Value>()["seq"].as_u64();
        } else {
            forbidden(&r, "roster");
        }
        let (_, r) = invite(&app, a, &v, &uuid(), 4, 1).await;
        if may {
            assert_eq!(r.status, StatusCode::CREATED, "invite {}", text(&r));
        } else {
            forbidden(&r, "invite");
        }
    }
    // A manager invites no more than their own role; nobody invites an owner.
    let (_, r) = invite(&app, &manage, &v, &uuid(), 1, 1).await;
    forbidden(&r, "invite owner");
    let (_, r) = invite(&app, &owner, &v, &uuid(), 1, 1).await;
    forbidden(&r, "invite owner by owner");

    // Role changes: manage and owner, never the owner's, never above one's own.
    let set = |a: &Acct, target: &str, role: u8| {
        let app = &app;
        let (t, url) = (a.t, vault_url(&v, &format!("/members/{target}")));
        async move { app.put(&url, Some(&t), &json!({"role": role})).await }
    };
    forbidden(&set(&write, &read.id, 4).await, "role by writer");
    forbidden(&set(&read, &write.id, 4).await, "role by reader");
    forbidden(&set(&manage, &owner.id, 4).await, "demote owner");
    forbidden(&set(&owner, &owner.id, 2).await, "owner role change");
    forbidden(&set(&owner, &write.id, 1).await, "promote to owner");
    assert_eq!(
        set(&manage, &write.id, 4).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        set(&owner, &write.id, 3).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(set(&manage, &uuid(), 3).await.status, StatusCode::NOT_FOUND);

    // Rotation and the plain key upload are the owner's alone.
    for a in [&manage, &write, &read] {
        let r = app
            .post(
                &vault_url(&v, "/rotate"),
                Some(&a.t),
                &json!({"generation": 2, "wrapped": b64(wrapped_key(5).as_bytes()),
                        "device": device(&app, a), "expectedHead": 0, "roster": roster(&v, 2),
                        "seals": [], "inviteSeals": [], "revoke": []}),
            )
            .await;
        forbidden(&r, "rotate");
        let r = app
            .put(
                &vault_url(&v, "/keys/2"),
                Some(&a.t),
                &json!({"wrappedKey": b64(wrapped_key(5).as_bytes()),
                        "deviceId": device(&app, a), "expectedHeadSeq": null}),
            )
            .await;
        forbidden(&r, "put key");
    }
    // A non-member sees neither.
    let stranger = acct(&app, "stranger").await;
    assert_eq!(
        app.get(&vault_url(&v, "/members"), Some(&stranger.t))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

// ---- rotation ---------------------------------------------------------------

/// An owner with a manager and a reader in one vault.
async fn trio(app: &TestApp) -> (Acct, Acct, Acct, TestVault) {
    let owner = acct(app, "owner").await;
    let b = acct(app, "bob").await;
    let c = acct(app, "carol").await;
    let v = vault_with(app, &owner).await;
    join(app, &owner, &v, &b, 2).await;
    join(app, &owner, &v, &c, 4).await;
    (owner, b, c, v)
}

#[tokio::test(start_paused = true)]
async fn rotate_requires_all_members_sealed_or_revoked() {
    let app = TestApp::new(RegistrationMode::Open);
    let (owner, b, c, v) = trio(&app).await;
    // Missing a member.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", text(&r));
    assert_eq!(code(&r), "members_mismatch");
    let d = r.error().detail.unwrap();
    assert_eq!(d["members"].as_array().unwrap().len(), 2);
    // Sealed and revoked at once; a stranger; the owner herself.
    let stranger = uuid();
    for (seals, revoke) in [
        (vec![&b.id, &c.id], vec![&b.id]),
        (vec![&b.id, &c.id, &stranger], vec![]),
        (vec![&b.id], vec![&stranger]),
    ] {
        let seals: Vec<&str> = seals.iter().map(|s| s.as_str()).collect();
        let revoke: Vec<&str> = revoke.iter().map(|s| s.as_str()).collect();
        let r = rotate(
            &app,
            &owner,
            &v,
            &Rot {
                generation: 2,
                seals: &seals,
                invite_seals: &[],
                revoke: &revoke,
            },
        )
        .await;
        assert_eq!(code(&r), "members_mismatch", "{seals:?} {revoke:?}");
    }
    // An open invite must be refreshed too.
    let d = acct(&app, "dave").await;
    let (inv, r) = invite(&app, &owner, &v, &d.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id, &c.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(code(&r), "members_mismatch");
    assert_eq!(r.error().detail.unwrap()["invites"][0], inv);
    // Nothing changed yet.
    assert_eq!(
        sql(&app, |c| c.query_row(
            "SELECT current_generation FROM vaults",
            [],
            |r| r.get::<_, i64>(0)
        )),
        1
    );

    // The full request: shares for the members, a refreshed invite, and the
    // roster of the new generation, in one step.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id, &c.id],
            invite_seals: &[&inv],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let seq = r.json::<Value>()["seq"].as_u64().unwrap();
    assert_eq!(head(&app, &owner, &v).await, seq);
    // A member sees the share and the new roster in the sharing-aware page,
    // and no wrapped key of generation 2 until they claim.
    let page: Value = app
        .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&b.t))
        .await
        .json();
    assert_eq!(page["generation"], 2);
    assert_eq!(page["shares"].as_array().unwrap().len(), 1);
    assert_eq!(page["shares"][0]["generation"], 2);
    assert_eq!(page["shares"][0]["sender"], owner.id);
    assert_eq!(page["rosters"][0]["generation"], 2);
    assert_eq!(page["keys"].as_array().unwrap().len(), 1);
    // Without the flag the page is the M6 page.
    let plain: Value = app
        .get(&vault_url(&v, "/changes?since=0"), Some(&b.t))
        .await
        .json();
    assert!(plain.get("shares").is_none() && plain.get("role").is_none());
    // The invite now carries generation 2, so the invitee can accept it.
    assert_eq!(accept(&app, &d, &inv).await.status, StatusCode::OK);
    assert_eq!(
        sql(&app, |c| c.query_row(
            "SELECT generation FROM vault_keys WHERE account_id = ?1 ORDER BY generation DESC",
            [d.u.account_id.as_bytes().as_slice()],
            |r| r.get::<_, i64>(0)
        )),
        2
    );

    // Revocation: C is removed in the same step as the rotation.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 3,
            seals: &[&b.id, &d.id],
            invite_seals: &[],
            revoke: &[&c.id],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!(
        app.get(&vault_url(&v, "/changes?since=0"), Some(&c.t))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        sql(&app, |db| db.query_row(
            "SELECT COUNT(*) FROM key_shares WHERE recipient = ?1",
            [c.u.account_id.as_bytes().as_slice()],
            |r| r.get::<_, i64>(0)
        )),
        0
    );
    // A stale second try of the same generation.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 3,
            seals: &[&b.id, &d.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(code(&r), "generation_taken");
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 9,
            seals: &[&b.id, &d.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(code(&r), "generation_mismatch");
}

#[tokio::test(start_paused = true)]
async fn rotate_checks_head_and_device_and_roster() {
    let app = TestApp::new(RegistrationMode::Open);
    let (owner, b, c, v) = trio(&app).await;
    let body = |head: u64, device: String, roster: String| {
        json!({"generation": 2, "wrapped": b64(wrapped_key(5).as_bytes()), "device": device,
               "expectedHead": head, "roster": roster,
               "seals": [{"recipient": b.id, "seal": seal(3)}, {"recipient": c.id, "seal": seal(3)}],
               "inviteSeals": [], "revoke": []})
    };
    let h = head(&app, &owner, &v).await;
    let url = vault_url(&v, "/rotate");
    // The device must be the caller's.
    let r = app
        .post(
            &url,
            Some(&owner.t),
            &body(h, device(&app, &b), roster(&v, 2)),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    // The head must match.
    let r = app
        .post(
            &url,
            Some(&owner.t),
            &body(h + 1, device(&app, &owner), roster(&v, 2)),
        )
        .await;
    assert_eq!(code(&r), "head_moved");
    // The roster must be of the new generation.
    let r = app
        .post(
            &url,
            Some(&owner.t),
            &body(h, device(&app, &owner), roster(&v, 1)),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    // A seal of the wrong shape.
    let mut bad = body(h, device(&app, &owner), roster(&v, 2));
    bad["seals"][0]["seal"] = json!(b64(&[0; 82]));
    assert_eq!(
        app.post(&url, Some(&owner.t), &bad).await.status,
        StatusCode::BAD_REQUEST
    );
    let ok = app
        .post(
            &url,
            Some(&owner.t),
            &body(h, device(&app, &owner), roster(&v, 2)),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK, "{}", text(&ok));
}

#[tokio::test(start_paused = true)]
async fn owner_cannot_be_revoked_or_leave() {
    let app = TestApp::new(RegistrationMode::Open);
    let (owner, b, c, v) = trio(&app).await;
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id, &c.id],
            invite_seals: &[],
            revoke: &[&owner.id],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", text(&r));
    assert_eq!(code(&r), "role_forbidden");
    let r = app
        .post(&vault_url(&v, "/leave"), Some(&owner.t), &json!({}))
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "owner_cannot_leave");
    // The owner is a member of her vault still.
    assert_eq!(
        app.get(&vault_url(&v, "/members"), Some(&owner.t))
            .await
            .status,
        StatusCode::OK
    );
    // A member leaves; the vault and its key shares forget them.
    let r = app
        .post(&vault_url(&v, "/leave"), Some(&c.t), &json!({}))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(
        app.get(&vault_url(&v, "/changes?since=0"), Some(&c.t))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    // The owner's rotation now seals only to B; naming the leaver is a mismatch.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id, &c.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(code(&r), "members_mismatch");
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
}

#[tokio::test(start_paused = true)]
async fn plain_key_put_refused_for_shared_vault() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let v = vault_with(&app, &owner).await;
    let put_key = |g: u32| {
        let app = &app;
        let (t, url, dev) = (
            owner.t,
            vault_url(&v, &format!("/keys/{g}")),
            device(app, &owner),
        );
        async move {
            app.put(
                &url,
                Some(&t),
                &json!({"wrappedKey": b64(wrapped_key(5).as_bytes()), "deviceId": dev,
                        "expectedHeadSeq": null}),
            )
            .await
        }
    };
    // An open invite alone already makes the vault shared.
    let (inv, r) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let r = put_key(2).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "members_mismatch");
    // Cancelled, the vault is unshared again and the M6 path works.
    let r = app
        .delete(&format!("/v1/invites/{inv}"), Some(&owner.t))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(put_key(2).await.status, StatusCode::OK);
    // With a member it is refused for good.
    let (inv, _) = invite(&app, &owner, &v, &b.id, 3, 2).await;
    assert_eq!(accept(&app, &b, &inv).await.status, StatusCode::OK);
    let r = put_key(3).await;
    assert_eq!(code(&r), "members_mismatch");
}

#[tokio::test(start_paused = true)]
async fn claim_requires_share() {
    let app = TestApp::new(RegistrationMode::Open);
    let (owner, b, c, v) = trio(&app).await;
    let claim = |a: &Acct, g: u32| {
        let app = &app;
        let (t, url, dev) = (
            a.t,
            vault_url(&v, &format!("/keys/{g}/claim")),
            device(app, a),
        );
        async move {
            app.put(
                &url,
                Some(&t),
                &json!({"wrapped": b64(wrapped_key(6).as_bytes()), "device": dev}),
            )
            .await
        }
    };
    // Nothing to claim before a rotation.
    let r = claim(&b, 1).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(code(&r), "no_share");
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id, &c.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    // Wrong generation, a non-member, and another device's account.
    assert_eq!(code(&claim(&b, 1).await), "generation_mismatch");
    let stranger = acct(&app, "stranger").await;
    assert_eq!(claim(&stranger, 2).await.status, StatusCode::FORBIDDEN);
    // The owner holds the key already and was never sent a share.
    assert_eq!(code(&claim(&owner, 2).await), "no_share");
    // B claims; the share is gone, a second claim is a conflict, and the
    // account's key row appears in the feed.
    let r = claim(&b, 2).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let seq = r.json::<Value>()["seq"].as_u64().unwrap();
    assert_eq!(head(&app, &b, &v).await, seq);
    let page: Value = app
        .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&b.t))
        .await
        .json();
    assert_eq!(page["shares"].as_array().unwrap().len(), 0);
    assert_eq!(page["keys"].as_array().unwrap().len(), 2);
    assert_eq!(code(&claim(&b, 2).await), "no_share");
    // C's share is untouched by B's claim.
    let page: Value = app
        .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&c.t))
        .await
        .json();
    assert_eq!(page["shares"].as_array().unwrap().len(), 1);
    // A device that is not the session's is refused.
    let r = app
        .put(
            &vault_url(&v, "/keys/2/claim"),
            Some(&c.t),
            &json!({"wrapped": b64(wrapped_key(6).as_bytes()), "device": device(&app, &b)}),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
}

// ---- invites ----------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn invite_unique_pending() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let v = vault_with(&app, &owner).await;
    let (first, r) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    assert!(r.json::<Value>()["expiresAt"].as_u64().unwrap() > T0);
    let (_, r) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "invite_exists");
    // A fake recipient behaves the same.
    let ghost = uuid();
    assert_eq!(
        invite(&app, &owner, &v, &ghost, 3, 1).await.1.status,
        StatusCode::CREATED
    );
    assert_eq!(
        code(&invite(&app, &owner, &v, &ghost, 3, 1).await.1),
        "invite_exists"
    );
    // The same invite ID again.
    let r = app
        .post(
            &vault_url(&v, "/invites"),
            Some(&owner.t),
            &json!({"inviteId": first, "recipient": uuid(), "role": 3, "generation": 1,
                    "seal": seal(7)}),
        )
        .await;
    assert_eq!(code(&r), "invite_exists");
    // Wrong generation, a bad seal, an invite to oneself.
    assert_eq!(
        code(&invite(&app, &owner, &v, &uuid(), 3, 5).await.1),
        "generation_mismatch"
    );
    let r = app
        .post(
            &vault_url(&v, "/invites"),
            Some(&owner.t),
            &json!({"inviteId": uuid(), "recipient": uuid(), "role": 3, "generation": 1,
                    "seal": b64(&[0; 82])}),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        invite(&app, &owner, &v, &owner.id, 3, 1).await.1.status,
        StatusCode::BAD_REQUEST
    );
    // Only the recipient sees it, and sees who sent it.
    let inbox: Value = app.get("/v1/invites", Some(&b.t)).await.json();
    let incoming = inbox["incoming"].as_array().unwrap();
    assert_eq!(incoming.len(), 1);
    assert_eq!(incoming[0]["senderEmail"], owner.u.email);
    assert_eq!(incoming[0]["sender"], owner.id);
    // The sender's outbox shows it delivered after the listing.
    let out: Value = app.get("/v1/invites", Some(&owner.t)).await.json();
    let entry = out["outgoing"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["inviteId"] == first)
        .unwrap();
    assert_eq!(entry["delivered"], true);
    let ghost_entry = out["outgoing"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["recipient"] == ghost)
        .unwrap();
    assert_eq!(ghost_entry["delivered"], false);
    // Someone else can't read the preview or accept.
    let stranger = acct(&app, "stranger").await;
    assert_eq!(
        app.get(&format!("/v1/invites/{first}/vault"), Some(&stranger.t))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        accept(&app, &stranger, &first).await.status,
        StatusCode::NOT_FOUND
    );
    // The invitee reads the vault's metadata and roster (none yet) read-only.
    let pv: Value = app
        .get(&format!("/v1/invites/{first}/vault"), Some(&b.t))
        .await
        .json();
    assert!(pv["meta"].as_str().unwrap().len() > 10);
    assert!(pv["roster"].is_null());
    // Expiry: after 14 days the invite is gone and a new one is possible.
    app.clock.advance_ms(14 * 24 * HOUR + 1000);
    let t = app.login_device(&b.u, b.u.device).await;
    assert_eq!(
        app.get(&format!("/v1/invites/{first}/vault"), Some(&t))
            .await
            .status,
        StatusCode::GONE
    );
    let owner_t = app.login_device(&owner.u, owner.u.device).await;
    let owner2 = Acct {
        u: User::new("x@example.com"),
        t: owner_t,
        id: owner.id.clone(),
    };
    let inbox: Value = app.get("/v1/invites", Some(&t)).await.json();
    assert_eq!(inbox["incoming"].as_array().unwrap().len(), 0);
    let (_, r) = invite(&app, &owner2, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
}

#[tokio::test(start_paused = true)]
async fn invite_stale_after_rotation() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let v = vault_with(&app, &owner).await;
    let (inv, r) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    // The rotation refreshes the open invite's seal and generation.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[],
            invite_seals: &[&inv],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let inbox: Value = app.get("/v1/invites", Some(&b.t)).await.json();
    assert_eq!(inbox["incoming"][0]["generation"], 2);
    // An invite that was not refreshed (an old seal) is stale, not accepted.
    sql(&app, |c| {
        c.execute("UPDATE share_invites SET generation = 1", [])
    });
    let r = accept(&app, &b, &inv).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "invite_stale");
    assert_eq!(r.error().detail.unwrap()["currentGeneration"], 2);
    assert_eq!(
        sql(&app, |c| c.query_row(
            "SELECT COUNT(*) FROM memberships",
            [],
            |r| r.get::<_, i64>(0)
        )),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn decline_cancel_and_block() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let v = vault_with(&app, &owner).await;
    // Decline: the sender's outbox keeps it (declined, delivered) so that a
    // delivered seal can be revoked.
    let (inv, _) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    app.get("/v1/invites", Some(&b.t)).await;
    let r = app
        .post(
            &format!("/v1/invites/{inv}/decline"),
            Some(&b.t),
            &json!({"block": false}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let out: Value = app.get("/v1/invites", Some(&owner.t)).await.json();
    assert_eq!(out["outgoing"][0]["state"], 2);
    assert_eq!(out["outgoing"][0]["delivered"], true);
    // A second decline finds nothing; the vault is unshared again.
    let r = app
        .post(
            &format!("/v1/invites/{inv}/decline"),
            Some(&b.t),
            &json!({"block": false}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    // Deleting a finished invite removes it from the list.
    let r = app
        .delete(&format!("/v1/invites/{inv}"), Some(&owner.t))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let out: Value = app.get("/v1/invites", Some(&owner.t)).await.json();
    assert_eq!(out["outgoing"].as_array().unwrap().len(), 0);
    // Only the sender or a manager cancels.
    let (inv, _) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(
        app.delete(&format!("/v1/invites/{inv}"), Some(&b.t))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.delete(&format!("/v1/invites/{inv}"), Some(&owner.t))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(accept(&app, &b, &inv).await.status, StatusCode::NOT_FOUND);
}

#[tokio::test(start_paused = true)]
async fn blocked_sender_invite_vanishes() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let v = vault_with(&app, &owner).await;
    let (inv, _) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    // Decline and block.
    let r = app
        .post(
            &format!("/v1/invites/{inv}/decline"),
            Some(&b.t),
            &json!({"block": true}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    // The next invite answers 201 like any other, shows up nowhere for B,
    // and a repeat answers exactly as it would for an unblocked recipient.
    let (inv2, r) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let inbox: Value = app.get("/v1/invites", Some(&b.t)).await.json();
    assert_eq!(inbox["incoming"].as_array().unwrap().len(), 0);
    assert_eq!(accept(&app, &b, &inv2).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        app.get(&format!("/v1/invites/{inv2}/vault"), Some(&b.t))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        code(&invite(&app, &owner, &v, &b.id, 3, 1).await.1),
        "invite_exists"
    );
    // B unblocks; after the dropped invite is cancelled, a new one arrives.
    let r = app
        .delete(&format!("/v1/blocks/{}", owner.id), Some(&b.t))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    app.delete(&format!("/v1/invites/{inv2}"), Some(&owner.t))
        .await;
    app.delete(&format!("/v1/invites/{inv2}"), Some(&owner.t))
        .await;
    let (inv3, r) = invite(&app, &owner, &v, &b.id, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let inbox: Value = app.get("/v1/invites", Some(&b.t)).await.json();
    assert_eq!(inbox["incoming"][0]["inviteId"], inv3);
    // Blocking an ID that does not exist is fine too (no oracle).
    assert_eq!(
        app.post(&format!("/v1/blocks/{}", uuid()), Some(&b.t), &json!({}))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test(start_paused = true)]
async fn invite_caps() {
    let app = TestApp::new(RegistrationMode::Open);
    let owner = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let v = vault_with(&app, &owner).await;
    let v3 = vault_with(&app, &owner).await;
    // 49 open invites and the owner fill a vault of 50.
    for i in 0..49 {
        let (_, r) = invite(&app, &owner, &v, &uuid(), 4, 1).await;
        assert_eq!(r.status, StatusCode::CREATED, "invite {i}");
        app.clock.advance_ms(HOUR / 40);
    }
    let (_, r) = invite(&app, &owner, &v, &uuid(), 4, 1).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "members_limit");

    // 200 open invites per sender (planted, each in a vault of its own: the
    // rate limit alone would take hours).
    let owner_id = owner.u.account_id.as_bytes().to_vec();
    sql(&app, |c| {
        for _ in 0..200 {
            c.execute(
                "INSERT INTO vaults (vault_id, owner_account_id, current_generation, head_seq, created_at)                  VALUES (randomblob(16), ?1, 1, 2, 0)",
                [&owner_id],
            )?;
        }
        c.execute(
            "INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, generation, seal,              created_at, expires_at, state, hidden)              SELECT randomblob(16), vault_id, ?1, randomblob(16), 4, 1, zeroblob(82), 0, 9000000000000000, 0, 0              FROM vaults WHERE vault_id NOT IN (?2, ?3)",
            rusqlite::params![owner_id, v.id.as_bytes().as_slice(), v3.id.as_bytes().as_slice()],
        )?;
        Ok(())
    });
    app.clock.advance_ms(HOUR);
    let (_, r) = invite(&app, &owner, &v3, &uuid(), 4, 1).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "invite_limit");

    // 100 open invites per recipient: more are stored but never shown.
    sql(&app, |c| {
        c.execute("DELETE FROM share_invites", [])?;
        c.execute(
            "INSERT INTO share_invites (invite_id, vault_id, sender, recipient, role, generation, seal,              created_at, expires_at, state, hidden)              SELECT randomblob(16), vault_id, ?1, ?2, 4, 1, zeroblob(82), 0, 9000000000000000, 0, 0              FROM vaults WHERE vault_id NOT IN (?3, ?4) LIMIT 100",
            rusqlite::params![
                owner_id,
                b.u.account_id.as_bytes().as_slice(),
                v.id.as_bytes().as_slice(),
                v3.id.as_bytes().as_slice()
            ],
        )?;
        Ok(())
    });
    app.clock.advance_ms(HOUR);
    let (extra, r) = invite(&app, &owner, &v3, &b.id, 4, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let inbox: Value = app.get("/v1/invites", Some(&b.t)).await.json();
    let incoming = inbox["incoming"].as_array().unwrap();
    assert_eq!(incoming.len(), 100);
    assert!(incoming.iter().all(|i| i["inviteId"] != extra));
    assert_eq!(accept(&app, &b, &extra).await.status, StatusCode::NOT_FOUND);
    // The sender sees it as an open, undelivered invite, like a fake one.
    let out: Value = app.get("/v1/invites", Some(&owner.t)).await.json();
    let mine = out["outgoing"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["inviteId"] == extra)
        .unwrap();
    assert_eq!(
        (mine["state"].as_u64(), mine["delivered"].as_bool()),
        (Some(0), Some(false))
    );
}

// ---- re-invite -----------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn reinvite_is_blocked_until_the_next_rotation() {
    let app = TestApp::new(RegistrationMode::Open);
    let (owner, b, c, v) = trio(&app).await;
    // Revoking C rotates to generation 2; C can't be invited again at 2.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 2,
            seals: &[&b.id],
            invite_seals: &[],
            revoke: &[&c.id],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let (_, r) = invite(&app, &owner, &v, &c.id, 4, 2).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "reinvite_blocked");
    assert_eq!(r.error().detail.unwrap()["currentGeneration"], 2);
    // Neither at the old generation.
    assert_eq!(
        code(&invite(&app, &owner, &v, &c.id, 4, 1).await.1),
        "generation_mismatch"
    );
    // After the next rotation the account may be invited again.
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 3,
            seals: &[&b.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    let (inv, r) = invite(&app, &owner, &v, &c.id, 4, 3).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    assert_eq!(accept(&app, &c, &inv).await.status, StatusCode::OK);

    // A leave blocks the same way: the revocation rotation comes first and
    // the account stays out until the one after it.
    let r = app
        .post(&vault_url(&v, "/leave"), Some(&b.t), &json!({}))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    for generation in [3u32, 4] {
        assert_eq!(
            code(&invite(&app, &owner, &v, &b.id, 4, generation).await.1),
            if generation == 3 {
                "reinvite_blocked"
            } else {
                "generation_mismatch"
            },
        );
    }
    let r = rotate(
        &app,
        &owner,
        &v,
        &Rot {
            generation: 4,
            seals: &[&c.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!(
        code(&invite(&app, &owner, &v, &b.id, 4, 4).await.1),
        "reinvite_blocked"
    );
}

// ---- roster ------------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn roster_is_a_compare_and_swap_per_generation() {
    let app = TestApp::new(RegistrationMode::Open);
    let (owner, b, _c, v) = trio(&app).await;
    let put = |a: &Acct, generation: u32, ct: String, base: Option<u64>| {
        let app = &app;
        let (t, url) = (a.t, vault_url(&v, "/roster"));
        async move {
            app.put(
                &url,
                Some(&t),
                &json!({"generation": generation, "ciphertext": ct, "baseSeq": base}),
            )
            .await
        }
    };
    let r = put(&owner, 1, roster(&v, 1), None).await;
    assert_eq!(r.status, StatusCode::OK);
    let seq = r.json::<Value>()["seq"].as_u64().unwrap();
    // A stale base loses and learns the stored sequence.
    let r = put(&b, 1, roster(&v, 1), None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "conflict");
    assert_eq!(r.error().detail.unwrap()["currentSeq"], seq);
    let r = put(&b, 1, roster(&v, 1), Some(seq)).await;
    assert_eq!(r.status, StatusCode::OK);
    // Only the current generation; the ciphertext header must agree.
    let r = put(&owner, 2, roster(&v, 2), None).await;
    assert_eq!(code(&r), "generation_mismatch");
    let r = put(&owner, 1, roster(&v, 2), Some(seq + 1)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    // The roster is served with the page and to a pending invitee.
    let page: Value = app
        .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&b.t))
        .await
        .json();
    assert_eq!(page["rosters"].as_array().unwrap().len(), 1);
    let newer: Value = app
        .get(
            &vault_url(
                &v,
                &format!("/changes?since={}&sharing=1", head(&app, &b, &v).await),
            ),
            Some(&b.t),
        )
        .await
        .json();
    assert_eq!(newer["rosters"].as_array().unwrap().len(), 0);
    let d = acct(&app, "dave").await;
    let (inv, _) = invite(&app, &owner, &v, &d.id, 4, 1).await;
    let pv: Value = app
        .get(&format!("/v1/invites/{inv}/vault"), Some(&d.t))
        .await
        .json();
    assert!(pv["roster"].is_string());
}

// ---- logs and storage ------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn lookup_targets_and_sharing_data_are_not_logged_or_stored_in_clear() {
    let logs = crate::common::init_logs();
    let app = TestApp::with_file_db(RegistrationMode::Open);
    let a = acct(&app, "alice").await;
    let target = "lookup-target-5c2e7b@marker-lookup-3f9a.example";
    let r = lookup(&app, &a.t, target).await;
    assert_eq!(r.status, StatusCode::OK);
    let ghost = AccountId::new_random().unwrap().to_string();
    let v = vault_with(&app, &a).await;
    let (inv, r) = invite(&app, &a, &v, &ghost, 3, 1).await;
    assert_eq!(r.status, StatusCode::CREATED);
    publish(&app, &a, 11).await;
    app.state
        .db()
        .run(
            |c| {
                Ok::<_, ApiError>(c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?)
            },
        )
        .unwrap();

    let log = logs.contents();
    let text = String::from_utf8_lossy(&log).into_owned();
    assert!(text.contains("/v1/directory/lookup"), "the route is logged");
    for s in [
        "marker-lookup-3f9a",
        "lookup-target-5c2e7b",
        ghost.as_str(),
        inv.as_str(),
        a.id.as_str(),
    ] {
        assert!(!text.contains(s), "the log contains {s}");
    }
    // The lookup target is nowhere on disk (it is not registered), and the
    // wrapped identity key is the opaque blob the client sent.
    for (name, bytes) in crate::no_plaintext::stored_bytes(app.dir.path()) {
        assert!(
            !crate::no_plaintext::contains(&bytes, b"marker-lookup-3f9a"),
            "{name} holds the lookup target"
        );
    }
}
