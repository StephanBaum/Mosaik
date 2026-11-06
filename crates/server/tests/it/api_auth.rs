//! Registration, login, sessions, TOTP, fresh-login windows (CRYPTO_SPEC §9).

use axum::http::{Method, StatusCode};
use mosaic_core::{Argon2Profile, InstallId};
use mosaic_server::ApiError;
use mosaic_sync::wire::json::{
    self, AccountResponse, AccountVersionDetail, B64, CodeRequest, DevicesResponse, Empty,
    InfoResponse, LoginFinishRequest, LoginStartRequest, LoginStartResponse, LoginStatus,
    PasswordFinishRequest, PasswordFinishResponse, PasswordStartRequest, PreloginResponse,
    RegistrationMode, RegistrationStartResponse, TotpSetupResponse,
};
use mosaic_sync::wire::opaque::{
    OsRng, Suite, identifiers,
    ke::{
        ClientLogin, ClientRegistration, ClientRegistrationFinishParameters, RegistrationResponse,
    },
};
use mosaic_totp::{Algorithm, Digits, Otp, OtpKind, OtpSecret, Period};
use sha2::{Digest, Sha256};

use crate::common::{IP, LoginResult, ORIGIN, T0, TestApp, User, wrapped_key};

fn open_app() -> TestApp {
    TestApp::new(RegistrationMode::Open)
}

#[tokio::test(start_paused = true)]
async fn register_login_roundtrip() {
    let app = open_app();
    let info = app.get("/v1/info", None).await;
    assert_eq!(info.status, StatusCode::OK);
    assert_eq!(info.headers["cache-control"], "no-store");
    let info: InfoResponse = info.json();
    assert_eq!(info.protocol, 1);
    assert_eq!(info.server_identity, ORIGIN);
    assert_eq!(info.registration, RegistrationMode::Open);

    let u = User::new("Alice@Example.com");
    assert_eq!(app.register(&u, None).await.status, StatusCode::CREATED);
    // The same email, any case, is taken.
    let dup = User::new("alice@example.COM");
    let r = app.register(&dup, None).await;
    assert_eq!(r.status, StatusCode::CONFLICT);
    assert_eq!(r.code(), "account_exists");

    let LoginResult::Ok {
        token,
        status,
        expires_at,
    } = app.login(&u).await
    else {
        panic!("login failed");
    };
    assert_eq!(status, LoginStatus::Ok);
    assert_eq!(expires_at, T0 + 30 * 86_400_000);

    let acc = app.get("/v1/account", Some(&token)).await;
    assert_eq!(acc.status, StatusCode::OK);
    let acc: AccountResponse = acc.json();
    assert_eq!(acc.account_id, u.account_id);
    assert_eq!(acc.email, "alice@example.com");
    assert_eq!(acc.salt.0, u.salt);
    assert_eq!(acc.profile, Argon2Profile::DEFAULT);
    assert_eq!(acc.wrapped_ak, wrapped_key(7));
    assert_eq!(acc.account_version, 1);
    assert!(!acc.totp_enabled);

    // No token, a wrong token and a malformed header are all unauthorized.
    assert_eq!(
        app.get("/v1/account", None).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.get("/v1/account", Some(&[9u8; 32])).await.status,
        StatusCode::UNAUTHORIZED
    );
    let r = app
        .send_with(
            Method::GET,
            "/v1/account",
            None,
            IP,
            &[("authorization", "Bearer !!")],
            vec![],
        )
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    // Unknown routes answer JSON.
    assert_eq!(app.get("/v1/nope", None).await.code(), "not_found");
}

/// What an observer can compare: status, headers that matter and the shape
/// of the JSON.
fn shape(r: &crate::common::Resp) -> (StatusCode, Vec<String>, Vec<usize>) {
    let v: serde_json::Value = serde_json::from_slice(&r.body).expect("json");
    let obj = v.as_object().expect("object");
    (
        r.status,
        obj.keys().cloned().collect(),
        obj.values().map(|v| v.to_string().len()).collect(),
    )
}

