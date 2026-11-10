//! The WebAuthn second factor (CRYPTO_SPEC §9.3, ADR-0077, ADR-0078): full
//! ceremonies against the real handlers. The authenticator is
//! `mosaic-passkey` (key, credential ID, COSE key, `none` attestation) plus
//! `p256` for signing assertions whose authenticator data and client data the
//! tests set by hand, so every refusal of §9.3 can be provoked.

use axum::http::{Method, StatusCode};
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::InstallId;
use mosaic_passkey::{
    Ceremony, Es256Key, attestation_object_none, client_data_json, new_credential_id, rp_id_hash,
};
use mosaic_server::ApiError;
use mosaic_sync::wire::json::{
    Empty, HandoffOptionsResponse, HandoffStartResponse, HandoffState, HandoffStateResponse,
    InfoFeaturesResponse, InfoResponse, LoginMethodsResponse, LoginStatus, RegistrationMode,
    RevokedDetail, WebauthnEntry,
};
use p256::ecdsa::{Signature, SigningKey, signature::Signer};
use p256::pkcs8::DecodePrivateKey;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::common::{IP, LoginResult, ORIGIN, Resp, TestApp, User};

const RP_ID: &str = "sync.example.com";
const UP: u8 = 0x01;
const UV: u8 = 0x04;

fn b64(bytes: &[u8]) -> String {
    Base64UrlUnpadded::encode_string(bytes)
}

fn unb64(s: &str) -> Vec<u8> {
    Base64UrlUnpadded::decode_vec(s).expect("base64url")
}

fn open() -> TestApp {
    TestApp::new(RegistrationMode::Open)
}

fn count(app: &TestApp, sql: &str) -> i64 {
    app.state
        .db()
        .run(|c| Ok::<_, ApiError>(c.query_row(sql, [], |r| r.get(0))?))
        .unwrap()
}

// ---- the authenticator --------------------------------------------------------

struct Key {
    sk: SigningKey,
    id: Vec<u8>,
    cose: Vec<u8>,
    /// The user handle the relying party gave at registration.
    handle: Vec<u8>,
}

impl Key {
    fn new() -> Self {
        let k = Es256Key::generate().unwrap();
        let der = k.to_pkcs8_der().unwrap();
        Self {
            sk: SigningKey::from_pkcs8_der(der.expose_secret()).unwrap(),
            id: new_credential_id().unwrap(),
            cose: k.cose_public_key().to_vec(),
            handle: Vec::new(),
        }
    }

    /// The `PublicKeyCredential` JSON of a registration with `none` attestation.
    fn registration(&self, challenge: &[u8], origin: &str, flags: u8) -> Value {
        let mut ad = rp_id_hash(RP_ID).to_vec();
        ad.push(flags | 0x40);
        ad.extend_from_slice(&0u32.to_be_bytes());
        ad.extend_from_slice(&[0u8; 16]);
        ad.extend_from_slice(&u16::try_from(self.id.len()).unwrap().to_be_bytes());
        ad.extend_from_slice(&self.id);
        ad.extend_from_slice(&self.cose);
        let att = attestation_object_none(&ad).unwrap();
        let cdj = client_data_json(Ceremony::Create, challenge, origin);
        json!({
            "id": b64(&self.id), "rawId": b64(&self.id), "type": "public-key",
            "response": {
                "clientDataJSON": b64(&cdj),
                "attestationObject": b64(&att),
                "transports": ["usb"],
            },
        })
    }

    /// The JSON of an assertion over `challenge`.
    fn assertion(&self, challenge: &[u8], o: &Assert) -> Value {
        let mut ad = rp_id_hash(&o.rp_id).to_vec();
        ad.push(o.flags);
        ad.extend_from_slice(&o.count.to_be_bytes());
        let cdj = o
            .client_data
            .clone()
            .unwrap_or_else(|| client_data_json(Ceremony::Get, challenge, &o.origin));
        let mut msg = ad.clone();
        msg.extend_from_slice(&Sha256::digest(&cdj));
        let sig: Signature = self.sk.sign(&msg);
        let mut response = json!({
            "clientDataJSON": b64(&cdj),
            "authenticatorData": b64(&ad),
            "signature": b64(sig.to_der().as_bytes()),
        });
        if let Some(h) = &o.handle {
            response["userHandle"] = b64(h).into();
        }
        json!({"id": b64(&self.id), "rawId": b64(&self.id), "type": "public-key", "response": response})
    }
}

