//! The atomic account re-key (M7 spec §4, §7.3): `/v1/account/rekey/start`
//! and `finish`, their coverage rules, the seals and invites they drop, the
//! sessions they end, and what stays out of the log and the disk.

use axum::http::StatusCode;
use mosaic_core::Argon2Profile;
use mosaic_server::ApiError;
use mosaic_sync::wire::json::{RegistrationMode, RegistrationStartResponse};
use mosaic_sync::wire::opaque::{
    OsRng, Suite, identifiers,
    ke::{ClientRegistration, ClientRegistrationFinishParameters, RegistrationResponse},
};
use serde_json::{Value, json};

use crate::common::{IP, LoginResult, ORIGIN, Resp, TestApp, TestVault, User, wrapped_key};
use crate::sharing::{
    Acct, Rot, acct, b64, device, head, invite, join, publish, roster, rotate, seal, text,
    vault_url, vault_with,
};

const NEW_OPW: [u8; 32] = [0x42; 32];

fn sql<T>(app: &TestApp, f: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T>) -> T {
    app.state
        .db()
        .run(|c| f(c).map_err(ApiError::from))
        .unwrap()
}

/// A count query; `?1` and `?2` are bound to `p`.
fn n(app: &TestApp, query: &str, p: &[&str]) -> i64 {
    sql(app, |c| {
        c.query_row(query, rusqlite::params_from_iter(p.iter()), |r| r.get(0))
    })
}

fn epoch_of(app: &TestApp, a: &Acct) -> i64 {
    n(
        app,
        "SELECT ak_epoch FROM accounts WHERE lower(hex(account_id)) = replace(?1, '-', '')",
        &[&a.id],
    )
}

fn code(r: &Resp) -> String {
    r.error().error
}

/// A new session of the same account (a fresh OPAQUE login).
async fn relogin(app: &TestApp, a: &Acct) -> Acct {
    app.clock.advance_ms(13_000);
    let t = app.login_device(&a.u, a.u.device).await;
    Acct {
        u: User {
            email: a.u.email.clone(),
            opw: a.u.opw,
            account_id: a.u.account_id,
            device: a.u.device,
            salt: a.u.salt,
        },
        t,
        id: a.id.clone(),
    }
}

/// `rekey/start` and the client half of the OPAQUE registration of `NEW_OPW`:
/// the upload for `finish`.
async fn upload(app: &TestApp, a: &Acct, epoch: u32) -> String {
    let mut rng = OsRng;
    let start = ClientRegistration::<Suite>::start(&mut rng, &NEW_OPW).unwrap();
    let r = app
        .post(
            "/v1/account/rekey/start",
            Some(&a.t),
            &json!({"registrationRequest": b64(&start.message.serialize()), "akEpoch": epoch}),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let resp: RegistrationStartResponse = r.json();
    let fin = start
        .state
        .finish(
            &mut rng,
            &NEW_OPW,
            RegistrationResponse::deserialize(&resp.registration_response.0).unwrap(),
            ClientRegistrationFinishParameters::new(
                identifiers(a.u.account_id.as_bytes(), ORIGIN.as_bytes()),
                None,
            ),
        )
        .unwrap();
    b64(&fin.message.serialize())
}

/// The parts of a `finish` body.
struct Rk {
    epoch: u32,
    profile: u8,
    identity: Value,
    vaults: Vec<Value>,
    rewraps: Vec<Value>,
    emergency: Vec<Value>,
}

impl Rk {
    fn new(epoch: u32) -> Self {
        Self {
            epoch,
            profile: Argon2Profile::DEFAULT.id(),
            identity: Value::Null,
            vaults: Vec::new(),
            rewraps: Vec::new(),
            emergency: Vec::new(),
        }
    }

    fn vaults(mut self, v: Vec<Value>) -> Self {
        self.vaults = v;
        self
    }

    fn json(&self, app: &TestApp, a: &Acct, upload: &str) -> Value {
        json!({
            "registrationUpload": upload,
            "wrappedAk": b64(wrapped_key(11).as_bytes()),
            "salt": b64(&[6u8; 16]),
            "profile": self.profile,
            "akEpoch": self.epoch,
            "identity": self.identity,
            "vaults": self.vaults,
            "memberRewraps": self.rewraps,
            "emergencySeals": self.emergency,
            "device": device(app, a),
        })
    }

    async fn send(&self, app: &TestApp, a: &Acct, upload: &str) -> Resp {
        app.post(
            "/v1/account/rekey/finish",
            Some(&a.t),
            &self.json(app, a, upload),
        )
        .await
    }
}

/// An owned vault of the request, unshared, at `g` with the expected head.
fn ve(v: &TestVault, g: u32, head: u64) -> Value {
    json!({"vaultId": v.id.to_string(), "generation": g,
           "wrapped": b64(wrapped_key(11).as_bytes()), "expectedHead": head})
}

/// The same for a shared vault: roster, a seal per member, a seal per invite.
fn ve_shared(v: &TestVault, g: u32, head: u64, members: &[&str], invites: &[&str]) -> Value {
    let mut e = ve(v, g, head);
    e["roster"] = json!(roster(v, g));
    e["seals"] = json!(
        members
            .iter()
            .map(|m| json!({"recipient": m, "seal": seal(3)}))
            .collect::<Vec<_>>()
    );
    e["inviteSeals"] = json!(
        invites
            .iter()
            .map(|i| json!({"inviteId": i, "seal": seal(4)}))
            .collect::<Vec<_>>()
    );
    e
}

fn rewrap(v: &TestVault, g: u32) -> Value {
    json!({"vaultId": v.id.to_string(), "generation": g,
           "wrapped": b64(wrapped_key(12).as_bytes())})
}

fn vault_gen(app: &TestApp, v: &TestVault) -> i64 {
    n(
        app,
        "SELECT current_generation FROM vaults WHERE lower(hex(vault_id)) = replace(?1, '-', '')",
        &[&v.id.to_string()],
    )
}

fn keys_of(app: &TestApp, v: &TestVault, a: &Acct, generation: &str) -> i64 {
    n(
        app,
        "SELECT COUNT(*) FROM vault_keys WHERE lower(hex(vault_id)) = replace(?1, '-', '') \
         AND lower(hex(account_id)) = replace(?2, '-', '') AND generation = CAST(?3 AS INTEGER)",
        &[&v.id.to_string(), &a.id, generation],
    )
}

fn inbox_seq(app: &TestApp, a: &Acct) -> i64 {
    n(
        app,
        "SELECT inbox_seq FROM accounts WHERE lower(hex(account_id)) = replace(?1, '-', '')",
        &[&a.id],
    )
}

fn open() -> TestApp {
    TestApp::new(RegistrationMode::Open)
}

// ---- freshness, version, epoch ------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rekey_requires_fresh_login() {
    let app = open();
    let a = acct(&app, "alice").await;
    let stale_body = Rk::new(2).json(&app, &a, &b64(&[1u8; 64]));
    app.clock.advance_ms(301_000);
    let r = app
        .post(
            "/v1/account/rekey/start",
            Some(&a.t),
            &json!({"registrationRequest": b64(&[1u8; 32]), "akEpoch": 2}),
        )
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", text(&r));
    assert_eq!(code(&r), "fresh_login_required");
    let r = app
        .post("/v1/account/rekey/finish", Some(&a.t), &stale_body)
        .await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", text(&r));
    assert_eq!(code(&r), "fresh_login_required");
    assert_eq!(epoch_of(&app, &a), 1);

    // A fresh login passes.
    let fresh = relogin(&app, &a).await;
    upload(&app, &fresh, 2).await;
}

