//! Quotas, the body limit and the rate limits (M6 spec §6.4).

use axum::http::{Method, StatusCode};
use mosaic_core::{AttachmentId, InstallId, ItemId};
use mosaic_server::ApiError;
use mosaic_sync::wire::json::{
    B64, LoginStartRequest, PreloginRequest, QuotaDetail, RegistrationMode,
};
use mosaic_sync::wire::opaque::{OsRng, Suite, ke::ClientLogin};

use crate::common::{IP, LoginResult, TestApp, TestVault, User};

fn quota_kind(r: &crate::common::Resp) -> String {
    assert_eq!(
        r.status,
        StatusCode::FORBIDDEN,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    assert_eq!(r.code(), "quota_exceeded");
    r.error().detail_as::<QuotaDetail>().unwrap().kind
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
async fn quota_records_bytes_attachments_devices() {
    // Record count: the batch that would pass the limit fails as a whole.
    let app = TestApp::with(RegistrationMode::Open, |c| {
        c.limits.account_record_count = 3;
    });
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    app.push_new_items(&t, &v, 3).await;
    let r = app
        .push(
            &t,
            &v,
            vec![v.item_record(ItemId::new_v7().unwrap(), "x", None)],
        )
        .await;
    assert_eq!(quota_kind(&r), "recordCount");
    assert_eq!(count(&app, "records"), 3);
    // Replacing an existing record does not count again.
    let page: mosaic_sync::wire::json::ChangesResponse = app
        .get(
            &format!("/v1/vaults/{}/changes?since=0&limit=500", v.id),
            Some(&t),
        )
        .await
        .json();
    let first = page.records[0].clone();
    let r = app
        .push(
            &t,
            &v,
            vec![v.item_record(first.record_id, "again", Some(first.server_seq))],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);

    // Record bytes (an item is a few hundred bytes).
    let app = TestApp::with(RegistrationMode::Open, |c| {
        c.limits.account_record_bytes = 1300;
    });
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    app.push_new_items(&t, &v, 2).await;
    let r = app
        .push(
            &t,
            &v,
            vec![v.item_record(ItemId::new_v7().unwrap(), "x", None)],
        )
        .await;
    assert_eq!(quota_kind(&r), "recordBytes");
    assert_eq!(count(&app, "records"), 2);

    // Attachment bytes: soft-deleted ones count until purged.
    let item = ItemId::new_v7().unwrap();
    let probe = TestVault::new();
    let size = probe
        .attachment(item, AttachmentId::new_v7().unwrap(), 100)
        .len() as u64;
    let app = TestApp::with(RegistrationMode::Open, move |c| {
        c.limits.account_attachment_bytes = size + size / 2;
    });
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    let a1 = AttachmentId::new_v7().unwrap();
    let put = |id: AttachmentId| {
        let body = v.attachment(item, id, 100);
        let uri = format!("/v1/vaults/{}/attachments/{id}", v.id);
        let (app, t) = (&app, &t);
        async move { app.send(Method::PUT, &uri, Some(t), IP, body).await }
    };
    assert_eq!(put(a1).await.status, StatusCode::CREATED);
    let a2 = AttachmentId::new_v7().unwrap();
    assert_eq!(quota_kind(&put(a2).await), "attachmentBytes");
    app.delete(&format!("/v1/vaults/{}/attachments/{a1}", v.id), Some(&t))
        .await;
    assert_eq!(
        quota_kind(&put(a2).await),
        "attachmentBytes",
        "a soft-deleted attachment still counts"
    );
    assert_eq!(count(&app, "attachments"), 1);

    // Devices: 32 per account.
    let app = TestApp::new(RegistrationMode::Open);
    let (u, _) = app.user("alice").await;
    for _ in 1..32 {
        app.clock.advance_ms(13_000);
        app.login_device(&u, InstallId::new_v7().unwrap()).await;
    }
    app.clock.advance_ms(13_000);
    let LoginResult::Server(r) = app
        .login_with(
            &u.email,
            &u.opw,
            u.account_id,
            InstallId::new_v7().unwrap(),
            IP,
        )
        .await
    else {
        panic!("the 33rd device must be refused");
    };
    assert_eq!(quota_kind(&r), "devices");
    // A known device still logs in.
    app.clock.advance_ms(13_000);
    assert!(matches!(app.login(&u).await, LoginResult::Ok { .. }));
}

#[tokio::test(start_paused = true)]
async fn body_limit_413() {
    let app = TestApp::with(RegistrationMode::Open, |c| {
        c.limits.max_request_bytes = 16 * 1024;
    });
    let (_, t) = app.user("alice").await;
    let v = TestVault::new();
    app.create_vault(&t, &v).await;
    // 20 KiB of JSON on a JSON route.
    let big = format!(
        "{{\"generation\":1,\"records\":[],\"pad\":\"{}\"}}",
        "a".repeat(20 * 1024)
    );
    let r = app
        .send(
            Method::POST,
            &format!("/v1/vaults/{}/records", v.id),
            Some(&t),
            IP,
            big.into_bytes(),
        )
        .await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(r.code(), "too_large");
    // Also on a route before authentication.
    let r = app
        .send(
            Method::POST,
            "/v1/prelogin",
            None,
            IP,
            vec![b' '; 20 * 1024],
        )
        .await;
    assert_eq!(
        r.status,
        StatusCode::PAYLOAD_TOO_LARGE,
        "{}",
        String::from_utf8_lossy(&r.body)
    );
    // A small body is fine; the attachment route has its own, larger limit.
    assert_eq!(app.push(&t, &v, vec![]).await.status, StatusCode::OK);
    let id = AttachmentId::new_v7().unwrap();
    let body = v.attachment(ItemId::new_v7().unwrap(), id, 40 * 1024);
    assert!(body.len() > 16 * 1024);
    let r = app
        .send(
            Method::PUT,
            &format!("/v1/vaults/{}/attachments/{id}", v.id),
            Some(&t),
            IP,
            body,
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
}

async fn prelogin_from(
    app: &TestApp,
    ip: &str,
    email: &str,
    headers: &[(&str, &str)],
) -> crate::common::Resp {
    app.send_with(
        Method::POST,
        "/v1/prelogin",
        None,
        ip,
        headers,
        mosaic_sync::wire::json::to_vec(&PreloginRequest {
            email: email.to_owned(),
        })
        .unwrap(),
    )
    .await
}

#[tokio::test(start_paused = true)]
async fn rate_limits_per_ip_email_session_and_trusted_proxy_header() {
    // Prelogin per IP: burst 20, then 429 with Retry-After; another IP is fine.
    let app = TestApp::new(RegistrationMode::Open);
    for i in 0..20 {
        let r = prelogin_from(&app, "198.51.100.7:1", &format!("u{i}@example.com"), &[]).await;
        assert_eq!(r.status, StatusCode::OK, "{i}");
    }
    let limited = prelogin_from(&app, "198.51.100.7:1", "u99@example.com", &[]).await;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(limited.code(), "rate_limited");
    let retry: u32 = limited.headers["retry-after"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        (1..=6).contains(&retry),
        "10/min means one token per 6 s: {retry}"
    );
    assert_eq!(
        prelogin_from(&app, "198.51.100.8:1", "u99@example.com", &[])
            .await
            .status,
        StatusCode::OK
    );
    // The same peer again after the refill.
    app.clock.advance_ms(6_000);
    assert_eq!(
        prelogin_from(&app, "198.51.100.7:1", "u98@example.com", &[])
            .await
            .status,
        StatusCode::OK
    );
    // IPv6 clients are limited by their /64.
    for i in 0..20 {
        let ip = format!("[2001:db8:0:1::{i:x}]:1");
        assert_eq!(
            prelogin_from(&app, &ip, "v6@example.com", &[]).await.status,
            StatusCode::OK,
            "{i}"
        );
    }
    assert_eq!(
        prelogin_from(&app, "[2001:db8:0:1::ffff]:1", "v6b@example.com", &[])
            .await
            .status,
        StatusCode::TOO_MANY_REQUESTS
    );

    // Prelogin per email: 30 an hour, from any address.
    let app = TestApp::new(RegistrationMode::Open);
    for i in 0..30 {
        let ip = format!("203.0.113.{}:1", i + 1);
        assert_eq!(
            prelogin_from(&app, &ip, "same@example.com", &[])
                .await
                .status,
            StatusCode::OK,
            "{i}"
        );
    }
    let r = prelogin_from(&app, "203.0.113.200:1", "same@example.com", &[]).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    // The spelling of the address does not matter.
    let r = prelogin_from(&app, "203.0.113.201:1", " SAME@example.com", &[]).await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);

    // Login start per email: 5 a minute.
    let app = TestApp::new(RegistrationMode::Open);
    let start = |email: &str, ip: &str| -> (String, Vec<u8>) {
        let c = ClientLogin::<Suite>::start(&mut OsRng, &[3u8; 32]).unwrap();
        let body = mosaic_sync::wire::json::to_vec(&LoginStartRequest {
            email: email.to_owned(),
            credential_request: B64(c.message.serialize().to_vec()),
        })
        .unwrap();
        (ip.to_owned(), body)
    };
    for i in 0..5 {
        let (ip, body) = start("l@example.com", &format!("192.0.2.{}:1", i + 1));
        assert_eq!(
            app.send(Method::POST, "/v1/login/start", None, &ip, body)
                .await
                .status,
            StatusCode::OK
        );
    }
    let (ip, body) = start("l@example.com", "192.0.2.50:1");
    assert_eq!(
        app.send(Method::POST, "/v1/login/start", None, &ip, body)
            .await
            .status,
        StatusCode::TOO_MANY_REQUESTS
    );
    // Per IP: 10 a minute across emails.
    let app = TestApp::new(RegistrationMode::Open);
    for i in 0..10 {
        let (ip, body) = start(&format!("m{i}@example.com"), "192.0.2.99:1");
        assert_eq!(
            app.send(Method::POST, "/v1/login/start", None, &ip, body)
                .await
                .status,
            StatusCode::OK,
            "{i}"
        );
    }
    let (ip, body) = start("m10@example.com", "192.0.2.99:1");
    assert_eq!(
        app.send(Method::POST, "/v1/login/start", None, &ip, body)
            .await
            .status,
        StatusCode::TOO_MANY_REQUESTS
    );

    // Registration: 3 an hour per IP.
    let app = TestApp::new(RegistrationMode::Open);
    for i in 0..3 {
        let u = User::new(&format!("r{i}@example.com"));
        assert_eq!(
            app.register_from(&u, None, "192.0.2.5:1").await.status,
            StatusCode::CREATED
        );
    }
    let u = User::new("r3@example.com");
    assert_eq!(
        app.register_from(&u, None, "192.0.2.5:1").await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        app.register_from(&u, None, "192.0.2.6:1").await.status,
        StatusCode::CREATED
    );

    // Per session: 600 a minute.
    let (_, t) = app.user("session").await;
    for i in 0..600 {
        let r = app.get("/v1/vaults", Some(&t)).await;
        assert_eq!(r.status, StatusCode::OK, "{i}");
    }
    assert_eq!(
        app.get("/v1/vaults", Some(&t)).await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
    app.clock.advance_ms(1_000);
    assert_eq!(app.get("/v1/vaults", Some(&t)).await.status, StatusCode::OK);
}

#[tokio::test(start_paused = true)]
async fn trusted_proxy_header_is_believed_only_from_trusted_peers() {
    let app = TestApp::new(RegistrationMode::Open);
    // The proxy (127.0.0.1 is trusted by default) forwards two clients: each
    // has its own bucket, keyed by the last hop of X-Forwarded-For.
    for i in 0..20 {
        let r = prelogin_from(
            &app,
            "127.0.0.1:9",
            &format!("a{i}@example.com"),
            &[("x-forwarded-for", "203.0.113.5")],
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{i}");
    }
    let r = prelogin_from(
        &app,
        "127.0.0.1:9",
        "a99@example.com",
        &[("x-forwarded-for", "203.0.113.5")],
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    let r = prelogin_from(
        &app,
        "127.0.0.1:9",
        "b0@example.com",
        &[("x-forwarded-for", "203.0.113.6")],
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    // A client-supplied first hop is ignored: the last hop is the proxy's.
    let r = prelogin_from(
        &app,
        "127.0.0.1:9",
        "b1@example.com",
        &[("x-forwarded-for", "203.0.113.77, 203.0.113.5")],
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);

    // From an untrusted peer the header is ignored: rotating it does not help.
    let app = TestApp::new(RegistrationMode::Open);
    for i in 0..20 {
        let hop = format!("203.0.113.{i}");
        let r = prelogin_from(
            &app,
            "198.51.100.9:9",
            &format!("c{i}@example.com"),
            &[("x-forwarded-for", &hop)],
        )
        .await;
        assert_eq!(r.status, StatusCode::OK, "{i}");
    }
    let r = prelogin_from(
        &app,
        "198.51.100.9:9",
        "c99@example.com",
        &[("x-forwarded-for", "203.0.113.250")],
    )
    .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    // And a trusted peer with a garbage header falls back to the peer itself.
    let r = prelogin_from(
        &app,
        "127.0.0.1:9",
        "d@example.com",
        &[("x-forwarded-for", "not an ip")],
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test(start_paused = true)]
async fn limits_apply_before_the_body_is_read() {
    let app = TestApp::new(RegistrationMode::Open);
    // The routes without a session have their own, small body limits (the
    // global one is 8 MiB): the body is not read past them.
    let big = |n: usize| vec![b' '; n];
    let r = app
        .send(Method::POST, "/v1/prelogin", None, IP, big(5 * 1024))
        .await;
    assert_eq!(
        (r.status, r.code().as_str()),
        (StatusCode::PAYLOAD_TOO_LARGE, "too_large")
    );
    for path in [
        "/v1/register/start",
        "/v1/register/finish",
        "/v1/login/start",
        "/v1/login/finish",
    ] {
        let r = app.send(Method::POST, path, None, IP, big(17 * 1024)).await;
        assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE, "{path}");
    }
    // A client over its per-IP limit is refused before its body is parsed:
    // garbage gets 429, not 400.
    let ip = "198.51.100.200:1";
    for i in 0..20 {
        let r = prelogin_from(&app, ip, &format!("g{i}@example.com"), &[]).await;
        assert_eq!(r.status, StatusCode::OK, "{i}");
    }
    let r = app
        .send(
            Method::POST,
            "/v1/prelogin",
            None,
            ip,
            b"{not json".to_vec(),
        )
        .await;
    assert_eq!(r.status, StatusCode::TOO_MANY_REQUESTS);
    let r = app
        .send(
            Method::POST,
            "/v1/prelogin",
            None,
            IP,
            b"{not json".to_vec(),
        )
        .await;
    assert_eq!(
        r.status,
        StatusCode::BAD_REQUEST,
        "another client still gets the parse error"
    );
}