/// What an assertion looks like; the default is a valid one with a zero
/// counter and no user handle.
struct Assert {
    origin: String,
    rp_id: String,
    flags: u8,
    count: u32,
    handle: Option<Vec<u8>>,
    client_data: Option<Vec<u8>>,
}

impl Default for Assert {
    fn default() -> Self {
        Self {
            origin: ORIGIN.to_owned(),
            rp_id: RP_ID.to_owned(),
            flags: UP | UV,
            count: 0,
            handle: None,
            client_data: None,
        }
    }
}

// ---- the routes -----------------------------------------------------------------

struct Hand {
    id: String,
    code: String,
}

fn hand(r: &Resp) -> Hand {
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    let h: HandoffStartResponse = r.json();
    assert_eq!(h.match_code.len(), 6);
    Hand {
        id: b64(&h.handoff_id.0),
        code: h.match_code,
    }
}

async fn account_handoff(
    app: &TestApp,
    token: &[u8; 32],
    purpose: &str,
    label: Option<&str>,
) -> Resp {
    app.post(
        "/v1/account/webauthn/handoff",
        Some(token),
        &json!({"purpose": purpose, "label": label}),
    )
    .await
}

async fn options(app: &TestApp, h: &Hand, code: &str) -> Resp {
    app.post(
        "/v1/webauthn/handoff/options",
        None,
        &json!({"handoffId": h.id, "code": code}),
    )
    .await
}

async fn finish(app: &TestApp, h: &Hand, credential: &Value) -> Resp {
    app.post(
        "/v1/webauthn/handoff/finish",
        None,
        &json!({"handoffId": h.id, "credential": credential}),
    )
    .await
}

/// The challenge of an options answer.
fn challenge_of(r: &Resp) -> Vec<u8> {
    let o: HandoffOptionsResponse = r.json();
    unb64(o.public_key["challenge"].as_str().unwrap())
}

/// Registers a new key through a `register` handoff.
async fn register_key(app: &TestApp, token: &[u8; 32], label: &str) -> Key {
    // The unauthenticated routes allow 10 a minute with a burst of 20.
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(app, token, "register", Some(label)).await);
    let o = options(app, &h, &h.code).await;
    assert_eq!(
        o.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&o.body)
    );
    let parsed: HandoffOptionsResponse = o.json();
    assert_eq!(parsed.public_key["attestation"], "none");
    assert_eq!(
        parsed.public_key["authenticatorSelection"]["residentKey"],
        "discouraged"
    );
    assert_eq!(
        parsed.public_key["authenticatorSelection"]["requireResidentKey"],
        false
    );
    let algs: Vec<i64> = parsed.public_key["pubKeyCredParams"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["alg"].as_i64().unwrap())
        .collect();
    assert_eq!(algs, [-7, -8]);
    let challenge = unb64(parsed.public_key["challenge"].as_str().unwrap());
    let mut key = Key::new();
    key.handle = unb64(parsed.public_key["user"]["id"].as_str().unwrap());
    assert_eq!(key.handle.len(), 16);
    let r = finish(app, &h, &key.registration(&challenge, ORIGIN, UP | UV)).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    key
}

