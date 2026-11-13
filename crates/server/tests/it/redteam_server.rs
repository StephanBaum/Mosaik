//! Whole-app red team (2026-11-13), sync-server surface.
//!
//! One finding and two regression tests of attacks that held. The finding is
//! marked `#[ignore]` with its id so CI stays green; it asserts the behaviour
//! the spec requires and fails against the code today.
//!
//! - `RT-server-1`: the per-account second-factor failure budget of
//!   CRYPTO_SPEC §9.3 ("ten failures per account per hour from any session")
//!   is enforced only on `POST /v1/login/totp` and the WebAuthn handoff
//!   routes. The authenticated "recent second factor" path
//!   (`super::account::require_second_factor` → `check_code`, used by TOTP
//!   disable, device removal, account deletion and WebAuthn removal) checks
//!   the code without claiming the budget, so a caller that holds a fresh full
//!   session can try codes up to the per-session rate limit (600/min) instead
//!   of ten an hour.

use axum::http::{Method, StatusCode};
use mosaic_core::{AttachmentId, ItemId};
use mosaic_sync::wire::json::{
    CodeRequest, Empty, OptionalCodeRequest, RegistrationMode, TotpSetupResponse,
};
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};

use crate::common::{IP, T0, TestApp, TestVault};

fn open_app() -> TestApp {
    TestApp::new(RegistrationMode::Open)
}

/// A current TOTP code for a base32 secret at `unix_s`.
fn code_for(secret_b32: &str, unix_s: u64) -> String {
    let otp = Otp {
        secret: OtpSecret::from_base32(secret_b32).unwrap(),
        algorithm: Algorithm::Sha1,
        kind: OtpKind::Totp {
            digits: Digits::DEFAULT,
            period: Period::DEFAULT,
        },
    };
    otp.code_at(unix_s).unwrap().expose_secret().to_owned()
}

/// Registers a user, logs in and turns TOTP on; returns the full-session token
/// and the base32 secret.
async fn totp_account(app: &TestApp) -> ([u8; 32], String) {
    let (u, token) = app.user("alice").await;
    let setup: TotpSetupResponse = app
        .post("/v1/account/totp/setup", Some(&token), &Empty {})
        .await
        .json();
    let secret = setup.secret.expose_secret().to_owned();
    let code = code_for(&secret, T0 / 1000);
    let r = app
        .post(
            "/v1/account/totp/confirm",
            Some(&token),
            &CodeRequest { code: code.into() },
        )
        .await;
    assert_eq!(r.status, StatusCode::OK, "{}", String::from_utf8_lossy(&r.body));
    let _ = u;
    (token, secret)
}

/// RT-server-1 (Medium). The per-account second-factor failure budget
/// (CRYPTO_SPEC §9.3) must cover wrong codes on the authenticated second-factor
/// routes too, not only `/v1/login/totp`. Today `require_second_factor` does
/// not claim the budget, so a fresh full session can submit far more than ten
/// wrong codes an hour. This asserts the spec's limit and so fails today.
#[tokio::test(start_paused = true)]
#[ignore = "RT-server-1: per-account 2FA failure budget (CRYPTO_SPEC §9.3) is not enforced on require_second_factor; a fresh session brute-forces codes up to the 600/min session limit"]
async fn second_factor_budget_covers_the_account_routes() {
    let app = open_app();
    let (token, _secret) = totp_account(&app).await;

    // Wrong codes on TOTP disable: the account holds its second factor, so a
    // bad code is `two_factor_invalid`. After ten wrong codes in the hour the
    // account budget must be empty and answer `rate_limited` — on any session,
    // including this one. The code does not enforce it.
    let mut codes = Vec::new();
    for _ in 0..13 {
        let r = app
            .post(
                "/v1/account/totp/disable",
                Some(&token),
                &OptionalCodeRequest {
                    code: Some("000000".into()),
                },
            )
            .await;
        codes.push(r.code());
    }
    let rate_limited = codes.iter().filter(|c| *c == "rate_limited").count();
    assert!(
        rate_limited > 0,
        "the per-account 2FA budget (ten an hour, CRYPTO_SPEC §9.3) must stop a brute force \
         on the account routes, but {} wrong codes were all answered without rate_limited: {:?}",
        codes.len(),
        codes,
    );
}

/// Attack that held: cross-vault attachment IDOR. An account that owns a vault
/// cannot read another account's attachment by naming its own vault in the
/// path; the row's vault must match the path vault
/// (`api::attachments::get`, CRYPTO_SPEC §12 access control).
#[tokio::test(start_paused = true)]
async fn attachment_of_another_vault_is_not_served() {
    let app = open_app();
    let (_a, ta) = app.user("alice").await;
    let (_b, tb) = app.user("bob").await;

    // Alice stores an attachment in her vault.
    let va = TestVault::new();
    assert_eq!(app.create_vault(&ta, &va).await.status, StatusCode::CREATED);
    let item = ItemId::new_v7().unwrap();
    let aid = AttachmentId::new_v7().unwrap();
    let put = app
        .send(
            Method::PUT,
            &format!("/v1/vaults/{}/attachments/{aid}", va.id),
            Some(&ta),
            IP,
            va.attachment(item, aid, 128),
        )
        .await;
    assert_eq!(put.status, StatusCode::CREATED);

    // Bob is not a member of Alice's vault: reading it directly is refused.
    let direct = app
        .get(
            &format!("/v1/vaults/{}/attachments/{aid}", va.id),
            Some(&tb),
        )
        .await;
    assert!(
        matches!(direct.status, StatusCode::NOT_FOUND | StatusCode::FORBIDDEN),
        "cross-account attachment read returned {}",
        direct.status,
    );

    // Bob names Alice's attachment ID under a vault he does own: the filter on
    // the row's vault must still refuse it.
    let vb = TestVault::new();
    assert_eq!(app.create_vault(&tb, &vb).await.status, StatusCode::CREATED);
    let confused = app
        .get(
            &format!("/v1/vaults/{}/attachments/{aid}", vb.id),
            Some(&tb),
        )
        .await;
    assert_eq!(
        confused.status,
        StatusCode::NOT_FOUND,
        "an attachment of another vault must not be served under a vault the caller owns",
    );
}

/// Attack that held: a non-member cannot read another account's vault change
/// feed (`api::vaults::changes` → `db::vaults::access`).
#[tokio::test(start_paused = true)]
async fn non_member_cannot_read_a_vaults_changes() {
    let app = open_app();
    let (_a, ta) = app.user("alice").await;
    let (_b, tb) = app.user("bob").await;
    let va = TestVault::new();
    assert_eq!(app.create_vault(&ta, &va).await.status, StatusCode::CREATED);
    let r = app
        .get(
            &format!("/v1/vaults/{}/changes?since=0&limit=10", va.id),
            Some(&tb),
        )
        .await;
    assert_eq!(
        r.status,
        StatusCode::FORBIDDEN,
        "a non-member read of another account's vault must be forbidden",
    );
}