#[tokio::test(start_paused = true)]
async fn wrong_password_and_unknown_email_same_error_and_shape() {
    let app = open_app();
    let u = User::new("real@example.com");
    app.register(&u, None).await;

    // A wrong password: the server answers like for any password and the
    // client cannot open the response; an unknown email: the same.
    let wrong = [0x11u8; 32];
    assert!(matches!(
        app.login_with(&u.email, &wrong, u.account_id, u.device, IP)
            .await,
        LoginResult::ClientRejected
    ));
    assert!(matches!(
        app.login_with("nobody@example.com", &wrong, u.account_id, u.device, IP)
            .await,
        LoginResult::ClientRejected
    ));

    // The same request for both: same status, same keys, same lengths.
    let start = ClientLogin::<Suite>::start(&mut OsRng, &wrong).unwrap();
    let request = B64(start.message.serialize().to_vec());
    let mut starts = Vec::new();
    for email in ["real@example.com", "nobody@example.com"] {
        let r = app
            .post(
                "/v1/login/start",
                None,
                &LoginStartRequest {
                    email: email.to_owned(),
                    credential_request: request.clone(),
                },
            )
            .await;
        assert_eq!(r.status, StatusCode::OK);
        starts.push(r);
    }
    assert_eq!(shape(&starts[0]), shape(&starts[1]));
    assert_eq!(starts[0].body.len(), starts[1].body.len());

    // A forged finalization fails the same way for both, with the same body.
    let mut failures = Vec::new();
    for r in &starts {
        let s: LoginStartResponse = r.json();
        let f = app
            .post(
                "/v1/login/finish",
                None,
                &LoginFinishRequest {
                    login_id: s.login_id,
                    credential_finalization: B64(vec![0x42; 64]),
                    device_id: u.device,
                    device_name: "x".into(),
                },
            )
            .await;
        assert_eq!(f.status, StatusCode::UNAUTHORIZED);
        assert_eq!(f.code(), "login_failed");
        failures.push(f.body);
    }
    assert_eq!(failures[0], failures[1]);
    // An unknown or used login ID is the same answer.
    let again = app
        .post(
            "/v1/login/finish",
            None,
            &LoginFinishRequest {
                login_id: json::B64Fixed([1; 16]),
                credential_finalization: B64(vec![0x42; 64]),
                device_id: u.device,
                device_name: "x".into(),
            },
        )
        .await;
    assert_eq!(again.body, failures[0]);
    // A state is used once.
    let s: LoginStartResponse = starts[0].json();
    let reuse = app
        .post(
            "/v1/login/finish",
            None,
            &LoginFinishRequest {
                login_id: s.login_id,
                credential_finalization: B64(vec![0x42; 64]),
                device_id: u.device,
                device_name: "x".into(),
            },
        )
        .await;
    assert_eq!(reuse.body, failures[0]);

    // A login state expires after 60 s.
    let LoginResult::Ok { .. } = app.login(&u).await else {
        panic!()
    };
    let r = app
        .post(
            "/v1/login/start",
            None,
            &LoginStartRequest {
                email: u.email.clone(),
                credential_request: request,
            },
        )
        .await;
    let s: LoginStartResponse = r.json();
    app.clock.advance_ms(61_000);
    let late = app
        .post(
            "/v1/login/finish",
            None,
            &LoginFinishRequest {
                login_id: s.login_id,
                credential_finalization: B64(vec![0x42; 64]),
                device_id: u.device,
                device_name: "x".into(),
            },
        )
        .await;
    assert_eq!(late.code(), "login_failed");
}