#[tokio::test(start_paused = true)]
async fn rekey_enforces_version_at_login() {
    let app = open();
    let a = acct(&app, "alice").await;
    let up = upload(&app, &a, 2).await;
    // Another change moved the account under this session.
    sql(&app, |c| {
        c.execute(
            "UPDATE accounts SET account_version = account_version + 1",
            [],
        )
    });
    let r = Rk::new(2).send(&app, &a, &up).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", text(&r));
    assert_eq!(code(&r), "account_version_mismatch");
    assert_eq!(r.error().detail.unwrap()["accountVersion"], 2);
    let r = app
        .post(
            "/v1/account/rekey/start",
            Some(&a.t),
            &json!({"registrationRequest": b64(&[1u8; 32]), "akEpoch": 2}),
        )
        .await;
    assert_eq!(code(&r), "account_version_mismatch");
    assert_eq!(epoch_of(&app, &a), 1);
}

#[tokio::test(start_paused = true)]
async fn rekey_epoch_cas() {
    let app = open();
    let a = acct(&app, "alice").await;
    for wrong in [0u32, 1, 3, u32::MAX] {
        let r = app
            .post(
                "/v1/account/rekey/start",
                Some(&a.t),
                &json!({"registrationRequest": b64(&[1u8; 32]), "akEpoch": wrong}),
            )
            .await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{wrong}: {}", text(&r));
        assert_eq!(code(&r), "epoch_mismatch");
        assert_eq!(r.error().detail.unwrap()["currentEpoch"], 1);
    }
    let up = upload(&app, &a, 2).await;
    let r = Rk::new(3).send(&app, &a, &up).await;
    assert_eq!(code(&r), "epoch_mismatch");
    assert_eq!(epoch_of(&app, &a), 1, "nothing applied");

    let r = Rk::new(2).send(&app, &a, &up).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let done: Value = r.json();
    assert_eq!(
        (&done["accountVersion"], &done["akEpoch"]),
        (&json!(2), &json!(2))
    );
    assert_eq!(epoch_of(&app, &a), 2);

    // The account record serves the epoch to a client that asks for it
    // (`epoch=1`); the M6 and M6b bodies stay as they were.
    let m6b: Value = app.get("/v1/account?identity=1", Some(&a.t)).await.json();
    assert!(m6b.get("akEpoch").is_none() && m6b.get("identity").is_some());
    let acc: Value = app
        .get("/v1/account?identity=1&epoch=1", Some(&a.t))
        .await
        .json();
    assert_eq!(acc["akEpoch"], 2);
    let plain: Value = app.get("/v1/account", Some(&a.t)).await.json();
    assert!(plain.get("akEpoch").is_none());

    // The same epoch again is refused (the session moved to version 2).
    let r = app
        .post(
            "/v1/account/rekey/start",
            Some(&a.t),
            &json!({"registrationRequest": b64(&[1u8; 32]), "akEpoch": 2}),
        )
        .await;
    assert_eq!(code(&r), "epoch_mismatch");
    assert_eq!(r.error().detail.unwrap()["currentEpoch"], 2);

    // The database itself never lets the epoch fall or jump.
    let bad = |q: &str| {
        app.state
            .db()
            .run(|c| Ok(c.execute(q, []).is_err()))
            .unwrap()
    };
    assert!(bad("UPDATE accounts SET ak_epoch = 1"));
    assert!(bad("UPDATE accounts SET ak_epoch = 4"));
    assert!(!bad("UPDATE accounts SET ak_epoch = 3"));
}