/// A confirm or login handoff up to the options; the challenge and the handoff.
async fn assertion_handoff(
    app: &TestApp,
    token: &[u8; 32],
    purpose: Option<&str>,
) -> (Hand, Vec<u8>) {
    app.clock.advance_ms(7_000);
    let r = match purpose {
        Some(p) => account_handoff(app, token, p, None).await,
        None => {
            app.post("/v1/login/webauthn/handoff", Some(token), &Empty {})
                .await
        }
    };
    let h = hand(&r);
    let o = options(app, &h, &h.code).await;
    assert_eq!(
        o.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&o.body)
    );
    let parsed: HandoffOptionsResponse = o.json();
    assert_eq!(parsed.public_key["userVerification"], "preferred");
    let allow = parsed.public_key["allowCredentials"].as_array().unwrap();
    assert_ne!(allow.len(), 0);
    let c = challenge_of(&o);
    (h, c)
}

/// A `confirm` handoff answered by `key` with `a`; the finish answer.
async fn confirm(app: &TestApp, token: &[u8; 32], key: &Key, a: &Assert) -> Resp {
    let (h, c) = assertion_handoff(app, token, Some("confirm")).await;
    finish(app, &h, &key.assertion(&c, a)).await
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "reads naturally at the call sites"
)]
fn pending(r: LoginResult) -> [u8; 32] {
    match r {
        LoginResult::Ok {
            token,
            status: LoginStatus::TwoFactorRequired,
            ..
        } => token,
        _ => panic!("expected a pending session"),
    }
}

/// A login completed with `key`; the full session's token.
async fn login_with_key(app: &TestApp, u: &User, key: &Key) -> [u8; 32] {
    app.clock.advance_ms(13_000);
    let t = pending(app.login(u).await);
    let (h, c) = assertion_handoff(app, &t, None).await;
    let r = finish(app, &h, &key.assertion(&c, &Assert::default())).await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    let s: HandoffStateResponse = app
        .post(
            "/v1/login/webauthn/handoff/state",
            Some(&t),
            &json!({"handoffId": h.id}),
        )
        .await
        .json();
    assert_eq!(s.state, HandoffState::Done);
    t
}

fn invalid(r: &Resp) {
    assert_eq!(
        r.status,
        StatusCode::UNAUTHORIZED,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.code(), "two_factor_invalid");
}

// ---- the tests ----------------------------------------------------------------------

#[tokio::test]
async fn register_and_login_with_security_key() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "YubiKey").await;

    let list: Vec<WebauthnEntry> = app.get("/v1/account/webauthn", Some(&token)).await.json();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].label, "YubiKey");
    assert_eq!(list[0].created_at, app.clock_now());
    assert_eq!(list[0].last_used_at, None);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 17"), 1);

    let full = login_with_key(&app, &u, &key).await;
    assert_eq!(
        app.get("/v1/account", Some(&full)).await.status,
        StatusCode::OK,
        "the login handoff completed the pending session"
    );
    let list: Vec<WebauthnEntry> = app.get("/v1/account/webauthn", Some(&full)).await.json();
    assert!(list[0].last_used_at.is_some());
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 2"), 2);
}

#[tokio::test]
async fn login_requires_handoff_code_before_options() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(13_000);
    let t = pending(app.login(&u).await);
    let h = hand(
        &app.post("/v1/login/webauthn/handoff", Some(&t), &Empty {})
            .await,
    );
    let wrong = if h.code == "000000" {
        "111111"
    } else {
        "000000"
    };
    let r = options(&app, &h, wrong).await;
    invalid(&r);
    assert!(!String::from_utf8_lossy(&r.body).contains("challenge"));
    // Not six digits at all.
    invalid(&options(&app, &h, "12").await);
    assert_eq!(options(&app, &h, &h.code).await.status, StatusCode::OK);
}

#[tokio::test]
async fn five_wrong_handoff_codes_fail_the_handoff() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(13_000);
    let t = pending(app.login(&u).await);
    let h = hand(
        &app.post("/v1/login/webauthn/handoff", Some(&t), &Empty {})
            .await,
    );
    let wrong = if h.code == "000000" {
        "111111"
    } else {
        "000000"
    };
    for _ in 0..5 {
        invalid(&options(&app, &h, wrong).await);
    }
    // The right code no longer helps: the failed handoff is closed.
    let r = options(&app, &h, &h.code).await;
    assert_eq!(r.status, StatusCode::GONE);
    assert_eq!(r.code(), "handoff_expired");
    let s: HandoffStateResponse = app
        .post(
            "/v1/login/webauthn/handoff/state",
            Some(&t),
            &json!({"handoffId": h.id}),
        )
        .await
        .json();
    assert_eq!(s.state, HandoffState::Failed);
}