#[tokio::test(start_paused = true)]
async fn prelogin_unknown_email_deterministic_uuidv7_shaped() {
    let app = open_app();
    let a = app.prelogin("ghost@example.com").await;
    assert_eq!(a.status, StatusCode::OK);
    let a: PreloginResponse = a.json();
    // Deterministic, and the same for any spelling of the address.
    let b: PreloginResponse = app.prelogin("  GHOST@Example.COM ").await.json();
    assert_eq!(a, b);
    let c: PreloginResponse = app.prelogin("other@example.com").await.json();
    assert_ne!(a.account_id, c.account_id);
    assert_ne!(a.salt.0, c.salt.0);

    let id = *a.account_id.as_bytes();
    assert_eq!(id[6] >> 4, 7, "UUID version");
    assert_eq!(id[8] >> 6, 0b10, "UUID variant");
    let mut ts = [0u8; 8];
    ts[2..].copy_from_slice(&id[..6]);
    let ts = u64::from_be_bytes(ts);
    let init = T0 - 1000;
    assert!(
        ts <= init && init - ts < 15_552_000_000,
        "within 180 days before init"
    );
    // No accounts yet: the client's default profile.
    assert_eq!(a.profile, Argon2Profile::DEFAULT);

    // A registered email returns its real values.
    let u = User::new("real@example.com");
    app.register(&u, None).await;
    let real: PreloginResponse = app.prelogin("real@example.com").await.json();
    assert_eq!(real.account_id, u.account_id);
    assert_eq!(real.salt.0, u.salt);

    let bad = app.prelogin("not an address").await;
    assert_eq!(bad.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad.code(), "invalid_email");
}

#[tokio::test(start_paused = true)]
async fn fake_ke2_same_length_as_real() {
    let app = open_app();
    let u = User::new("real@example.com");
    app.register(&u, None).await;
    let start = ClientLogin::<Suite>::start(&mut OsRng, &u.opw).unwrap();
    let request = B64(start.message.serialize().to_vec());
    let mut lens = Vec::new();
    for email in ["real@example.com", "fake@example.com"] {
        let r = app
            .post(
                "/v1/login/start",
                None,
                &LoginStartRequest {
                    email: email.to_owned(),
                    credential_request: request.clone(),
                },
            )
            .await;
        let s: LoginStartResponse = r.json();
        lens.push((s.credential_response.0.len(), s.login_id.0.len()));
    }
    assert_eq!(lens[0], lens[1]);
    assert!(lens[0].0 > 100, "a full KE2 message");
}

#[tokio::test(start_paused = true)]
async fn only_token_hash_stored() {
    let app = open_app();
    let (_, token) = app.user("alice").await;
    let hash: [u8; 32] = Sha256::digest(token).into();
    app.state
        .db()
        .run(|c| {
            let by_hash: i64 = c.query_row(
                "SELECT COUNT(*) FROM sessions WHERE token_hash = ?1",
                [hash.as_slice()],
                |r| r.get(0),
            )?;
            let by_token: i64 = c.query_row(
                "SELECT COUNT(*) FROM sessions WHERE token_hash = ?1",
                [token.as_slice()],
                |r| r.get(0),
            )?;
            assert_eq!((by_hash, by_token), (1, 0));
            Ok::<_, ApiError>(())
        })
        .unwrap();
}

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

