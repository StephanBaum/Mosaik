//! Tests of the security-key second factor (M9 T4, `sync/webauthn_flow.rs`) against the real
//! server on `localhost` (a relying party needs a host name). The WebAuthn ceremony itself runs
//! in the browser and is tested in the server's own tests: here the account holds a stored key
//! row, the login that waits for its second factor is completed on the server with the TOTP code
//! (the stand-in for the browser page), and the handoff status call is answered by the test.

use mosaic_sync::http::hosted;
use mosaic_sync::wire::json::{HandoffState as WireState, RegistrationMode};

use crate::contract::errors::ErrorCode;
use crate::contract::events::SyncStatus;
use crate::contract::secret::SecretIn;
use crate::contract::session::SessionState;
use crate::contract::sync::{
    CodeRequest, HandoffState, HostedConnectRequest, HostedMode, JoinHostedRequest, JoinOutcome,
    SyncConfigView, TotpBeginRequest,
};
use crate::state::Backend;
use crate::sync_handler_tests::loopback::LoopbackServer;
use crate::sync_handler_tests::{Env, PASSWORD, wait_for};
use crate::test_support::kdf_permit;

/// A server whose canonical origin names the host `localhost`: only such a server can run
/// WebAuthn ceremonies.
fn host_server() -> LoopbackServer {
    let mut server = LoopbackServer::with_origin(RegistrationMode::Open, Some("localhost"));
    let port = server.base.rsplit(':').next().unwrap().to_owned();
    server.base = format!("http://localhost:{port}");
    server
}

/// Gives the account a security key (a stored row).
fn add_security_key(server: &LoopbackServer) {
    server
        .state
        .db()
        .run(|c| {
            c.execute(
                "INSERT INTO webauthn_credentials (account_id, credential_id, public_key, \
                 sign_count, label, created_at) \
                 SELECT account_id, randomblob(16), randomblob(8), 0, 'YubiKey', 0 FROM accounts",
                [],
            )?;
            Ok::<_, mosaic_server::ApiError>(())
        })
        .unwrap();
}

fn totp_code(uri: &str, server: &LoopbackServer) -> String {
    use mosaic_server::Clock as _;
    let otp = mosaic_totp::OtpAuth::parse(uri).unwrap();
    otp.otp
        .code_at(server.clock.now_ms() / 1000)
        .unwrap()
        .expose_secret()
        .to_owned()
}

/// Device `a` with the sync account, two-step verification (TOTP) and a security key on the
/// server. Returns the Secret Key and the TOTP setup link.
fn account_with_key(server: &LoopbackServer) -> (Env, String, String) {
    let a = Env::new();
    let secret_key = a.create();
    a.connect_hosted(server);
    a.wait_synced();
    server.advance_ms(60_000);
    let setup = a
        .b()
        .sync_totp_begin(&TotpBeginRequest {
            password: SecretIn::test(PASSWORD),
        })
        .unwrap();
    let uri = serde_json::to_value(&setup).unwrap()["otpauthUri"]
        .as_str()
        .unwrap()
        .to_owned();
    a.b()
        .sync_totp_confirm(&CodeRequest {
            code: SecretIn::test(&totp_code(&uri, server)),
        })
        .unwrap();
    add_security_key(server);
    a.b().sync_now().unwrap();
    a.wait_synced();
    (a, secret_key, uri)
}

fn join_request(server: &LoopbackServer, secret_key: &str) -> JoinHostedRequest {
    JoinHostedRequest {
        server_url: server.base.clone(),
        email: "alex@example.com".into(),
        password: SecretIn::test(PASSWORD),
        secret_key: SecretIn::test(secret_key),
        device_name: "Laptop".into(),
    }
}

/// What the browser page does once the user used the key: the login that waits is complete on
/// the server (here with the TOTP code), then the handoff reads `done`.
fn browser_finishes(b: &Backend, server: &LoopbackServer, uri: &str) {
    server.advance_ms(30_000);
    let code = totp_code(uri, server);
    b.sync
        .with_pending_token(|token| hosted::login_totp(&server.base, token, &code))
        .unwrap()
        .unwrap();
    b.sync.override_handoff_state(Some(WireState::Done));
}

#[test]
fn opens_only_origin_built_url() {
    let _kdf = kdf_permit();
    let server = host_server();
    let (_a, secret_key, _uri) = account_with_key(&server);

    server.advance_ms(60_000);
    let b = Env::new();
    assert_eq!(
        b.b()
            .join_hosted(join_request(&server, &secret_key))
            .unwrap(),
        JoinOutcome::TwoFactorRequired
    );
    let methods = b.b().sync_login_methods().unwrap();
    assert!(methods.totp && methods.webauthn);

    // A connect's begin does not take a join's login (no URL, no handoff).
    let wrong = b.b().sync_hosted_webauthn_begin().unwrap_err();
    assert_eq!(wrong.code, ErrorCode::SyncSignedOut);
    assert_eq!(b.opened().len(), 0);

    let begin = b.b().join_hosted_webauthn_begin().unwrap();
    assert_eq!(begin.match_code.len(), 6);
    assert!(begin.match_code.bytes().all(|c| c.is_ascii_digit()));
    // The browser opened once, on the configured origin and the page path, with the handoff ID
    // in the fragment only (22 base64url characters for 16 bytes).
    let opened = b.opened();
    assert_eq!(opened.len(), 1);
    let prefix = format!("{}/webauthn/#", server.base);
    let url = &opened[0];
    assert!(url.starts_with(&prefix), "{url}");
    let id = &url[prefix.len()..];
    assert_eq!(id.len(), 22, "{url}");
    assert!(
        id.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "{url}"
    );
    assert_eq!(url.matches('#').count(), 1);
    assert!(!url.contains('?'));

    // "Re-open" opens the same page again, nothing else.
    b.b().sync_webauthn_reopen().unwrap();
    assert_eq!(b.opened(), vec![url.clone(), url.clone()]);
}