#[tokio::test(start_paused = true)]
async fn profile_downgrade_refused() {
    let app = open();
    let a = acct(&app, "alice").await;
    sql(&app, |c| c.execute("UPDATE accounts SET profile = 3", []));
    let up = upload(&app, &a, 2).await;
    let r = Rk::new(2).send(&app, &a, &up).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", text(&r));
    assert_eq!(code(&r), "profile_downgrade");
    assert_eq!(epoch_of(&app, &a), 1);
    // The same or a stronger profile passes.
    let mut rk = Rk::new(2);
    rk.profile = 4;
    let r = rk.send(&app, &a, &up).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let acc: Value = app.get("/v1/account", Some(&a.t)).await.json();
    assert_eq!(acc["profile"], 4);
}

// ---- coverage ----------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rekey_must_cover_every_owned_vault() {
    let app = open();
    let a = acct(&app, "alice").await;
    let (v1, v2) = (vault_with(&app, &a).await, vault_with(&app, &a).await);
    let (h1, h2) = (head(&app, &a, &v1).await, head(&app, &a, &v2).await);
    let up = upload(&app, &a, 2).await;
    let stranger = TestVault::new();

    for (what, vaults, status) in [
        ("one missing", vec![ve(&v1, 2, h1)], StatusCode::CONFLICT),
        (
            "an unknown one",
            vec![ve(&v1, 2, h1), ve(&v2, 2, h2), ve(&stranger, 2, 2)],
            StatusCode::CONFLICT,
        ),
        ("none", vec![], StatusCode::CONFLICT),
        (
            "a duplicate",
            vec![ve(&v1, 2, h1), ve(&v1, 2, h1)],
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let r = Rk::new(2).vaults(vaults).send(&app, &a, &up).await;
        assert_eq!(r.status, status, "{what}: {}", text(&r));
        if status == StatusCode::CONFLICT {
            assert_eq!(code(&r), "rekey_mismatch", "{what}");
            assert_eq!(r.error().detail.unwrap()["set"], "vaults");
        }
    }
    // Nothing was applied.
    assert_eq!(
        (
            epoch_of(&app, &a),
            vault_gen(&app, &v1),
            vault_gen(&app, &v2)
        ),
        (1, 1, 1)
    );
    assert_eq!(keys_of(&app, &v1, &a, "1"), 1);

    let r = Rk::new(2)
        .vaults(vec![ve(&v1, 2, h1), ve(&v2, 2, h2)])
        .send(&app, &a, &up)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!((vault_gen(&app, &v1), vault_gen(&app, &v2)), (2, 2));
    // The new key row is there, the old wrapping is gone.
    assert_eq!(keys_of(&app, &v1, &a, "2"), 1);
    assert_eq!(keys_of(&app, &v1, &a, "1"), 0);
    // The head moved: the account's other devices fetch the new row.
    assert_eq!(head(&app, &a, &v1).await, h1 + 1);
}

#[tokio::test(start_paused = true)]
async fn rekey_head_moved_applies_nothing() {
    let app = open();
    let a = acct(&app, "alice").await;
    let v = vault_with(&app, &a).await;
    let h = head(&app, &a, &v).await;
    let up = upload(&app, &a, 2).await;

    let r = Rk::new(2)
        .vaults(vec![ve(&v, 2, h - 1)])
        .send(&app, &a, &up)
        .await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", text(&r));
    assert_eq!(code(&r), "head_moved");
    let d = r.error().detail.unwrap();
    assert_eq!(d["headSeq"], h);
    assert_eq!(d["vaultId"], v.id.to_string());

    // Taken: the generation is not above the current one.
    let r = Rk::new(2)
        .vaults(vec![ve(&v, 1, h)])
        .send(&app, &a, &up)
        .await;
    assert_eq!(code(&r), "generation_taken");
    // Skipping a generation.
    let r = Rk::new(2)
        .vaults(vec![ve(&v, 3, h)])
        .send(&app, &a, &up)
        .await;
    assert_eq!(code(&r), "generation_mismatch");

    assert_eq!((epoch_of(&app, &a), vault_gen(&app, &v)), (1, 1));
    assert_eq!(head(&app, &a, &v).await, h);
    // The account is untouched: the old password still logs in.
    app.clock.advance_ms(13_000);
    assert!(matches!(app.login(&a.u).await, LoginResult::Ok { .. }));
}

