//! Emergency access: the grant, the server-kept waiting period, refusal and
//! cooldown, release as a read membership, and the owner-only scope
//! (M6b spec §4, §8.4).

use axum::http::StatusCode;
use mosaic_server::ApiError;
use mosaic_sync::wire::json::RegistrationMode;
use serde_json::{Value, json};

use crate::common::{Resp, T0, TestApp, TestVault, wrapped_key};
use crate::sharing::{Acct, acct, b64, join, seal, text, uuid, vault_url, vault_with};

const DAY: u64 = 86_400_000;
const HOUR: u64 = 3_600_000;

fn code(r: &Resp) -> String {
    r.error().error
}

async fn grant(
    app: &TestApp,
    grantor: &Acct,
    grantee: &Acct,
    wait_days: u8,
    vaults: &[&TestVault],
) -> Resp {
    app.put(
        &format!("/v1/emergency/grants/{}", grantee.id),
        Some(&grantor.t),
        &json!({
            "waitDays": wait_days,
            "scope": 2,
            "seals": vaults.iter().map(|v| json!({
                "vaultId": v.id.to_string(), "generation": 1, "seal": seal(9)
            })).collect::<Vec<_>>(),
        }),
    )
    .await
}

async fn act(app: &TestApp, who: &Acct, path: &str, other: &Acct) -> Resp {
    app.post(
        &format!("/v1/emergency/grants/{}/{path}", other.id),
        Some(&who.t),
        &json!({}),
    )
    .await
}

async fn overview(app: &TestApp, who: &Acct) -> Value {
    let r = app.get("/v1/emergency", Some(&who.t)).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    r.json()
}