#[tokio::test]
async fn assertion_for_another_handoff_is_refused() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(13_000);
    let t = pending(app.login(&u).await);
    let (a, ca) = assertion_handoff(&app, &t, None).await;
    let (b, _cb) = assertion_handoff(&app, &t, None).await;
    // The challenge of A, posted to B.
    invalid(&finish(&app, &b, &key.assertion(&ca, &Assert::default())).await);
    // A is still intact and takes its own assertion.
    let r = finish(&app, &a, &key.assertion(&ca, &Assert::default())).await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test]
async fn challenge_is_single_use() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    // A finished handoff takes no second finish.
    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    let cred = key.assertion(&c, &Assert::default());
    assert_eq!(finish(&app, &h, &cred).await.status, StatusCode::OK);
    // A done handoff looks like an expired one.
    let again = finish(&app, &h, &cred).await;
    assert_eq!(again.status, StatusCode::GONE);
    assert_eq!(again.code(), "handoff_expired");
    // A failed finish uses the challenge up as well: the state is taken out
    // before verification.
    let (h, c) = assertion_handoff(&app, &token, Some("confirm")).await;
    let mut bad = key.assertion(&c, &Assert::default());
    bad["response"]["signature"] = b64(&[0u8; 8]).into();
    invalid(&finish(&app, &h, &bad).await);
    invalid(&finish(&app, &h, &key.assertion(&c, &Assert::default())).await);
}

#[tokio::test]
async fn wrong_origin_in_client_data_is_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let evil = Assert {
        origin: "https://evil.example".to_owned(),
        ..Assert::default()
    };
    invalid(&confirm(&app, &token, &key, &evil).await);
    // Registration checks the origin as well.
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &token, "register", Some("x")).await);
    let c = challenge_of(&options(&app, &h, &h.code).await);
    let k2 = Key::new();
    invalid(&finish(&app, &h, &k2.registration(&c, "https://evil.example", UP)).await);
}

#[tokio::test]
async fn wrong_rp_id_hash_is_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let wrong = Assert {
        rp_id: "evil.example".to_owned(),
        ..Assert::default()
    };
    invalid(&confirm(&app, &token, &key, &wrong).await);
}

#[tokio::test]
async fn user_presence_flag_required() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let no_up = Assert {
        flags: UV,
        ..Assert::default()
    };
    invalid(&confirm(&app, &token, &key, &no_up).await);
    // Presence without verification is enough (`userVerification: preferred`).
    let up_only = Assert {
        flags: UP,
        ..Assert::default()
    };
    assert_eq!(
        confirm(&app, &token, &key, &up_only).await.status,
        StatusCode::OK
    );
    // A registration without UP is refused too.
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &token, "register", Some("x")).await);
    let c = challenge_of(&options(&app, &h, &h.code).await);
    invalid(&finish(&app, &h, &Key::new().registration(&c, ORIGIN, 0)).await);
}

#[tokio::test]
async fn counter_regression_refused_and_audited() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let at = |n: u32| Assert {
        count: n,
        ..Assert::default()
    };
    assert_eq!(
        confirm(&app, &token, &key, &at(5)).await.status,
        StatusCode::OK
    );
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 20"), 0);
    invalid(&confirm(&app, &token, &key, &at(5)).await);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 20"), 1);
    invalid(&confirm(&app, &token, &key, &at(4)).await);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 20"), 2);
    assert_eq!(
        confirm(&app, &token, &key, &at(6)).await.status,
        StatusCode::OK
    );
    assert_eq!(
        count(&app, "SELECT sign_count FROM webauthn_credentials"),
        6
    );
}