#[tokio::test(start_paused = true)]
async fn rekey_must_cover_member_vaults() {
    let app = open();
    let o = acct(&app, "owner").await;
    let m = acct(&app, "member").await;
    let v = vault_with(&app, &o).await;
    join(&app, &o, &v, &m, 4).await;
    let up = upload(&app, &m, 2).await;

    // Missing, wrong generation, unknown vault.
    for (what, rewraps) in [
        ("missing", vec![]),
        ("wrong generation", vec![rewrap(&v, 2)]),
        ("unknown", vec![rewrap(&v, 1), rewrap(&TestVault::new(), 1)]),
    ] {
        let mut rk = Rk::new(2);
        rk.rewraps = rewraps;
        let r = rk.send(&app, &m, &up).await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{what}: {}", text(&r));
        assert_eq!(code(&r), "rekey_mismatch", "{what}");
        assert_eq!(r.error().detail.unwrap()["set"], "memberRewraps");
    }
    assert_eq!(epoch_of(&app, &m), 1);

    let h = head(&app, &o, &v).await;
    let owner_seq = inbox_seq(&app, &o);
    let mut rk = Rk::new(2);
    rk.rewraps = vec![rewrap(&v, 1)];
    let r = rk.send(&app, &m, &up).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));

    // The member's row is the new wrapping, at a new head.
    let wrapped: Vec<u8> = sql(&app, |c| {
        c.query_row(
            "SELECT wrapped FROM vault_keys WHERE lower(hex(account_id)) = replace(?1, '-', '')",
            [&m.id],
            |r| r.get(0),
        )
    });
    assert_eq!(wrapped, wrapped_key(12).as_bytes());
    assert_eq!(head(&app, &o, &v).await, h + 1);
    // The owner is asked to rotate and woken; the log has the request.
    let page: Value = app
        .get(
            &vault_url(&v, "/changes?since=0&sharing=1&rotation=1"),
            Some(&o.t),
        )
        .await
        .json();
    assert_eq!(page["rotationRequested"], true);
    assert!(inbox_seq(&app, &o) > owner_seq);
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM audit WHERE kind = 16 AND lower(hex(account_id)) = replace(?1, '-', '')",
            &[&o.id]
        ),
        1
    );
    // Without `rotation=1` the page is the M6b page.
    let page: Value = app
        .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&o.t))
        .await
        .json();
    assert!(page.get("rotationRequested").is_none());
    // The member's own feed does not carry the flag.
    let page: Value = app
        .get(
            &vault_url(&v, "/changes?since=0&sharing=1&rotation=1"),
            Some(&m.t),
        )
        .await
        .json();
    assert!(page.get("rotationRequested").is_none());
}