/// A new session of the same account (a fresh OPAQUE login).
async fn relogin(app: &TestApp, a: &Acct) -> Acct {
    let t = app.login_device(&a.u, a.u.device).await;
    Acct {
        u: crate::common::User {
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

fn count(app: &TestApp, table: &str) -> i64 {
    app.state
        .db()
        .run(|c| {
            Ok::<_, ApiError>(
                c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?,
            )
        })
        .unwrap()
}

#[tokio::test(start_paused = true)]
async fn emergency_state_machine() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    assert_eq!(
        grant(&app, &g, &e, 7, &[&v]).await.status,
        StatusCode::NO_CONTENT
    );

    // Both sides see it, idle. The grantee does not learn the vaults.
    let o = overview(&app, &g).await;
    assert_eq!(o["granted"][0]["grantee"], e.id);
    assert_eq!(o["granted"][0]["waitDays"], 7);
    assert_eq!(o["granted"][0]["state"], 0);
    assert_eq!(o["granted"][0]["vaults"][0]["vaultId"], v.id.to_string());
    let o = overview(&app, &e).await;
    assert_eq!(o["received"][0]["grantor"], g.id);
    assert_eq!(o["received"][0]["email"], g.u.email);
    assert!(o["received"][0].get("vaults").is_none());

    // Nothing to release before a request.
    let r = act(&app, &e, "release", &g).await;
    assert_eq!(code(&r), "emergency_not_requested");
    // The request starts the wait.
    let r = act(&app, &e, "request", &g).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let release_at = T0 + 7 * DAY;
    assert_eq!(r.json::<Value>()["releaseAt"], release_at);
    assert_eq!(overview(&app, &g).await["granted"][0]["state"], 1);
    assert_eq!(
        overview(&app, &g).await["granted"][0]["releaseAt"],
        release_at
    );
    // The wait can't be changed while it runs.
    assert_eq!(
        code(&grant(&app, &g, &e, 1, &[&v]).await),
        "emergency_waiting"
    );
    // Early release is refused, and says when.
    app.clock.advance_ms(6 * DAY);
    let r = act(&app, &e, "release", &g).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "emergency_waiting");
    assert_eq!(r.error().detail.unwrap()["releaseAt"], release_at);
    // The grantee has no access yet.
    assert_eq!(
        app.get(&vault_url(&v, "/changes?since=0"), Some(&e.t))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    // The wait ends: release hands over the seals and a read membership.
    app.clock.advance_ms(DAY);
    let r = act(&app, &e, "release", &g).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    let seals = r.json::<Value>();
    assert_eq!(seals["seals"][0]["vaultId"], v.id.to_string());
    assert_eq!(seals["seals"][0]["generation"], 1);
    assert_eq!(seals["seals"][0]["seal"], seal(9));
    assert_eq!(overview(&app, &g).await["granted"][0]["state"], 3);
    let page: Value = app
        .get(&vault_url(&v, "/changes?since=0&sharing=1"), Some(&e.t))
        .await
        .json();
    assert_eq!(page["role"], 4);
    let members: Value = app.get(&vault_url(&v, "/members"), Some(&g.t)).await.json();
    let emergency = members["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["accountId"] == e.id)
        .unwrap();
    assert_eq!(
        (emergency["role"].as_u64(), emergency["origin"].as_u64()),
        (Some(4), Some(2))
    );
    // Read only.
    let r = app
        .push(
            &e.t,
            &v,
            vec![v.item_record(mosaic_core::ItemId::new_v7().unwrap(), "x", None)],
        )
        .await;
    assert_eq!(code(&r), "role_forbidden");
    // Releasing again returns the same seals; a released grant is final.
    assert_eq!(act(&app, &e, "release", &g).await.status, StatusCode::OK);
    let g = relogin(&app, &g).await;
    assert_eq!(
        code(&grant(&app, &g, &e, 2, &[&v]).await),
        "emergency_released"
    );
    // The grantee claims the key (the emergency seal of the current generation).
    let claim = app
        .put(
            &vault_url(&v, "/keys/1/claim"),
            Some(&e.t),
            &json!({"wrapped": b64(wrapped_key(6).as_bytes()),
                    "device": crate::sharing::device(&app, &e)}),
        )
        .await;
    assert_eq!(claim.status, StatusCode::OK, "{}", text(&claim));
    // Deleting the grant removes the seals; the membership stays for the
    // owner's rotation to revoke.
    let r = app
        .delete(&format!("/v1/emergency/grants/{}", e.id), Some(&g.t))
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(count(&app, "emergency_seals"), 0);
    assert_eq!(count(&app, "emergency_grants"), 0);
    assert_eq!(
        app.get(&vault_url(&v, "/changes?since=0"), Some(&e.t))
            .await
            .status,
        StatusCode::OK
    );
    // Without a grant, release finds nothing.
    assert_eq!(
        act(&app, &e, "release", &g).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test(start_paused = true)]
async fn emergency_approve_releases_before_the_wait() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    grant(&app, &g, &e, 30, &[&v]).await;
    act(&app, &e, "request", &g).await;
    // Approving needs a fresh login.
    app.clock.advance_ms(HOUR);
    let r = act(&app, &g, "approve", &e).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(code(&r), "fresh_login_required");
    let g2 = relogin(&app, &g).await;
    assert_eq!(
        act(&app, &g2, "approve", &e).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(overview(&app, &g2).await["granted"][0]["state"], 2);
    // Repeating is fine; refusing an approved grant is not.
    assert_eq!(
        act(&app, &g2, "approve", &e).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        code(&act(&app, &g2, "decline", &e).await),
        "emergency_released"
    );
    let r = act(&app, &e, "release", &g).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!(overview(&app, &e).await["received"][0]["state"], 3);
    // Approving without a request is an error.
    let e2 = acct(&app, "other").await;
    let g3 = relogin(&app, &g).await;
    grant(&app, &g3, &e2, 1, &[&v]).await;
    assert_eq!(
        code(&act(&app, &g3, "approve", &e2).await),
        "emergency_not_requested"
    );
}

#[tokio::test(start_paused = true)]
async fn emergency_release_before_wait_refused() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    grant(&app, &g, &e, 1, &[&v]).await;
    act(&app, &e, "request", &g).await;
    app.clock.advance_ms(DAY - 1);
    let r = act(&app, &e, "release", &g).await;
    assert_eq!(code(&r), "emergency_waiting");
    assert_eq!(count(&app, "memberships"), 1, "no membership was added");
    // To the millisecond.
    app.clock.advance_ms(1);
    assert_eq!(act(&app, &e, "release", &g).await.status, StatusCode::OK);
    assert_eq!(count(&app, "memberships"), 2);
}

#[tokio::test(start_paused = true)]
async fn emergency_cooldown_after_decline() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    grant(&app, &g, &e, 7, &[&v]).await;
    act(&app, &e, "request", &g).await;
    app.clock.advance_ms(HOUR);
    // Declining needs no fresh login and returns to idle.
    assert_eq!(
        act(&app, &g, "decline", &e).await.status,
        StatusCode::NO_CONTENT
    );
    let o = overview(&app, &e).await;
    assert_eq!(o["received"][0]["state"], 0);
    let until = T0 + HOUR + DAY;
    assert_eq!(o["received"][0]["cooldownUntil"], until);
    // The release is gone with the request.
    assert_eq!(
        code(&act(&app, &e, "release", &g).await),
        "emergency_not_requested"
    );
    // A new request within 24 h is refused, and says until when.
    app.clock.advance_ms(23 * HOUR);
    let r = act(&app, &e, "request", &g).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(code(&r), "emergency_cooldown");
    assert_eq!(r.error().detail.unwrap()["until"], until);
    // After it the grantee may ask again, with a fresh wait.
    app.clock.advance_ms(HOUR);
    let r = act(&app, &e, "request", &g).await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!(r.json::<Value>()["releaseAt"], until + 7 * DAY);
    // Declining twice is harmless.
    assert_eq!(
        act(&app, &g, "decline", &e).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        act(&app, &g, "decline", &e).await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test(start_paused = true)]
async fn emergency_requests_are_rate_limited() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    grant(&app, &g, &e, 7, &[&v]).await;
    for i in 0..5 {
        assert_eq!(
            act(&app, &e, "request", &g).await.status,
            StatusCode::OK,
            "request {i}"
        );
    }
    let r = act(&app, &e, "request", &g).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(code(&r), "rate_limited");
}

#[tokio::test(start_paused = true)]
async fn emergency_owner_vaults_only() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let o = acct(&app, "owner").await;
    let e = acct(&app, "grantee").await;
    let mine = vault_with(&app, &g).await;
    let theirs = vault_with(&app, &o).await;
    // g manages a vault of o's.
    join(&app, &o, &theirs, &g, 2).await;
    // A vault shared to the grantor is excluded, even as manager.
    let r = grant(&app, &g, &e, 7, &[&mine, &theirs]).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN, "{}", text(&r));
    assert_eq!(code(&r), "emergency_owner_only");
    assert_eq!(count(&app, "emergency_grants"), 0, "nothing was stored");
    // So is a vault that does not exist, or a generation that was never reached.
    let ghost = TestVault::new();
    assert_eq!(
        code(&grant(&app, &g, &e, 7, &[&ghost]).await),
        "emergency_owner_only"
    );
    let r = app
        .put(
            &format!("/v1/emergency/grants/{}", e.id),
            Some(&g.t),
            &json!({"waitDays": 7, "scope": 1,
                    "seals": [{"vaultId": mine.id.to_string(), "generation": 5, "seal": seal(9)}]}),
        )
        .await;
    assert_eq!(code(&r), "emergency_owner_only");
    // The owner's own vault is fine, and the seal refresh checks the same rule.
    assert_eq!(
        grant(&app, &g, &e, 7, &[&mine]).await.status,
        StatusCode::NO_CONTENT
    );
    let r = app
        .put(
            &format!("/v1/emergency/grants/{}/seals", e.id),
            Some(&g.t),
            &json!({"seals": [{"vaultId": theirs.id.to_string(), "generation": 1, "seal": seal(9)}]}),
        )
        .await;
    assert_eq!(code(&r), "emergency_owner_only");
    let r = app
        .put(
            &format!("/v1/emergency/grants/{}/seals", e.id),
            Some(&g.t),
            &json!({"seals": [{"vaultId": mine.id.to_string(), "generation": 1, "seal": seal(8)}]}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    let o = overview(&app, &g).await;
    assert_eq!(o["granted"][0]["vaults"].as_array().unwrap().len(), 1);
    // Seals of a grant that does not exist.
    let r = app
        .put(
            &format!("/v1/emergency/grants/{}/seals", uuid()),
            Some(&g.t),
            &json!({"seals": []}),
        )
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

#[tokio::test(start_paused = true)]
async fn emergency_grant_input_rules() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    // Only the five waiting periods.
    for days in [0u8, 3, 31] {
        assert_eq!(
            grant(&app, &g, &e, days, &[&v]).await.status,
            StatusCode::BAD_REQUEST,
            "{days} days"
        );
    }
    for days in [1u8, 2, 7, 14, 30] {
        assert_eq!(
            grant(&app, &g, &e, days, &[&v]).await.status,
            StatusCode::NO_CONTENT,
            "{days} days"
        );
    }
    assert_eq!(overview(&app, &g).await["granted"][0]["waitDays"], 30);
    // Not to oneself, not to nobody, not twice the same vault, not with a bad seal.
    assert_eq!(
        grant(&app, &g, &g, 7, &[&v]).await.status,
        StatusCode::BAD_REQUEST
    );
    let nobody = acct(&app, "nobody").await;
    let ghost = Acct {
        id: uuid(),
        ..nobody
    };
    assert_eq!(
        grant(&app, &g, &ghost, 7, &[&v]).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        grant(&app, &g, &e, 7, &[&v, &v]).await.status,
        StatusCode::BAD_REQUEST
    );
    let r = app
        .put(
            &format!("/v1/emergency/grants/{}", e.id),
            Some(&g.t),
            &json!({"waitDays": 7, "scope": 2,
                    "seals": [{"vaultId": v.id.to_string(), "generation": 1, "seal": b64(&[0; 82])}]}),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    let r = app
        .put(
            &format!("/v1/emergency/grants/{}", e.id),
            Some(&g.t),
            &json!({"waitDays": 7, "scope": 3, "seals": []}),
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    // Creating or changing a grant needs a fresh login.
    app.clock.advance_ms(HOUR);
    let r = grant(&app, &g, &e, 7, &[&v]).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(code(&r), "fresh_login_required");
    // The grantee can't act as the grantor.
    assert_eq!(
        act(&app, &e, "approve", &g).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        act(&app, &e, "decline", &g).await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        act(&app, &g, "request", &e).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test(start_paused = true)]
async fn emergency_grant_limits() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let ge = g.u.account_id.as_bytes().to_vec();
    // 20 grants given: the 21st is refused, an update of an existing one is not.
    app.state
        .db()
        .run(|c| {
            for i in 0..19 {
                c.execute(
                    "INSERT INTO accounts (account_id, email, salt, profile, opaque_record, wrapped_ak, \
                     account_version, created_at) VALUES (randomblob(16), ?1, zeroblob(16), 1, x'00', \
                     zeroblob(74), 1, 0)",
                    [format!("filler{i}@example.com")],
                )?;
            }
            c.execute(
                "INSERT INTO emergency_grants (grantor, grantee, wait_days, scope, state, created_at) \
                 SELECT ?1, account_id, 7, 2, 0, 0 FROM accounts WHERE email LIKE 'filler%'",
                [&ge],
            )?;
            Ok::<_, ApiError>(())
        })
        .unwrap();
    assert_eq!(count(&app, "emergency_grants"), 19);
    assert_eq!(
        grant(&app, &g, &e, 7, &[]).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(count(&app, "emergency_grants"), 20);
    let o = acct(&app, "twentyfirst").await;
    let r = grant(&app, &g, &o, 7, &[]).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(code(&r), "quota_exceeded");
    assert_eq!(
        grant(&app, &g, &e, 14, &[]).await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test(start_paused = true)]
async fn emergency_membership_is_revoked_by_the_owners_rotation() {
    let app = TestApp::new(RegistrationMode::Open);
    let g = acct(&app, "grantor").await;
    let e = acct(&app, "grantee").await;
    let v = vault_with(&app, &g).await;
    grant(&app, &g, &e, 1, &[&v]).await;
    act(&app, &e, "request", &g).await;
    app.clock.advance_ms(DAY);
    assert_eq!(act(&app, &e, "release", &g).await.status, StatusCode::OK);
    // The grantee is a member now, so a rotation must seal to or revoke them.
    let g = relogin(&app, &g).await;
    let r = crate::sharing::rotate(
        &app,
        &g,
        &v,
        &crate::sharing::Rot {
            generation: 2,
            seals: &[],
            invite_seals: &[],
            revoke: &[],
        },
    )
    .await;
    assert_eq!(code(&r), "members_mismatch");
    let r = crate::sharing::rotate(
        &app,
        &g,
        &v,
        &crate::sharing::Rot {
            generation: 2,
            seals: &[],
            invite_seals: &[],
            revoke: &[&e.id],
        },
    )
    .await;
    assert_eq!(r.status, StatusCode::OK, "{}", text(&r));
    assert_eq!(
        app.get(&vault_url(&v, "/changes?since=0"), Some(&e.t))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    // The grant lives on; the stale seal can't be claimed at the new generation.
    assert_eq!(overview(&app, &g).await["granted"][0]["state"], 3);
}