#[tokio::test]
async fn zero_counters_accepted() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    for _ in 0..3 {
        assert_eq!(
            confirm(&app, &token, &key, &Assert::default()).await.status,
            StatusCode::OK
        );
    }
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 20"), 0);
}

#[tokio::test]
async fn duplicate_credential_id_across_accounts_refused() {
    let app = open();
    let (_a, alice) = app.user("alice").await;
    let (_b, bob) = app.user("bob").await;
    let key = register_key(&app, &alice, "k").await;
    // Bob registers a credential with Alice's ID.
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &bob, "register", Some("copy")).await);
    let c = challenge_of(&options(&app, &h, &h.code).await);
    invalid(&finish(&app, &h, &key.registration(&c, ORIGIN, UP)).await);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM webauthn_credentials"), 1);
}

#[tokio::test]
async fn eleventh_credential_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    for i in 0..10 {
        register_key(&app, &token, &format!("key {i}")).await;
    }
    let r = account_handoff(&app, &token, "register", Some("one too many")).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.code(), "quota_exceeded");
    assert_eq!(count(&app, "SELECT COUNT(*) FROM webauthn_credentials"), 10);
}

#[tokio::test]
async fn webauthn_only_account_is_pending_after_opaque() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(13_000);
    let t = pending(app.login(&u).await);
    let m: LoginMethodsResponse = app.get("/v1/login/methods", Some(&t)).await.json();
    assert_eq!((m.totp, m.webauthn), (false, true));
    assert_eq!(
        app.get("/v1/account", Some(&t)).await.status,
        StatusCode::UNAUTHORIZED
    );
    // TOTP cannot finish the login of an account that has none.
    let r = app
        .post("/v1/login/totp", Some(&t), &json!({"code": "123456"}))
        .await;
    invalid(&r);
}

fn set_fresh(app: &TestApp) {
    app.state
        .db()
        .run(|c| {
            c.execute("UPDATE sessions SET fresh_until = 9999999999999", [])?;
            Ok::<_, ApiError>(())
        })
        .unwrap();
}

#[tokio::test]
async fn confirm_handoff_allows_remove_for_five_minutes() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let k1 = register_key(&app, &token, "one").await;
    let _k2 = register_key(&app, &token, "two").await;
    let list: Vec<WebauthnEntry> = app.get("/v1/account/webauthn", Some(&token)).await.json();
    assert_eq!(list.len(), 2);
    let remove = |entry: &WebauthnEntry| json!({"credentialRef": b64(&entry.credential_ref.0), "code": null});

    // Two-step is on and nothing confirms it yet.
    invalid(
        &app.post(
            "/v1/account/webauthn/remove",
            Some(&token),
            &remove(&list[1]),
        )
        .await,
    );
    assert_eq!(
        confirm(&app, &token, &k1, &Assert::default()).await.status,
        StatusCode::OK
    );
    let r = app
        .post(
            "/v1/account/webauthn/remove",
            Some(&token),
            &remove(&list[1]),
        )
        .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 18"), 1);
    // The same reference twice: gone.
    assert_eq!(
        app.post(
            "/v1/account/webauthn/remove",
            Some(&token),
            &remove(&list[1])
        )
        .await
        .status,
        StatusCode::NOT_FOUND
    );

    // Five minutes later the confirmation is spent.
    set_fresh(&app);
    app.clock.advance_ms(301_000);
    invalid(
        &app.post(
            "/v1/account/webauthn/remove",
            Some(&token),
            &remove(&list[0]),
        )
        .await,
    );
}

#[tokio::test]
async fn removing_last_factor_turns_two_step_off() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "only").await;
    assert_eq!(
        confirm(&app, &token, &key, &Assert::default()).await.status,
        StatusCode::OK
    );
    let list: Vec<WebauthnEntry> = app.get("/v1/account/webauthn", Some(&token)).await.json();
    let r = app
        .post(
            "/v1/account/webauthn/remove",
            Some(&token),
            &json!({"credentialRef": b64(&list[0].credential_ref.0), "code": null}),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    app.clock.advance_ms(13_000);
    match app.login(&u).await {
        LoginResult::Ok { status, .. } => assert_eq!(status, LoginStatus::Ok),
        _ => panic!("login failed"),
    }
}