#[tokio::test(start_paused = true)]
async fn rotation_request_reaches_owner() {
    let app = open();
    let o = acct(&app, "owner").await;
    let m = acct(&app, "member").await;
    let v = vault_with(&app, &o).await;
    join(&app, &o, &v, &m, 3).await;
    let up = upload(&app, &m, 2).await;
    let mut rk = Rk::new(2);
    rk.rewraps = vec![rewrap(&v, 1)];
    assert_eq!(rk.send(&app, &m, &up).await.status, StatusCode::OK);
    assert_eq!(
        n(&app, "SELECT COUNT(*) FROM vault_rotation_requests", &[]),
        1
    );

    // The request survives until the owner rotates; the rotation answers it.
    let r = rotate(
        &app,
        &o,
        &v,
        &Rot {
            generation: 2,
            seals: &[&m.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!(
        n(&app, "SELECT COUNT(*) FROM vault_rotation_requests", &[]),
        0
    );
    let page: Value = app
        .get(
            &vault_url(&v, "/changes?since=0&sharing=1&rotation=1"),
            Some(&o.t),
        )
        .await
        .json();
    assert_eq!(page["rotationRequested"], false);
}

#[tokio::test(start_paused = true)]
async fn rekey_shared_vault_needs_all_seals() {
    let app = open();
    let o = acct(&app, "owner").await;
    let b = acct(&app, "bob").await;
    let c = acct(&app, "carol").await;
    let shared = vault_with(&app, &o).await;
    let plain = vault_with(&app, &o).await;
    join(&app, &o, &shared, &b, 2).await;
    let (inv, r) = invite(&app, &o, &shared, &c.id, 4, 1).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    let (hs, hp) = (head(&app, &o, &shared).await, head(&app, &o, &plain).await);
    let up = upload(&app, &o, 2).await;

    let try_with = |sh: Value, pl: Value| {
        let up = up.clone();
        let (app, o) = (&app, &o);
        async move { Rk::new(2).vaults(vec![sh, pl]).send(app, o, &up).await }
    };
    // No seal for the member.
    let r = try_with(ve_shared(&shared, 2, hs, &[], &[&inv]), ve(&plain, 2, hp)).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", text(&r));
    assert_eq!(code(&r), "members_mismatch");
    assert_eq!(
        r.error().detail.unwrap()["members"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // No seal for the open invite.
    let r = try_with(ve_shared(&shared, 2, hs, &[&b.id], &[]), ve(&plain, 2, hp)).await;
    assert_eq!(code(&r), "members_mismatch");
    // A seal for a stranger.
    let r = try_with(
        ve_shared(&shared, 2, hs, &[&b.id, &c.id], &[&inv]),
        ve(&plain, 2, hp),
    )
    .await;
    assert_eq!(code(&r), "members_mismatch");
    // The roster is required for a shared vault.
    let mut no_roster = ve_shared(&shared, 2, hs, &[&b.id], &[&inv]);
    no_roster.as_object_mut().unwrap().remove("roster");
    let r = try_with(no_roster, ve(&plain, 2, hp)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", text(&r));
    // An unshared vault takes no seals.
    let r = try_with(
        ve_shared(&shared, 2, hs, &[&b.id], &[&inv]),
        ve_shared(&plain, 2, hp, &[&b.id], &[]),
    )
    .await;
    assert_eq!(code(&r), "members_mismatch");
    // A roster whose header names another generation.
    let mut wrong_roster = ve_shared(&shared, 2, hs, &[&b.id], &[&inv]);
    wrong_roster["roster"] = json!(roster(&shared, 3));
    let r = try_with(wrong_roster, ve(&plain, 2, hp)).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", text(&r));
    assert_eq!(
        (
            epoch_of(&app, &o),
            vault_gen(&app, &shared),
            vault_gen(&app, &plain)
        ),
        (1, 1, 1)
    );

    let r = try_with(
        ve_shared(&shared, 2, hs, &[&b.id], &[&inv]),
        ve(&plain, 2, hp),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!((vault_gen(&app, &shared), vault_gen(&app, &plain)), (2, 2));
    // Bob keeps his membership and gets a share at the new generation; the
    // invite carries the new seal; the roster of the new generation is there.
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM key_shares WHERE generation = 2 AND lower(hex(recipient)) = replace(?1, '-', '')",
            &[&b.id]
        ),
        1
    );
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM memberships WHERE lower(hex(account_id)) = replace(?1, '-', '')",
            &[&b.id]
        ),
        1
    );
    assert_eq!(
        n(
            &app,
            "SELECT generation FROM share_invites WHERE lower(hex(invite_id)) = replace(?1, '-', '')",
            &[&inv]
        ),
        2
    );
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM vault_roster WHERE key_generation = 2",
            &[]
        ),
        1
    );
    // The request revoked nobody.
    assert_eq!(n(&app, "SELECT COUNT(*) FROM vault_removals", &[]), 0);
}

// ---- the identity -------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn identity_replaced_only_by_rekey() {
    let app = open();
    let a = acct(&app, "alice").await;
    publish(&app, &a, 1).await;
    let new_pair = json!({"ipk": b64(&[2u8; 32]), "wrappedIsk": b64(wrapped_key(2).as_bytes())});

    // The plain route refuses a second pair.
    let r = app.put("/v1/account/identity", Some(&a.t), &new_pair).await;
    assert_eq!(r.status, StatusCode::CONFLICT, "{}", text(&r));
    assert_eq!(code(&r), "identity_exists");

    let up = upload(&app, &a, 2).await;
    // An account with an identity sends a different one.
    let r = Rk::new(2).send(&app, &a, &up).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST, "{}", text(&r));
    let mut same = Rk::new(2);
    same.identity = json!({"ipk": b64(&[1u8; 32]), "wrappedIsk": b64(wrapped_key(2).as_bytes())});
    assert_eq!(
        same.send(&app, &a, &up).await.status,
        StatusCode::BAD_REQUEST
    );
    let mut zero = Rk::new(2);
    zero.identity = json!({"ipk": b64(&[0u8; 32]), "wrappedIsk": b64(wrapped_key(2).as_bytes())});
    assert_eq!(
        zero.send(&app, &a, &up).await.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(epoch_of(&app, &a), 1);

    let mut rk = Rk::new(2);
    rk.identity = new_pair.clone();
    let r = rk.send(&app, &a, &up).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let acc: Value = app
        .get("/v1/account?identity=1&epoch=1", Some(&a.t))
        .await
        .json();
    assert_eq!(acc["identity"], new_pair);
    // The old pair can't come back through the plain route.
    let old = json!({"ipk": b64(&[1u8; 32]), "wrappedIsk": b64(wrapped_key(1).as_bytes())});
    let r = app.put("/v1/account/identity", Some(&a.t), &old).await;
    assert_eq!(code(&r), "identity_exists");
    assert_eq!(r.error().detail.unwrap()["ipk"], b64(&[2u8; 32]));

    // An account without an identity may publish one in its re-key.
    let b = acct(&app, "bob").await;
    let up = upload(&app, &b, 2).await;
    let mut rk = Rk::new(2);
    rk.identity = new_pair.clone();
    assert_eq!(rk.send(&app, &b, &up).await.status, StatusCode::OK);
    let acc: Value = app
        .get("/v1/account?identity=1&epoch=1", Some(&b.t))
        .await
        .json();
    assert_eq!(acc["identity"], new_pair);
    // And one without may send none.
    let c = acct(&app, "carol").await;
    let up = upload(&app, &c, 2).await;
    assert_eq!(Rk::new(2).send(&app, &c, &up).await.status, StatusCode::OK);
    let acc: Value = app
        .get("/v1/account?identity=1&epoch=1", Some(&c.t))
        .await
        .json();
    assert!(acc["identity"].is_null());
}