#[tokio::test(start_paused = true)]
async fn totp_flow_replay_and_attempt_limit() {
    let app = open_app();
    let (u, token) = app.user("alice").await;

    let setup = app
        .post("/v1/account/totp/setup", Some(&token), &Empty {})
        .await;
    assert_eq!(setup.status, StatusCode::OK);
    let setup: TotpSetupResponse = setup.json();
    let secret = setup.secret.expose_secret().to_owned();
    assert!(
        setup
            .otpauth_uri
            .expose_secret()
            .starts_with("otpauth://totp/Mosaic%20sync:alice%40example.com?")
            || setup
                .otpauth_uri
                .expose_secret()
                .starts_with("otpauth://totp/Mosaic%20sync:alice@example.com?"),
        "{}",
        setup.otpauth_uri.expose_secret()
    );
    assert!(
        setup
            .otpauth_uri
            .expose_secret()
            .contains(&format!("secret={secret}"))
    );
    assert!(
        setup
            .otpauth_uri
            .expose_secret()
            .contains("issuer=Mosaic%20sync")
    );

    // Confirming needs a correct code.
    let now_s = || T0 / 1000;
    let wrong = app
        .post(
            "/v1/account/totp/confirm",
            Some(&token),
            &CodeRequest {
                code: "000000".into(),
            },
        )
        .await;
    assert_eq!(wrong.code(), "two_factor_invalid");
    let code = code_for(&secret, now_s());
    let ok = app
        .post(
            "/v1/account/totp/confirm",
            Some(&token),
            &CodeRequest { code: code.clone() },
        )
        .await;
    assert_eq!(
        ok.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&ok.body)
    );
    let acc: AccountResponse = app.get("/v1/account", Some(&token)).await.json();
    assert!(acc.totp_enabled);

    // The next login is pending and works only for the TOTP route.
    app.clock.advance_ms(30_000);
    let LoginResult::Ok {
        token: pending,
        status,
        ..
    } = app.login(&u).await
    else {
        panic!()
    };
    assert_eq!(status, LoginStatus::TwoFactorRequired);
    assert_eq!(
        app.get("/v1/account", Some(&pending)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.get("/v1/vaults", Some(&pending)).await.status,
        StatusCode::UNAUTHORIZED
    );

    // A code of the step that confirmed the setup is a replay.
    let replay = app
        .post("/v1/login/totp", Some(&pending), &CodeRequest { code })
        .await;
    assert_eq!(replay.code(), "two_factor_invalid");
    // The next step's code works.
    let code = code_for(&secret, now_s() + 30);
    let ok = app
        .post(
            "/v1/login/totp",
            Some(&pending),
            &CodeRequest { code: code.clone() },
        )
        .await;
    assert_eq!(
        ok.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&ok.body)
    );
    assert_eq!(
        app.get("/v1/account", Some(&pending)).await.status,
        StatusCode::OK
    );

    // The same code cannot be used again on another pending session.
    app.clock.advance_ms(13_000);
    let LoginResult::Ok { token: second, .. } = app.login(&u).await else {
        panic!()
    };
    let reuse = app
        .post("/v1/login/totp", Some(&second), &CodeRequest { code })
        .await;
    assert_eq!(reuse.code(), "two_factor_invalid");

    // Five wrong codes delete the pending session, even for the right one.
    let good = code_for(&secret, now_s() + 60);
    for _ in 0..4 {
        let r = app
            .post(
                "/v1/login/totp",
                Some(&second),
                &CodeRequest {
                    code: "111111".into(),
                },
            )
            .await;
        assert_eq!(r.status, StatusCode::UNAUTHORIZED);
    }
    let gone = app
        .post("/v1/login/totp", Some(&second), &CodeRequest { code: good })
        .await;
    assert_eq!(gone.code(), "unauthorized");

    // A completed session cannot use the TOTP route.
    let r = app
        .post(
            "/v1/login/totp",
            Some(&token),
            &CodeRequest {
                code: "123456".into(),
            },
        )
        .await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
}