#[tokio::test]
async fn password_change_during_handoff_ends_it() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let h = hand(&account_handoff(&app, &token, "register", Some("k")).await);
    let r = app.change_password(&u, &token, &[0x77; 32]).await;
    assert_eq!(r.status, StatusCode::OK);
    let r = options(&app, &h, &h.code).await;
    assert_eq!(r.status, StatusCode::GONE);
    assert_eq!(r.code(), "handoff_expired");
}

#[tokio::test]
async fn handoff_expires_with_pending_session() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.clock.advance_ms(13_000);
    let t = pending(app.login(&u).await);
    let h = hand(
        &app.post("/v1/login/webauthn/handoff", Some(&t), &Empty {})
            .await,
    );
    app.clock.advance_ms(301_000);
    let r = options(&app, &h, &h.code).await;
    assert_eq!(r.status, StatusCode::GONE);
    assert_eq!(r.code(), "handoff_expired");
    assert_eq!(
        app.post(
            "/v1/login/webauthn/handoff/state",
            Some(&t),
            &json!({"handoffId": h.id})
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED,
        "the pending session itself ended"
    );
}

#[tokio::test]
async fn ip_origin_reports_webauthn_unavailable() {
    let app = TestApp::with(RegistrationMode::Open, |c| {
        c.canonical_origin = "http://127.0.0.1:8743".to_owned();
    });
    let info: InfoFeaturesResponse = app.get("/v1/info?features=1", None).await.json();
    assert!(!info.features.webauthn);
    let plain: InfoResponse = app.get("/v1/info", None).await.json();
    assert_eq!(plain.server_identity, "http://127.0.0.1:8743");
    let r = options(
        &app,
        &Hand {
            id: b64(&[1; 16]),
            code: String::new(),
        },
        "123456",
    )
    .await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "webauthn_unavailable");

    let on = open();
    let info: InfoFeaturesResponse = on.get("/v1/info?features=1", None).await.json();
    assert!(info.features.webauthn);
    let off = TestApp::with(RegistrationMode::Open, |c| c.webauthn.enabled = false);
    let info: InfoFeaturesResponse = off.get("/v1/info?features=1", None).await.json();
    assert!(!info.features.webauthn);
}

#[tokio::test]
async fn page_headers_are_strict() {
    let app = open();
    for (path, ctype) in [
        ("/webauthn/", "text/html"),
        ("/webauthn/page.js", "text/javascript"),
        ("/webauthn/page.css", "text/css"),
        ("/webauthn/strings.json", "application/json"),
    ] {
        let r = app.get(path, None).await;
        assert_eq!(r.status, StatusCode::OK, "{path}");
        let h = |name: &str| r.headers.get(name).unwrap().to_str().unwrap().to_owned();
        assert!(h("content-type").starts_with(ctype), "{path}");
        assert_eq!(
            h("content-security-policy"),
            "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; \
             img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'"
        );
        assert_eq!(h("referrer-policy"), "no-referrer");
        assert_eq!(h("cache-control"), "no-store");
        assert_eq!(h("x-content-type-options"), "nosniff");
        assert_eq!(h("cross-origin-opener-policy"), "same-origin");
        assert_eq!(
            h("permissions-policy"),
            "publickey-credentials-get=(self), publickey-credentials-create=(self)"
        );
    }
}