/// `who` gives `to` emergency access to `v`.
async fn grant(app: &TestApp, who: &Acct, to: &Acct, v: &TestVault) -> Resp {
    app.put(
        &format!("/v1/emergency/grants/{}", to.id),
        Some(&who.t),
        &json!({"waitDays": 1, "scope": 2,
                "seals": [{"vaultId": v.id.to_string(), "generation": 1, "seal": seal(9)}]}),
    )
    .await
}

// ---- seals, shares and invites ------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rekey_drops_shares_and_grantee_seals() {
    let app = open();
    let x = acct(&app, "xavier").await;
    let o = acct(&app, "owner").await;
    let g = acct(&app, "grace").await;
    // X is a member of O's vault, and O has rotated since: an unclaimed share.
    let vo = vault_with(&app, &o).await;
    join(&app, &o, &vo, &x, 4).await;
    let r = rotate(
        &app,
        &o,
        &vo,
        &Rot {
            generation: 2,
            seals: &[&x.id],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    // G gives X emergency access to a vault, and X gives G access to one.
    let vg = vault_with(&app, &g).await;
    let vx = vault_with(&app, &x).await;
    assert_eq!(
        grant(&app, &g, &x, &vg).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        grant(&app, &x, &g, &vx).await.status,
        StatusCode::NO_CONTENT
    );
    let by_x = |t: &str| {
        n(
            &app,
            &format!("SELECT COUNT(*) FROM {t} WHERE lower(hex(recipient)) = replace(?1, '-', '')"),
            &[&x.id],
        )
    };
    assert_eq!(by_x("key_shares"), 1);
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM emergency_seals WHERE lower(hex(grantee)) = replace(?1, '-', '')",
            &[&x.id]
        ),
        1
    );
    let hx = head(&app, &x, &vx).await;
    let up = upload(&app, &x, 2).await;
    let grantor_seq = inbox_seq(&app, &g);

    // X must refresh the seal it gave, at the new generation.
    let with_vault = |emergency: Vec<Value>| {
        let mut rk = Rk::new(2).vaults(vec![ve(&vx, 2, hx)]);
        rk.emergency = emergency;
        rk
    };
    let fresh =
        json!({"grantee": g.id, "vaultId": vx.id.to_string(), "generation": 2, "seal": seal(5)});
    for (what, emergency) in [
        ("none", vec![]),
        (
            "stale generation",
            vec![
                json!({"grantee": g.id, "vaultId": vx.id.to_string(), "generation": 1, "seal": seal(5)}),
            ],
        ),
        (
            "wrong vault",
            vec![
                json!({"grantee": g.id, "vaultId": vg.id.to_string(), "generation": 2, "seal": seal(5)}),
            ],
        ),
        ("twice", vec![fresh.clone(), fresh.clone()]),
    ] {
        let r = with_vault(emergency).send(&app, &x, &up).await;
        assert_eq!(r.status, StatusCode::CONFLICT, "{what}: {}", text(&r));
        assert_eq!(code(&r), "rekey_mismatch", "{what}");
        assert_eq!(r.error().detail.unwrap()["set"], "emergencySeals");
    }
    assert_eq!(epoch_of(&app, &x), 1);

    // X holds no current key of O's vault (the share is unclaimed): no rewrap.
    let r = with_vault(vec![fresh.clone()]).send(&app, &x, &up).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));

    // Shares and seals addressed to the old identity key are gone; the grant
    // itself stays; the seal X gave is the fresh one; the grantor re-seals.
    assert_eq!(by_x("key_shares"), 0);
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM emergency_seals WHERE lower(hex(grantee)) = replace(?1, '-', '')",
            &[&x.id]
        ),
        0
    );
    assert_eq!(
        n(
            &app,
            "SELECT COUNT(*) FROM emergency_grants WHERE lower(hex(grantee)) = replace(?1, '-', '')",
            &[&x.id]
        ),
        1
    );
    let (generation, stored): (i64, Vec<u8>) = sql(&app, |c| {
        c.query_row(
            "SELECT generation, seal FROM emergency_seals WHERE lower(hex(grantor)) = replace(?1, '-', '')",
            [&x.id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
    });
    assert_eq!(generation, 2);
    assert_eq!(b64(&stored), seal(5));
    assert!(inbox_seq(&app, &g) > grantor_seq);
    // O is asked to rotate vo although X held no current row.
    assert_eq!(
        n(&app, "SELECT COUNT(*) FROM vault_rotation_requests", &[]),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn rekey_cancels_incoming_invites() {
    let app = open();
    let x = acct(&app, "xavier").await;
    let o = acct(&app, "owner").await;
    let c = acct(&app, "carol").await;
    // O invites X (pending); O's second vault has X as a manager who invited C.
    let v1 = vault_with(&app, &o).await;
    let v2 = vault_with(&app, &o).await;
    join(&app, &o, &v2, &x, 2).await;
    let (to_x, r) = invite(&app, &o, &v1, &x.id, 4, 1).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    let (by_x, r) = invite(&app, &x, &v2, &c.id, 4, 1).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    let c_seq = inbox_seq(&app, &c);

    let up = upload(&app, &x, 2).await;
    let mut rk = Rk::new(2);
    rk.rewraps = vec![rewrap(&v2, 1)];
    let r = rk.send(&app, &x, &up).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));

    let state = |id: &str| {
        n(
            &app,
            "SELECT state FROM share_invites WHERE lower(hex(invite_id)) = replace(?1, '-', '')",
            &[id],
        )
    };
    assert_eq!(state(&to_x), 5, "cancelled by the recipient's re-key");
    assert_eq!(state(&by_x), 3, "its seal came from the old identity key");
    // X sees no invite; O's outbox shows the cancelled one; C was woken.
    let list: Value = app.get("/v1/invites", Some(&x.t)).await.json();
    assert_eq!(list["incoming"], json!([]));
    let list: Value = app.get("/v1/invites", Some(&o.t)).await.json();
    let states: Vec<i64> = list["outgoing"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|i| i["inviteId"] == to_x)
        .map(|i| i["state"].as_i64().unwrap())
        .collect();
    assert_eq!(states, [5]);
    assert!(inbox_seq(&app, &c) > c_seq);
    let list: Value = app.get("/v1/invites", Some(&c.t)).await.json();
    assert_eq!(list["incoming"], json!([]));
}

// ---- sessions and logins ------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rekey_ends_other_sessions() {
    let app = open();
    let a = acct(&app, "alice").await;
    app.clock.advance_ms(13_000);
    let second = mosaic_core::InstallId::new_v7().unwrap();
    let t2 = app.login_device(&a.u, second).await;
    // A session still waiting for its second factor.
    sql(&app, |c| {
        c.execute(
            "INSERT INTO sessions (token_hash, account_id, device_id, created_at, last_used_at, \
             expires_at, fresh_until, pending_2fa, totp_attempts, account_version) \
             SELECT randomblob(32), account_id, randomblob(16), 0, 0, 9999999999999, 0, 1, 0, 1 \
             FROM accounts",
            [],
        )
    });
    assert_eq!(n(&app, "SELECT COUNT(*) FROM sessions", &[]), 3);

    app.clock.advance_ms(60_000);
    // A login that started against the old password.
    let result = app
        .login_with_pause(
            &a.u.email,
            &a.u.opw,
            a.u.account_id,
            second,
            IP,
            async || {
                let up = upload(&app, &a, 2).await;
                let r = Rk::new(2).send(&app, &a, &up).await;
                assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
            },
        )
        .await;
    match result {
        LoginResult::Server(r) => {
            assert_eq!(r.status, StatusCode::UNAUTHORIZED);
            assert_eq!(code(&r), "login_failed");
        }
        _ => panic!("the login in flight must be refused"),
    }
    // The changing session stays; the others, pending ones included, are gone.
    assert_eq!(
        app.get("/v1/devices", Some(&a.t)).await.status,
        StatusCode::OK
    );
    assert_eq!(
        app.get("/v1/devices", Some(&t2)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(n(&app, "SELECT COUNT(*) FROM sessions", &[]), 1);
    assert_eq!(
        n(&app, "SELECT COUNT(*) FROM audit WHERE kind = 15", &[]),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn rekey_lost_answer_visible_by_login() {
    let app = open();
    let a = acct(&app, "alice").await;
    let v = vault_with(&app, &a).await;
    let h = head(&app, &a, &v).await;
    let up = upload(&app, &a, 2).await;
    // The client never sees the answer.
    let r = Rk::new(2)
        .vaults(vec![ve(&v, 2, h)])
        .send(&app, &a, &up)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));

    // A login with the new password shows the prepared wrapped AK and the
    // target epoch, so the client can tell the change was applied.
    app.clock.advance_ms(60_000);
    assert!(matches!(app.login(&a.u).await, LoginResult::ClientRejected));
    let LoginResult::Ok { token, .. } = app
        .login_with(&a.u.email, &NEW_OPW, a.u.account_id, a.u.device, IP)
        .await
    else {
        panic!("the new password logs in");
    };
    let acc: Value = app
        .get("/v1/account?identity=1&epoch=1", Some(&token))
        .await
        .json();
    assert_eq!(acc["wrappedAk"], b64(wrapped_key(11).as_bytes()));
    assert_eq!(acc["salt"], b64(&[6u8; 16]));
    assert_eq!(
        (&acc["akEpoch"], &acc["accountVersion"]),
        (&json!(2), &json!(2))
    );
}

// ---- the schema ---------------------------------------------------------------

#[tokio::test(start_paused = true)]
async fn schema_v3_rules() {
    let app = open();
    let o = acct(&app, "owner").await;
    let m = acct(&app, "member").await;
    let v = vault_with(&app, &o).await;
    join(&app, &o, &v, &m, 4).await;
    let (inv, r) = invite(&app, &o, &v, &crate::sharing::uuid(), 4, 1).await;
    assert_eq!(r.status, StatusCode::CREATED, "{}", text(&r));
    let bad = |q: &str| {
        app.state
            .db()
            .run(|c| Ok(c.execute(q, []).is_err()))
            .unwrap()
    };
    // Rotation requests: only reason 1, one per vault and requester.
    let req = |reason: u8| {
        format!(
            "INSERT INTO vault_rotation_requests SELECT vault_id, account_id, {reason}, 0 \
             FROM memberships WHERE role = 4"
        )
    };
    assert!(bad(&req(2)));
    assert!(!bad(&req(1)));
    assert!(bad(&req(1)));
    // Audit kinds up to 16; invite states up to 5.
    assert!(bad(
        "INSERT INTO audit SELECT account_id, 1, 17, NULL FROM accounts"
    ));
    assert!(!bad(
        "INSERT INTO audit SELECT account_id, 1, 16, NULL FROM accounts"
    ));
    let plain = inv.replace('-', "");
    assert!(!bad(&format!(
        "UPDATE share_invites SET state = 5 WHERE lower(hex(invite_id)) = '{plain}'"
    )));
    assert!(bad(&format!(
        "UPDATE share_invites SET state = 6 WHERE lower(hex(invite_id)) = '{plain}'"
    )));
    // Deleting an account removes its requests.
    sql(&app, |c| {
        c.execute(
            "DELETE FROM accounts WHERE lower(hex(account_id)) = replace(?1, '-', '')",
            [&m.id],
        )
    });
    assert_eq!(
        n(&app, "SELECT COUNT(*) FROM vault_rotation_requests", &[]),
        0
    );
}

// ---- what the server writes ---------------------------------------------------

#[tokio::test(start_paused = true)]
async fn rekey_leaves_no_plaintext_in_logs_or_on_disk() {
    use crate::no_plaintext::{contains, stored_bytes};

    let logs = crate::common::init_logs();
    let app = TestApp::with_file_db(RegistrationMode::Open);
    let peer = "203.0.113.78:51874";
    let u = User::new("rekey@rekey-domain-7b1e.example");
    assert_eq!(
        app.register_from(&u, None, peer).await.status,
        StatusCode::CREATED
    );
    let LoginResult::Ok { token, .. } = app
        .login_with(&u.email, &u.opw, u.account_id, u.device, peer)
        .await
    else {
        panic!("login failed");
    };
    let a = Acct {
        id: u.account_id.to_string(),
        u,
        t: token,
    };
    let v = vault_with(&app, &a).await;
    let h = head(&app, &a, &v).await;
    let up = upload(&app, &a, 2).await;
    let r = Rk::new(2)
        .vaults(vec![ve(&v, 2, h)])
        .send(&app, &a, &up)
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));

    let log = logs.contents();
    let text = String::from_utf8_lossy(&log).into_owned();
    assert!(
        text.contains("/v1/account/rekey/start") && text.contains("/v1/account/rekey/finish"),
        "{text}"
    );
    let forbidden: [(&str, Vec<u8>); 6] = [
        ("peer IP", b"203.0.113.78".to_vec()),
        ("request IP", b"198.51.100.1".to_vec()),
        ("old OPAQUE password", a.u.opw.to_vec()),
        ("new OPAQUE password", NEW_OPW.to_vec()),
        ("session token", a.t.to_vec()),
        ("OPAQUE upload", up.clone().into_bytes()),
    ];
    for (what, bytes) in &forbidden {
        assert!(!contains(&log, bytes), "log contains the {what}");
    }
    for s in [
        a.u.email.as_str(),
        "rekey-domain-7b1e",
        &a.id,
        &v.id.to_string(),
    ] {
        assert!(!text.contains(s), "log contains {s}");
    }
    for (name, bytes) in stored_bytes(app.dir.path()) {
        for (what, needle) in &forbidden {
            assert!(!contains(&bytes, needle), "{name} contains the {what}");
        }
    }
}
