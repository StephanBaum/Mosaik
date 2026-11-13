//! M9 test-engineer pass on the WebAuthn relying party and the handoffs
//! (CRYPTO_SPEC §9.3): ceremony-type confusion, the finish budget, the live
//! limit, logout, expiry mid-ceremony and malformed match codes. These use the
//! helpers of `webauthn.rs`.

use axum::http::StatusCode;
use mosaic_passkey::{Ceremony, client_data_json};
use mosaic_sync::wire::json::{
    Empty, HandoffState, HandoffStateResponse, RegistrationMode, WebauthnEntry,
};
use serde_json::json;

use crate::common::{ORIGIN, TestApp};
use crate::webauthn::{
    Assert, Key, account_handoff, assertion_handoff, b64, challenge_of, finish, hand, invalid,
    options, register_key, set_fresh, wrong_code,
};

/// A closed or ended handoff answers like an expired one.
fn gone(r: &crate::common::Resp) {
    assert_eq!(
        r.status,
        StatusCode::GONE,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.code(), "handoff_expired");
}

fn open() -> TestApp {
    TestApp::new(RegistrationMode::Open)
}

/// A registration whose client data says `webauthn.get` is refused, and an
/// assertion whose client data says `webauthn.create` as well.
#[tokio::test]
async fn ceremony_type_confusion_is_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;

    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &token, "register", Some("x")).await);
    let c = challenge_of(&options(&app, &h, &h.code).await);
    let k2 = Key::new();
    let mut cred = k2.registration(&c, ORIGIN, 0x01);
    cred["response"]["clientDataJSON"] = b64(&client_data_json(Ceremony::Get, &c, ORIGIN)).into();
    invalid(&finish(&app, &h, &cred).await);

    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    let a = Assert {
        client_data: Some(client_data_json(Ceremony::Create, &c, ORIGIN)),
        ..Assert::default()
    };
    invalid(&finish(&app, &h, &key.assertion(&c, &a)).await);
    let list: Vec<WebauthnEntry> = app.get("/v1/account/webauthn", Some(&token)).await.json();
    assert_eq!(list.len(), 1, "nothing was registered");
}

/// Four failed finishes leave the fifth usable; five close the handoff for good.
#[tokio::test]
async fn finish_budget_is_five_attempts_per_handoff() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;

    // Four bad finishes, then a fresh challenge and a good fifth.
    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    let mut bad = key.assertion(&c, &Assert::default());
    bad["response"]["signature"] = b64(&[0u8; 8]).into();
    for _ in 0..4 {
        invalid(&finish(&app, &h, &bad).await);
    }
    let c2 = challenge_of(&options(&app, &h, &h.code).await);
    assert_eq!(
        finish(&app, &h, &key.assertion(&c2, &Assert::default()))
            .await
            .status,
        StatusCode::OK,
        "the fifth attempt still counts as an attempt and may succeed"
    );

    // Five bad finishes: the handoff fails and a fresh challenge is refused.
    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    let mut bad = key.assertion(&c, &Assert::default());
    bad["response"]["signature"] = b64(&[0u8; 8]).into();
    for _ in 0..5 {
        invalid(&finish(&app, &h, &bad).await);
    }
    gone(&options(&app, &h, &h.code).await);
    gone(&finish(&app, &h, &key.assertion(&c, &Assert::default())).await);
    let s: HandoffStateResponse = app
        .post(
            "/v1/account/webauthn/handoff/state",
            Some(&token),
            &json!({"handoffId": h.id}),
        )
        .await
        .json();
    assert_eq!(s.state, HandoffState::Failed);
}

/// A fourth live handoff of one account is refused; expiry frees the slots.
#[tokio::test]
async fn fourth_live_handoff_is_refused_until_one_expires() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    for _ in 0..3 {
        app.clock.advance_ms(1_000);
        hand(&account_handoff(&app, &token, "confirm", None).await);
    }
    app.clock.advance_ms(1_000);
    let r = account_handoff(&app, &token, "confirm", None).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(r.code(), "rate_limited");
    // Five minutes after the first ones were made, all three are gone.
    app.clock.advance_ms(300_000);
    set_fresh(&app);
    hand(&account_handoff(&app, &token, "confirm", None).await);
}

/// A logout ends the handoffs of that session.
#[tokio::test]
async fn logout_ends_the_sessions_handoffs() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &token, "confirm", None).await);
    assert_eq!(
        app.post("/v1/logout", Some(&token), &Empty {}).await.status,
        StatusCode::OK
    );
    let r = options(&app, &h, &h.code).await;
    assert_eq!(r.status, StatusCode::GONE);
    assert_eq!(r.code(), "handoff_expired");
}

/// A ceremony that outlives its handoff is refused at finish.
#[tokio::test]
async fn finish_after_the_five_minutes_is_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    app.clock.advance_ms(300_001);
    let r = finish(&app, &h, &key.assertion(&c, &Assert::default())).await;
    assert_eq!(r.status, StatusCode::GONE);
    assert_eq!(r.code(), "handoff_expired");
}

/// Only the exact six ASCII digits match: a suffix, a prefix space and
/// look-alike digits are wrong codes, and they count.
#[tokio::test]
async fn malformed_match_codes_are_wrong_and_counted() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &token, "confirm", None).await);
    for bad in [
        format!("{}0", h.code),
        format!(" {}", h.code),
        "\u{ff11}\u{ff12}\u{ff13}\u{ff14}\u{ff15}\u{ff16}".to_owned(),
        String::new(),
    ] {
        invalid(&options(&app, &h, &bad).await);
    }
    // Four counted; the fifth wrong code closes it even before the right one.
    invalid(&options(&app, &h, wrong_code(&h)).await);
    gone(&options(&app, &h, &h.code).await);
}

/// A finished handoff takes no second options request.
#[tokio::test]
async fn done_handoff_refuses_new_options() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    assert_eq!(
        finish(&app, &h, &key.assertion(&c, &Assert::default()))
            .await
            .status,
        StatusCode::OK
    );
    gone(&options(&app, &h, &h.code).await);
}