#[tokio::test]
async fn handoff_routes_refuse_unknown_keys() {
    let app = open();
    let (u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let bad = |r: Resp| {
        assert_eq!(
            r.status,
            StatusCode::BAD_REQUEST,
            "{}",
            String::from_utf8_lossy(&r.body)
        );
    };
    let id = b64(&[3; 16]);
    bad(app
        .post(
            "/v1/webauthn/handoff/options",
            None,
            &json!({"handoffId": id, "code": "123456", "extra": 1}),
        )
        .await);
    bad(app
        .post(
            "/v1/webauthn/handoff/finish",
            None,
            &json!({"handoffId": id, "credential": {}, "extra": 1}),
        )
        .await);
    bad(app
        .post(
            "/v1/account/webauthn/handoff/state",
            Some(&token),
            &json!({"handoffId": id, "extra": 1}),
        )
        .await);
    bad(app
        .post(
            "/v1/account/webauthn/handoff",
            Some(&token),
            &json!({"purpose": "register", "label": null, "extra": 1}),
        )
        .await);
    bad(app
        .post(
            "/v1/account/webauthn/remove",
            Some(&token),
            &json!({"credentialRef": b64(&[0; 16]), "code": null, "extra": 1}),
        )
        .await);
    app.clock.advance_ms(13_000);
    let t = pending(app.login(&u).await);
    bad(app
        .post(
            "/v1/login/webauthn/handoff/state",
            Some(&t),
            &json!({"handoffId": id, "extra": 1}),
        )
        .await);
    // A credential with an unknown member is refused by the relying party.
    let (h, c) = assertion_handoff(&app, &t, None).await;
    let mut cred = key.assertion(&c, &Assert::default());
    cred["response"]["surprise"] = 1.into();
    invalid(&finish(&app, &h, &cred).await);
    // The handoff ID is never part of a path.
    assert_eq!(
        app.get(&format!("/v1/login/webauthn/handoff/{id}"), Some(&t))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn crossorigin_and_toporigin_are_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    let cdj = |challenge: &[u8], extra: &str| {
        format!(
            "{{\"type\":\"webauthn.get\",\"challenge\":\"{}\",\"origin\":\"{ORIGIN}\"{extra}}}",
            b64(challenge)
        )
        .into_bytes()
    };
    // One handoff, a new challenge for each attempt (the state is single use).
    app.clock.advance_ms(7_000);
    let h = hand(&account_handoff(&app, &token, "confirm", None).await);
    for extra in [
        ",\"crossOrigin\":true",
        ",\"crossOrigin\":false,\"topOrigin\":\"https://evil.example\"",
        ",\"topOrigin\":\"https://sync.example.com\"",
    ] {
        let c = challenge_of(&options(&app, &h, &h.code).await);
        let a = Assert {
            client_data: Some(cdj(&c, extra)),
            ..Assert::default()
        };
        invalid(&finish(&app, &h, &key.assertion(&c, &a)).await);
    }
    // The same client data without the extra members passes.
    let c = challenge_of(&options(&app, &h, &h.code).await);
    let ok = Assert {
        client_data: Some(cdj(&c, ",\"crossOrigin\":false")),
        ..Assert::default()
    };
    assert_eq!(
        finish(&app, &h, &key.assertion(&c, &ok)).await.status,
        StatusCode::OK
    );
}
#[tokio::test]
async fn assertion_without_user_handle_is_accepted() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let key = register_key(&app, &token, "k").await;
    assert_eq!(
        confirm(&app, &token, &key, &Assert::default()).await.status,
        StatusCode::OK
    );
    // The account's own handle is fine too.
    let own = Assert {
        handle: Some(key.handle.clone()),
        ..Assert::default()
    };
    assert_eq!(
        confirm(&app, &token, &key, &own).await.status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn assertion_with_another_accounts_handle_is_refused() {
    let app = open();
    let (_u, token) = app.user("alice").await;
    let (_v, bob) = app.user("bob").await;
    let key = register_key(&app, &token, "k").await;
    let bob_key = register_key(&app, &bob, "bobs").await;
    assert_ne!(key.handle, bob_key.handle);
    let wrong = Assert {
        handle: Some(bob_key.handle.clone()),
        ..Assert::default()
    };
    invalid(&confirm(&app, &token, &key, &wrong).await);
}

#[tokio::test]
async fn reset_second_factor_removes_both_and_sessions() {
    let app = TestApp::with_file_db(RegistrationMode::Open);
    let (u, token) = app.user("alice").await;
    let _key = register_key(&app, &token, "k").await;
    app.state
        .db()
        .run(|c| {
            c.execute("UPDATE accounts SET totp_sealed = x'0102'", [])?;
            Ok::<_, ApiError>(())
        })
        .unwrap();
    app.clock.advance_ms(13_000);
    let other = pending(
        app.login_with(
            &u.email,
            &u.opw,
            u.account_id,
            InstallId::new_v7().unwrap(),
            IP,
        )
        .await,
    );

    let d = app.dir.path().display();
    let cfg = app.dir.path().join("mosaic-server.toml");
    std::fs::write(
        &cfg,
        format!(
            "listen = \"127.0.0.1:8743\"\ncanonical_origin = \"{ORIGIN}\"\ndata_dir = '{d}'\n\
             secret_file = '{d}/server.key'\nregistration = \"open\"\n"
        ),
    )
    .unwrap();
    let mut out = Vec::new();
    mosaic_server::cli::run(
        &[
            "account".to_owned(),
            "reset-second-factor".to_owned(),
            "--config".to_owned(),
            cfg.display().to_string(),
            "--email".to_owned(),
            u.email.clone(),
        ],
        &mut out,
    )
    .unwrap();

    assert_eq!(count(&app, "SELECT COUNT(*) FROM webauthn_credentials"), 0);
    assert_eq!(
        count(
            &app,
            "SELECT COUNT(*) FROM accounts WHERE totp_sealed IS NOT NULL"
        ),
        0
    );
    assert_eq!(count(&app, "SELECT COUNT(*) FROM sessions"), 0);
    assert_eq!(count(&app, "SELECT COUNT(*) FROM audit WHERE kind = 19"), 1);
    for t in [&token, &other] {
        let r = app.get("/v1/account", Some(t)).await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
        let d: RevokedDetail = r.error().detail_as().unwrap();
        assert_eq!(d.revoked, "operator");
    }
    // The password still works and no second factor is asked for.
    app.clock.advance_ms(13_000);
    match app.login(&u).await {
        LoginResult::Ok { status, .. } => assert_eq!(status, LoginStatus::Ok),
        _ => panic!("login failed"),
    }
    // Unknown accounts are reported as such.
    let err = mosaic_server::cli::run(
        &[
            "account".to_owned(),
            "reset-second-factor".to_owned(),
            "--config".to_owned(),
            cfg.display().to_string(),
            "--email".to_owned(),
            "nobody@example.com".to_owned(),
        ],
        &mut out,
    );
    assert!(matches!(
        err,
        Err(mosaic_server::cli::CliError::NoSuchAccount)
    ));
}

/// A request that panics yields a bare 500 and the handler never sees a
/// message: this checks the live router's answer for an unknown route only;
/// the layer itself is unit-tested in `api`.
#[tokio::test]
async fn unmatched_route_is_a_plain_404() {
    let app = open();
    let r = app
        .send(Method::GET, "/webauthn/nothing", None, IP, Vec::new())
        .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
}

// ---- known-answer vectors ----------------------------------------------------------

// The WebAuthn Level 3 specification publishes no registration or assertion
// test vectors with signatures that this suite could cite byte for byte (CRYPTO_SPEC
// section 16 note 8: it has no private-key vectors), and the exact bytes are not
// available offline. They are not invented here: the tests stay ignored until
// the published values are added from the specification text.

#[test]
#[ignore = "M9-KAT: vectors not available offline"]
fn webauthn_l3_es256_none_vector() {}

#[test]
#[ignore = "M9-KAT: vectors not available offline"]
fn webauthn_l3_eddsa_none_vector() {}

impl TestApp {
    fn clock_now(&self) -> u64 {
        use mosaic_server::Clock as _;
        self.clock.now_ms()
    }
}