#[tokio::test(start_paused = true)]
async fn fresh_login_required_for_password_totp_account_delete_device_remove() {
    let app = open_app();
    let (u, token) = app.user("alice").await;
    let other = InstallId::new_v7().unwrap();
    app.login_device(&u, other).await;

    app.clock.advance_ms(301_000);
    let stale = |what: &str, r: crate::common::Resp| {
        assert_eq!(r.status, StatusCode::FORBIDDEN, "{what}");
        assert_eq!(r.code(), "fresh_login_required", "{what}");
    };
    stale(
        "password start",
        app.post(
            "/v1/account/password/start",
            Some(&token),
            &PasswordStartRequest {
                registration_request: B64(vec![1; 32]),
            },
        )
        .await,
    );
    stale(
        "totp setup",
        app.post("/v1/account/totp/setup", Some(&token), &Empty {})
            .await,
    );
    stale(
        "totp confirm",
        app.post(
            "/v1/account/totp/confirm",
            Some(&token),
            &CodeRequest {
                code: "123456".into(),
            },
        )
        .await,
    );
    stale(
        "totp disable",
        app.post(
            "/v1/account/totp/disable",
            Some(&token),
            &CodeRequest {
                code: "123456".into(),
            },
        )
        .await,
    );
    stale(
        "delete account",
        app.send(
            Method::DELETE,
            "/v1/account",
            Some(&token),
            IP,
            br#"{"code":null}"#.to_vec(),
        )
        .await,
    );
    stale(
        "remove device",
        app.delete(&format!("/v1/devices/{other}"), Some(&token))
            .await,
    );

    // Reading and renaming need no fresh login.
    assert_eq!(
        app.get("/v1/devices", Some(&token)).await.status,
        StatusCode::OK
    );

    // A new login opens a new window.
    let fresh = app.login_device(&u, u.device).await;
    assert_eq!(
        app.post("/v1/account/totp/setup", Some(&fresh), &Empty {})
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        app.delete(&format!("/v1/devices/{other}"), Some(&fresh))
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test(start_paused = true)]
async fn password_change_cas_account_version() {
    let app = open_app();
    let (u, token) = app.user("alice").await;
    let new_opw = [0x77u8; 32];

    let mut rng = OsRng;
    let start = ClientRegistration::<Suite>::start(&mut rng, &new_opw).unwrap();
    let r = app
        .post(
            "/v1/account/password/start",
            Some(&token),
            &PasswordStartRequest {
                registration_request: B64(start.message.serialize().to_vec()),
            },
        )
        .await;
    assert_eq!(
        r.status,
        StatusCode::OK,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    let resp: RegistrationStartResponse = r.json();
    let fin = start
        .state
        .finish(
            &mut rng,
            &new_opw,
            RegistrationResponse::deserialize(&resp.registration_response.0).unwrap(),
            ClientRegistrationFinishParameters::new(
                identifiers(u.account_id.as_bytes(), ORIGIN.as_bytes()),
                None,
            ),
        )
        .unwrap();
    let finish = |version: u64| PasswordFinishRequest {
        registration_upload: B64(fin.message.serialize().to_vec()),
        wrapped_ak: wrapped_key(9),
        salt: json::B64Fixed([5; 16]),
        profile: Argon2Profile::Strong,
        expected_account_version: version,
    };
    let ok = app
        .post("/v1/account/password/finish", Some(&token), &finish(1))
        .await;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(ok.json::<PasswordFinishResponse>().account_version, 2);

    // The device that lost the race learns the current version.
    let lost = app
        .post("/v1/account/password/finish", Some(&token), &finish(1))
        .await;
    assert_eq!(lost.status, StatusCode::CONFLICT);
    assert_eq!(lost.code(), "account_version_mismatch");
    let detail: AccountVersionDetail = lost.error().detail_as().unwrap();
    assert_eq!(detail.account_version, 2);

    let acc: AccountResponse = app.get("/v1/account", Some(&token)).await.json();
    assert_eq!(
        (acc.account_version, acc.wrapped_ak, acc.salt.0, acc.profile),
        (2, wrapped_key(9), [5; 16], Argon2Profile::Strong)
    );

    // The old password no longer opens a login, the new one does.
    app.clock.advance_ms(60_000);
    assert!(matches!(app.login(&u).await, LoginResult::ClientRejected));
    assert!(matches!(
        app.login_with(&u.email, &new_opw, u.account_id, u.device, IP)
            .await,
        LoginResult::Ok { .. }
    ));
}

#[tokio::test(start_paused = true)]
async fn logout_and_device_removal_revoke() {
    let app = open_app();
    let (u, main) = app.user("alice").await;
    let second = InstallId::new_v7().unwrap();
    let third = InstallId::new_v7().unwrap();
    app.clock.advance_ms(13_000);
    let t2 = app.login_device(&u, second).await;
    app.clock.advance_ms(13_000);
    let t3 = app.login_device(&u, third).await;

    let devices: DevicesResponse = app.get("/v1/devices", Some(&main)).await.json();
    assert_eq!(devices.devices.len(), 3);
    assert_eq!(devices.devices.iter().filter(|d| d.current).count(), 1);
    assert!(
        devices
            .devices
            .iter()
            .find(|d| d.device_id == u.device)
            .unwrap()
            .current
    );

    // Rename.
    let r = app
        .send(
            Method::PATCH,
            &format!("/v1/devices/{second}"),
            Some(&main),
            IP,
            br#"{"name":"Laptop"}"#.to_vec(),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let devices: DevicesResponse = app.get("/v1/devices", Some(&main)).await.json();
    assert!(devices.devices.iter().any(|d| d.name == "Laptop"));
    let missing = app
        .send(
            Method::PATCH,
            &format!("/v1/devices/{}", InstallId::new_v7().unwrap()),
            Some(&main),
            IP,
            br#"{"name":"x"}"#.to_vec(),
        )
        .await;
    assert_eq!(missing.code(), "not_found");

    // Logout ends only that session.
    assert_eq!(
        app.post("/v1/logout", Some(&t2), &Empty {}).await.status,
        StatusCode::OK
    );
    assert_eq!(
        app.get("/v1/account", Some(&t2)).await.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.get("/v1/account", Some(&main)).await.status,
        StatusCode::OK
    );

    // Removing a device deletes its sessions.
    let current = app
        .delete(&format!("/v1/devices/{}", u.device), Some(&main))
        .await;
    assert_eq!(current.status, StatusCode::CONFLICT);
    assert_eq!(current.code(), "device_is_current");
    assert_eq!(
        app.delete(&format!("/v1/devices/{third}"), Some(&main))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        app.get("/v1/account", Some(&t3)).await.status,
        StatusCode::UNAUTHORIZED
    );
    let devices: DevicesResponse = app.get("/v1/devices", Some(&main)).await.json();
    assert!(devices.devices.iter().all(|d| d.device_id != third));
    assert_eq!(
        app.delete(&format!("/v1/devices/{third}"), Some(&main))
            .await
            .code(),
        "not_found"
    );
}

#[tokio::test(start_paused = true)]
async fn revoked_device_cannot_log_in() {
    let app = open_app();
    let (u, main) = app.user("alice").await;
    let gone = InstallId::new_v7().unwrap();
    app.clock.advance_ms(13_000);
    app.login_device(&u, gone).await;
    assert_eq!(
        app.delete(&format!("/v1/devices/{gone}"), Some(&main))
            .await
            .status,
        StatusCode::OK
    );

    app.clock.advance_ms(13_000);
    let LoginResult::Server(r) = app
        .login_with(&u.email, &u.opw, u.account_id, gone, IP)
        .await
    else {
        panic!("expected a server refusal");
    };
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.code(), "device_revoked");
    // The other devices are unaffected.
    app.clock.advance_ms(13_000);
    assert!(matches!(app.login(&u).await, LoginResult::Ok { .. }));
}

/// Registers after 22 minutes: 3 registrations per hour per IP.
async fn reg(app: &TestApp, u: &User, code: Option<&str>) -> crate::common::Resp {
    app.clock.advance_ms(1_320_000);
    app.register(u, code).await
}

#[tokio::test(start_paused = true)]
async fn registration_modes_open_invite_closed() {
    // Closed.
    let closed = TestApp::new(RegistrationMode::Closed);
    let u = User::new("a@example.com");
    let r = closed.register(&u, None).await;
    assert_eq!(r.status, StatusCode::FORBIDDEN);
    assert_eq!(r.code(), "registration_closed");
    assert_eq!(
        closed
            .get("/v1/info", None)
            .await
            .json::<InfoResponse>()
            .registration,
        RegistrationMode::Closed
    );

    // Invite: a code is needed, works once and is checked at finish. (The
    // clock moves 22 minutes between attempts: 3 registrations per hour per IP.)
    let app = TestApp::new(RegistrationMode::Invite);
    app.add_invite("ABCDEFGHJKMNPQRSTVWX");
    let none = reg(&app, &u, None).await;
    assert_eq!(
        (none.status, none.code().as_str()),
        (StatusCode::FORBIDDEN, "invalid_invite")
    );
    assert_eq!(
        reg(&app, &u, Some("ZZZZZZZZZZZZZZZZZZZZ")).await.code(),
        "invalid_invite"
    );
    assert_eq!(reg(&app, &u, Some("ABC")).await.code(), "invalid_invite");
    // The failed attempts did not use the invite; typed in lower case with
    // separators it works.
    let ok = reg(&app, &u, Some("abcde-fghjk-mnpqr-stvwx")).await;
    assert_eq!(
        ok.status,
        StatusCode::CREATED,
        "{}",
        String::from_utf8_lossy(&ok.body)
    );
    let again = User::new("b@example.com");
    assert_eq!(
        reg(&app, &again, Some("ABCDEFGHJKMNPQRSTVWX")).await.code(),
        "invalid_invite"
    );

    // A failed registration (email taken) does not burn an invite; Crockford
    // look-alike letters are mapped.
    app.add_invite("0123456789ABCDEFGHJK");
    let taken = User::new("a@example.com");
    assert_eq!(
        reg(&app, &taken, Some("0123456789ABCDEFGHJK")).await.code(),
        "account_exists"
    );
    assert_eq!(
        reg(&app, &again, Some("O123456789ABCDEFGHJK")).await.status,
        StatusCode::CREATED
    );

    // Open: no code needed, one that is given is ignored.
    let open = open_app();
    assert_eq!(
        open.register(&u, Some("whatever")).await.status,
        StatusCode::CREATED
    );
}

#[tokio::test(start_paused = true)]
async fn account_delete_with_totp_removes_everything() {
    use mosaic_core::{AttachmentId, ItemId};

    use crate::common::TestVault;

    let app = TestApp::with_file_db(RegistrationMode::Open);
    let (u, token) = app.user("alice").await;
    // A vault with a record and an attachment, and a second factor.
    let v = TestVault::new();
    app.create_vault(&token, &v).await;
    app.push_new_items(&token, &v, 2).await;
    let att = AttachmentId::new_v7().unwrap();
    let put = app
        .send(
            Method::PUT,
            &format!("/v1/vaults/{}/attachments/{att}", v.id),
            Some(&token),
            IP,
            v.attachment(ItemId::new_v7().unwrap(), att, 300),
        )
        .await;
    assert_eq!(put.status, StatusCode::CREATED);
    let setup: TotpSetupResponse = app
        .post("/v1/account/totp/setup", Some(&token), &Empty {})
        .await
        .json();
    let secret = setup.secret.expose_secret().to_owned();
    let code = code_for(&secret, T0 / 1000);
    let confirm = app
        .post(
            "/v1/account/totp/confirm",
            Some(&token),
            &CodeRequest { code },
        )
        .await;
    assert_eq!(confirm.status, StatusCode::OK);

    // The second factor is needed again for the deletion, and a replay of the
    // code that confirmed the setup does not count.
    let delete = |body: &str| {
        app.send(
            Method::DELETE,
            "/v1/account",
            Some(&token),
            IP,
            body.as_bytes().to_vec(),
        )
    };
    assert_eq!(
        delete(r#"{"code":null}"#).await.code(),
        "two_factor_invalid"
    );
    assert_eq!(
        delete(r#"{"code":"000000"}"#).await.code(),
        "two_factor_invalid"
    );
    app.clock.advance_ms(30_000);
    let next = code_for(&secret, T0 / 1000 + 30);
    assert_eq!(
        delete(&format!(r#"{{"code":"{next}"}}"#)).await.status,
        StatusCode::OK
    );

    // Everything is gone: the session, the rows, the blob.
    assert_eq!(
        app.get("/v1/account", Some(&token)).await.status,
        StatusCode::UNAUTHORIZED
    );
    for table in [
        "accounts",
        "devices",
        "sessions",
        "vaults",
        "vault_keys",
        "records",
        "attachments",
        "audit",
    ] {
        let n: i64 = app
            .state
            .db()
            .run(|c| {
                Ok::<_, ApiError>(c.query_row(
                    &format!("SELECT COUNT(*) FROM {table}"),
                    [],
                    |r| r.get(0),
                )?)
            })
            .unwrap();
        assert_eq!(n, 0, "{table}");
    }
    let blobs = app.dir.path().join("blobs");
    let files = std::fs::read_dir(&blobs).map_or(0, |d| {
        d.flatten()
            .flat_map(|e| std::fs::read_dir(e.path()).into_iter().flatten().flatten())
            .count()
    });
    assert_eq!(files, 0, "the attachment file is removed");
    // The email is free again.
    app.clock.advance_ms(3_600_000);
    assert_eq!(
        app.register(&User::new(&u.email), None).await.status,
        StatusCode::CREATED
    );
}