#[test]
fn join_done_finishes_join() {
    let _kdf = kdf_permit();
    let server = host_server();
    let (_a, secret_key, uri) = account_with_key(&server);

    server.advance_ms(60_000);
    let b = Env::new();
    assert_eq!(
        b.b()
            .join_hosted(join_request(&server, &secret_key))
            .unwrap(),
        JoinOutcome::TwoFactorRequired
    );
    b.b().join_hosted_webauthn_begin().unwrap();
    // The real server says the handoff still waits.
    let waiting = b.b().sync_webauthn_status().unwrap();
    assert_eq!(waiting.state, HandoffState::Waiting);
    assert!(matches!(
        b.b().session_state().unwrap(),
        SessionState::NoAccount
    ));

    browser_finishes(b.b(), &server, &uri);
    let done = b.b().sync_webauthn_status().unwrap();
    assert_eq!(done.state, HandoffState::Done);
    assert!(matches!(
        b.b().session_state().unwrap(),
        SessionState::Unlocked {
            kit_verified: true,
            ..
        }
    ));
    // The device is set up like one that signed in with a code: hosted, with its token.
    assert!(matches!(
        b.status().config,
        Some(SyncConfigView::Hosted { .. })
    ));
    assert!(b.token_stored());
}

#[test]
fn status_done_finishes_connect() {
    let _kdf = kdf_permit();
    let server = host_server();
    let (a, _secret_key, uri) = account_with_key(&server);

    // The server signs this device out; it signs in again and meets the second factor.
    a.b().sync_disconnect().unwrap();
    server.advance_ms(60_000);
    let asked = a
        .b()
        .sync_hosted_connect(HostedConnectRequest {
            server_url: server.base.clone(),
            mode: HostedMode::SignIn,
            password: SecretIn::test(PASSWORD),
            invite_code: None,
            device_name: "Mosaic on Windows".into(),
        })
        .unwrap_err();
    assert_eq!(asked.code, ErrorCode::SyncTwoFactorRequired);
    let methods = a.b().sync_login_methods().unwrap();
    assert!(methods.totp && methods.webauthn);
    let begin = a.b().sync_hosted_webauthn_begin().unwrap();
    assert_eq!(begin.match_code.len(), 6);
    assert_eq!(a.opened().len(), 1);
    assert!(a.opened()[0].starts_with(&format!("{}/webauthn/#", server.base)));
    assert_eq!(
        a.b().sync_webauthn_status().unwrap().state,
        HandoffState::Waiting
    );
    assert!(a.status().config.is_none());

    browser_finishes(a.b(), &server, &uri);
    assert_eq!(
        a.b().sync_webauthn_status().unwrap().state,
        HandoffState::Done
    );
    // The connect finished as after a code: configured, token kept, syncing.
    assert!(matches!(
        a.status().config,
        Some(SyncConfigView::Hosted { .. })
    ));
    assert!(a.token_stored());
    wait_for("the sync thread", || {
        a.status().state.status != SyncStatus::NotConfigured
    });
    // The handoff is gone: another status call has nothing to report.
    assert_eq!(
        a.b().sync_webauthn_status().unwrap_err().code,
        ErrorCode::SyncSignedOut
    );
}

#[test]
fn expired_handoff_reports_expired() {
    let _kdf = kdf_permit();
    let server = host_server();
    let (_a, secret_key, _uri) = account_with_key(&server);

    server.advance_ms(60_000);
    let b = Env::new();
    assert_eq!(
        b.b()
            .join_hosted(join_request(&server, &secret_key))
            .unwrap(),
        JoinOutcome::TwoFactorRequired
    );
    // No handoff yet: nothing to ask about, nothing to re-open.
    assert_eq!(
        b.b().sync_webauthn_status().unwrap_err().code,
        ErrorCode::SyncHandoffExpired
    );
    assert_eq!(
        b.b().sync_webauthn_reopen().unwrap_err().code,
        ErrorCode::SyncHandoffExpired
    );

    // The server says the handoff ran out: `expired`, and the handoff is dropped (the login
    // may still be finished with a code).
    b.b().join_hosted_webauthn_begin().unwrap();
    b.b().sync.override_handoff_state(Some(WireState::Expired));
    assert_eq!(
        b.b().sync_webauthn_status().unwrap().state,
        HandoffState::Expired
    );
    b.b().sync.override_handoff_state(None);
    assert_eq!(
        b.b().sync_webauthn_reopen().unwrap_err().code,
        ErrorCode::SyncHandoffExpired
    );
    // A new handoff can start on the same login, and after the login's own five minutes
    // everything is gone.
    b.b().join_hosted_webauthn_begin().unwrap();
    b.b().sync.advance(std::time::Duration::from_secs(301));
    b.b().sync.sweep();
    assert_eq!(
        b.b().sync_webauthn_status().unwrap_err().code,
        ErrorCode::SyncSignedOut
    );
}
