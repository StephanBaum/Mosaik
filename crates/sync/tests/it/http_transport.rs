//! The HTTP transport and the hosted client functions against the real
//! server on `127.0.0.1:0` (M6 T4), and against canned answers of a raw
//! socket for the responses a real server never sends.

#![expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "integration tests"
)]
#![allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::excessive_nesting,
    reason = "the scenarios are long"
)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use mosaic_core::sync_auth::session_token;
use mosaic_core::{
    Argon2Profile, AttachmentId, AttachmentKey, InstallId, ItemCiphertext, ItemId, KeyGeneration,
    MasterPassword, SessionToken, TombstoneCiphertext, VaultId, VaultKey, VaultMetaCiphertext,
    WrappedKey,
};
use mosaic_sync::http::{HttpTransport, canonical_origin, hosted};
use mosaic_sync::transport::{
    Cursor, KeyPushResult, PushRecord, PushResult, Rejection, Transport, TransportError,
    VaultCreateResult,
};
use mosaic_sync::wire::json::{LoginStatus, RegistrationMode};
use mosaic_vault::session::{LockedSession, UnlockedSession};
use mosaic_vault::storage::{RecordBody, Store};

#[path = "../../../server/tests/it/loopback.rs"]
mod loopback;

use loopback::LoopbackServer;

const PASSWORD: &str = "correct horse battery staple";

fn token(fill: u8) -> SessionToken {
    session_token(&[fill; 32]).unwrap()
}

// ---- a raw socket server -------------------------------------------------------

/// Answers every request with the bytes `answer(path)` returns, until
/// dropped. One connection at a time, `Connection: close`.
struct RawServer {
    base: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl RawServer {
    fn start(answer: impl Fn(&str, &str) -> Vec<u8> + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let port = listener.local_addr().unwrap().port();
        let origin = format!("http://127.0.0.1:{port}");
        let stop = Arc::new(AtomicBool::new(false));
        let stopped = stop.clone();
        let served = origin.clone();
        let thread = std::thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(5)))
                            .unwrap();
                        let mut request = Vec::new();
                        let mut buf = [0u8; 1024];
                        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                            match stream.read(&mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => request.extend_from_slice(&buf[..n]),
                            }
                        }
                        let head = String::from_utf8_lossy(&request).into_owned();
                        let path = head.split_whitespace().nth(1).unwrap_or("").to_owned();
                        let reply = answer(&path, &served);
                        let _ = stream.write_all(&reply);
                        let _ = stream.flush();
                        let _ = stream.shutdown(std::net::Shutdown::Both);
                    }
                    Err(_) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        });
        Self {
            base: origin,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for RawServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn http(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut out = format!("HTTP/1.1 {status}\r\nConnection: close\r\n").into_bytes();
    for (k, v) in headers {
        out.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    out.extend_from_slice(body);
    out
}

fn info_json(origin: &str, extra: &str) -> Vec<u8> {
    format!(
        "{{\"protocol\":1,\"serverIdentity\":\"{origin}\",\"registration\":\"open\",\
         \"limits\":{{\"maxRequestBytes\":8388608,\"maxAttachmentBytes\":52428800,\
         \"accountAttachmentBytes\":1073741824,\"accountRecordBytes\":1073741824}}{extra}}}"
    )
    .into_bytes()
}

fn json_ok(body: &[u8]) -> Vec<u8> {
    http("200 OK", &[("Content-Type", "application/json")], body)
}

// ---- URL and response rules ------------------------------------------------------------

#[test]
fn refuses_http_non_loopback() {
    for url in [
        "http://sync.example.com",
        "http://10.0.0.5:8743",
        "ftp://sync.example.com",
    ] {
        assert_eq!(
            HttpTransport::connect(url, token(1)).err(),
            Some(TransportError::InsecureUrl),
            "{url}"
        );
        assert_eq!(
            HttpTransport::info(url).err(),
            Some(TransportError::InsecureUrl)
        );
        assert_eq!(
            hosted::prelogin(url, "a@example.com").err(),
            Some(TransportError::InsecureUrl)
        );
    }
    assert_eq!(
        HttpTransport::connect("not a url", token(1)).err(),
        Some(TransportError::InvalidUrl)
    );
    // Loopback over http is the one exception, and https stays allowed.
    assert!(canonical_origin("http://127.0.0.1:1234").is_ok());
    assert!(canonical_origin("http://localhost:1234").is_ok());
    assert!(canonical_origin("http://[::1]:1234").is_ok());
    assert!(canonical_origin("https://sync.example.com").is_ok());
}

#[test]
fn rejects_unknown_response_fields() {
    let server = RawServer::start(|_, origin| json_ok(&info_json(origin, ",\"surprise\":true")));
    assert_eq!(
        HttpTransport::connect(&server.base, token(1)).err(),
        Some(TransportError::Protocol)
    );
    // Missing keys are as bad as unknown ones.
    let server = RawServer::start(|_, _| json_ok(b"{\"protocol\":1}"));
    assert_eq!(
        HttpTransport::info(&server.base).err(),
        Some(TransportError::Protocol)
    );
    // A page of changes with an extra key, behind a valid info.
    let server = RawServer::start(|path, origin| {
        if path == "/v1/info" {
            json_ok(&info_json(origin, ""))
        } else {
            json_ok(
                b"{\"headSeq\":0,\"generation\":1,\"keys\":[],\"meta\":null,\"records\":[],\
                  \"more\":false,\"nextSince\":0,\"extra\":1}",
            )
        }
    });
    let mut t = HttpTransport::connect(&server.base, token(1)).unwrap();
    assert_eq!(
        t.pull(VaultId::new_v7().unwrap(), &Cursor::default(), 10)
            .err(),
        Some(TransportError::Protocol)
    );
}

#[test]
fn response_size_capped() {
    // A body announced above the 16 MiB cap is refused before it is read.
    let server = RawServer::start(|_, _| {
        let mut r =
            b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\n".to_vec();
        r.extend_from_slice(b"Content-Length: 20000000\r\n\r\n");
        r.extend_from_slice(&vec![b' '; 4096]);
        r
    });
    assert_eq!(
        HttpTransport::info(&server.base).err(),
        Some(TransportError::Protocol)
    );
    // The same without a length: read up to the cap, then refused.
    let server = RawServer::start(|_, _| {
        let mut r =
            b"HTTP/1.1 200 OK\r\nConnection: close\r\nContent-Type: application/json\r\n\r\n"
                .to_vec();
        r.extend_from_slice(&vec![b' '; 17 * 1024 * 1024]);
        r
    });
    assert_eq!(
        HttpTransport::info(&server.base).err(),
        Some(TransportError::Protocol)
    );
}

#[test]
fn server_identity_mismatch_refused() {
    // The server names itself `localhost`; the client was told 127.0.0.1.
    let server = LoopbackServer::with_origin(RegistrationMode::Open, Some("localhost"));
    assert_eq!(
        HttpTransport::connect(&server.base, token(1)).err(),
        Some(TransportError::ServerMismatch)
    );
    assert_eq!(
        HttpTransport::info(&server.base).err(),
        Some(TransportError::ServerMismatch)
    );
    assert_eq!(
        hosted::prelogin(&server.base, "a@example.com").err(),
        Some(TransportError::ServerMismatch)
    );
    // Another protocol version is not spoken.
    let raw = RawServer::start(|_, origin| {
        json_ok(
            &String::from_utf8(info_json(origin, ""))
                .unwrap()
                .replace("\"protocol\":1", "\"protocol\":2")
                .into_bytes(),
        )
    });
    assert_eq!(
        HttpTransport::info(&raw.base).err(),
        Some(TransportError::Incompatible)
    );
}

#[test]
fn retry_after_mapped() {
    let limited = |retry: Option<&'static str>| {
        RawServer::start(move |path, origin| {
            if path == "/v1/info" {
                return json_ok(&info_json(origin, ""));
            }
            let body = b"{\"error\":\"rate_limited\",\"detail\":null}";
            match retry {
                Some(v) => http(
                    "429 Too Many Requests",
                    &[("Content-Type", "application/json"), ("Retry-After", v)],
                    body,
                ),
                None => http(
                    "429 Too Many Requests",
                    &[("Content-Type", "application/json")],
                    body,
                ),
            }
        })
    };
    for (header, seconds) in [
        (Some("7"), 7),
        (None, 60),
        (Some("Wed, 21 Oct 2026 07:28:00 GMT"), 60),
        (Some("0"), 1),
        (Some("99999999"), 86_400),
    ] {
        let server = limited(header);
        let mut t = HttpTransport::connect(&server.base, token(1)).unwrap();
        assert_eq!(
            t.list_vaults().err(),
            Some(TransportError::RateLimited {
                retry_after_s: seconds
            }),
            "{header:?}"
        );
    }
}

#[test]
fn error_codes_map_to_transport_errors() {
    let answer = |status: &'static str, body: &'static str| {
        RawServer::start(move |path, origin| {
            if path == "/v1/info" {
                json_ok(&info_json(origin, ""))
            } else {
                http(
                    status,
                    &[("Content-Type", "application/json")],
                    body.as_bytes(),
                )
            }
        })
    };
    let cases: [(&str, &str, TransportError); 9] = [
        (
            "401 Unauthorized",
            "{\"error\":\"unauthorized\",\"detail\":null}",
            TransportError::Unauthorized,
        ),
        (
            "403 Forbidden",
            "{\"error\":\"fresh_login_required\",\"detail\":null}",
            TransportError::FreshLoginRequired,
        ),
        (
            "403 Forbidden",
            "{\"error\":\"forbidden\",\"detail\":null}",
            TransportError::Forbidden,
        ),
        (
            "404 Not Found",
            "{\"error\":\"not_found\",\"detail\":null}",
            TransportError::NotFound,
        ),
        (
            "413 Payload Too Large",
            "{\"error\":\"too_large\",\"detail\":null}",
            TransportError::Rejected(Rejection::TooLarge),
        ),
        (
            "409 Conflict",
            "{\"error\":\"quota_exceeded\",\"detail\":{\"kind\":\"records\"}}",
            TransportError::QuotaExceeded,
        ),
        (
            "503 Service Unavailable",
            "unavailable",
            TransportError::Unreachable,
        ),
        (
            "500 Internal Server Error",
            "{\"error\":\"internal\",\"detail\":null}",
            TransportError::Io,
        ),
        (
            "418 I'm a teapot",
            "{\"error\":\"bogus\",\"detail\":null}",
            TransportError::Protocol,
        ),
    ];
    for (status, body, expected) in cases {
        let server = answer(status, body);
        let mut t = HttpTransport::connect(&server.base, token(1)).unwrap();
        assert_eq!(t.list_vaults().err(), Some(expected), "{status} {body}");
    }
    let server = answer(
        "409 Conflict",
        "{\"error\":\"generation_mismatch\",\"detail\":{\"currentGeneration\":4}}",
    );
    let mut t = HttpTransport::connect(&server.base, token(1)).unwrap();
    assert_eq!(
        t.list_vaults().err(),
        Some(TransportError::GenerationMismatch {
            current: KeyGeneration::new(4).unwrap()
        })
    );
}

// ---- against the real server ---------------------------------------------------------------

struct Account {
    _dir: tempfile::TempDir,
    session: UnlockedSession,
    secret_key: mosaic_core::SecretKey,
    email: String,
    device: InstallId,
}

fn register(server: &LoopbackServer, name: &str) -> Account {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("a.db")).unwrap();
    let device = store.install_id().unwrap();
    let (session, created) = LockedSession::open(store)
        .create_account(
            &MasterPassword::new(PASSWORD.to_owned()),
            Argon2Profile::Baseline,
        )
        .unwrap();
    let email = format!("{name}@example.com");
    let boot = session.account_bootstrap().unwrap();
    let opw = session
        .opaque_password(
            &MasterPassword::new(PASSWORD.to_owned()),
            &created.secret_key,
        )
        .unwrap();
    hosted::register(
        &server.base,
        &email,
        None,
        &boot,
        &opw,
        device,
        "Test device",
    )
    .unwrap();
    Account {
        _dir: dir,
        session,
        secret_key: created.secret_key,
        email,
        device,
    }
}

fn login(server: &LoopbackServer, a: &Account) -> hosted::LoginGrant {
    server.advance_ms(60_000);
    let opw = a
        .session
        .opaque_password(&MasterPassword::new(PASSWORD.to_owned()), &a.secret_key)
        .unwrap();
    hosted::login(
        &server.base,
        &a.email,
        a.session.account_id(),
        &opw,
        a.device,
        "Test device",
    )
    .unwrap()
}

fn fake_wrapped(fill: u8) -> WrappedKey {
    let mut b = [fill; WrappedKey::LEN];
    b[0] = 1;
    b[1] = 1;
    WrappedKey::from_bytes(&b).unwrap()
}

struct Vault {
    id: VaultId,
    key: VaultKey,
}

impl Vault {
    fn new() -> Self {
        Self {
            id: VaultId::new_v7().unwrap(),
            key: VaultKey::generate(KeyGeneration::FIRST).unwrap(),
        }
    }

    fn meta(&self) -> VaultMetaCiphertext {
        VaultMetaCiphertext::seal(&self.key, &self.id, b"{\"name\":\"x\"}").unwrap()
    }

    fn item(&self, id: ItemId, text: &str) -> RecordBody {
        RecordBody::Item(
            ItemCiphertext::seal(&self.key, &self.id, &id, 1, text.as_bytes()).unwrap(),
        )
    }

    fn tombstone(&self, id: ItemId) -> RecordBody {
        RecordBody::Tombstone(TombstoneCiphertext::seal(&self.key, &self.id, &id, b"{}").unwrap())
    }
}

#[test]
fn transport_round_trip_against_the_server() {
    let server = LoopbackServer::start();
    let a = register(&server, "alice");
    let grant = login(&server, &a);
    assert_eq!(grant.status, LoginStatus::Ok);
    let mut t = HttpTransport::connect(&server.base, grant.token).unwrap();
    let device = a.device;
    let v = Vault::new();

    assert_eq!(t.list_vaults().unwrap().len(), 0);
    assert_eq!(
        t.create_vault(
            v.id,
            KeyGeneration::FIRST,
            &fake_wrapped(1),
            &v.meta(),
            device
        )
        .unwrap(),
        VaultCreateResult::Created
    );
    assert_eq!(
        t.create_vault(
            v.id,
            KeyGeneration::FIRST,
            &fake_wrapped(1),
            &v.meta(),
            device
        )
        .unwrap(),
        VaultCreateResult::Exists
    );
    assert_eq!(t.list_vaults().unwrap().len(), 1);

    // Records: new, then conflicting, then a tombstone over the item.
    let ids: Vec<ItemId> = (0..5).map(|_| ItemId::new_v7().unwrap()).collect();
    let bodies: Vec<RecordBody> = ids.iter().map(|i| v.item(*i, "x")).collect();
    let batch: Vec<PushRecord<'_>> = ids
        .iter()
        .zip(&bodies)
        .map(|(id, body)| PushRecord {
            record_id: *id,
            body,
            base_seq: None,
        })
        .collect();
    let results = t.push_records(v.id, KeyGeneration::FIRST, &batch).unwrap();
    assert_eq!(results.len(), 5);
    let seqs: Vec<u64> = results
        .iter()
        .map(|r| match r {
            PushResult::Accepted { seq: Some(s) } => *s,
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(seqs.windows(2).all(|w| w[0] < w[1]));
    let again = t
        .push_records(v.id, KeyGeneration::FIRST, &batch[..1])
        .unwrap();
    assert!(matches!(again[0], PushResult::Conflict { current_seq } if current_seq == seqs[0]));
    let tomb = v.tombstone(ids[0]);
    let over = t
        .push_records(
            v.id,
            KeyGeneration::FIRST,
            &[PushRecord {
                record_id: ids[0],
                body: &tomb,
                base_seq: Some(seqs[0]),
            }],
        )
        .unwrap();
    assert!(matches!(over[0], PushResult::Accepted { seq: Some(_) }));
    assert_eq!(
        t.push_records(v.id, KeyGeneration::new(2).unwrap(), &batch[..1])
            .err(),
        Some(TransportError::GenerationMismatch {
            current: KeyGeneration::FIRST
        })
    );

    // Pages in order, keys and meta on the first, the cursor at the head.
    let first = t.pull(v.id, &Cursor::default(), 2).unwrap();
    assert!(first.more);
    assert_eq!(first.records.len(), 2);
    assert_eq!(first.keys.len(), 1);
    assert_eq!(first.metas.len(), 1);
    let mut cursor = first.next;
    let mut count = first.records.len();
    loop {
        let page = t.pull(v.id, &cursor, 2).unwrap();
        count += page.records.len();
        cursor = page.next;
        if !page.more {
            break;
        }
    }
    assert_eq!(count, 5);
    let idle = t.pull(v.id, &cursor, 10).unwrap();
    assert!(idle.records.is_empty() && idle.keys.is_empty() && idle.metas.is_empty());
    assert_eq!(idle.next, cursor);

    // Meta: a conflict, then accepted on top of what the server holds.
    let page = t.pull(v.id, &Cursor::default(), 500).unwrap();
    let meta_seq = page.metas[0].seq;
    assert!(matches!(
        t.push_meta(v.id, &v.meta(), None).unwrap(),
        PushResult::Conflict { .. }
    ));
    assert!(matches!(
        t.push_meta(v.id, &v.meta(), meta_seq).unwrap(),
        PushResult::Accepted { seq: Some(_) }
    ));

    // A key: the head moved, then accepted, then taken.
    let stale = Cursor(0u64.to_be_bytes().to_vec());
    assert!(matches!(
        t.push_vault_key(
            v.id,
            KeyGeneration::new(2).unwrap(),
            &fake_wrapped(2),
            device,
            Some(&stale)
        )
        .unwrap(),
        KeyPushResult::HeadMoved { .. }
    ));
    let head = t.pull(v.id, &Cursor::default(), 500).unwrap().next;
    assert!(matches!(
        t.push_vault_key(
            v.id,
            KeyGeneration::new(2).unwrap(),
            &fake_wrapped(2),
            device,
            Some(&head)
        )
        .unwrap(),
        KeyPushResult::Accepted { seq: Some(_) }
    ));
    assert!(matches!(
        t.push_vault_key(v.id, KeyGeneration::new(2).unwrap(), &fake_wrapped(3), device, None)
            .unwrap(),
        KeyPushResult::GenerationTaken { current } if current.get() == 2
    ));

    // Attachments.
    let item = ItemId::new_v7().unwrap();
    let att = AttachmentId::new_v7().unwrap();
    let fk = AttachmentKey::generate().unwrap();
    let ct = mosaic_core::seal_attachment(&fk, &v.id, &item, &att, &[7u8; 5000]).unwrap();
    assert_eq!(t.get_attachment(v.id, att).unwrap(), None);
    t.put_attachment(v.id, att, &ct).unwrap();
    t.put_attachment(v.id, att, &ct).unwrap();
    assert_eq!(
        t.get_attachment(v.id, att).unwrap().as_deref(),
        Some(ct.as_slice())
    );
    let listed = t.attachments(v.id).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].deleted_at_ms, None);
    t.delete_attachment(v.id, att).unwrap();
    assert!(t.attachments(v.id).unwrap()[0].deleted_at_ms.is_some());
    assert!(t.restore_attachment(v.id, att).unwrap());
    assert!(
        !t.restore_attachment(v.id, AttachmentId::new_v7().unwrap())
            .unwrap()
    );

    // Devices and the change counter.
    let devices = t.devices().unwrap();
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].device_id, device);
    let first = t.wait_for_change(None, Duration::from_secs(5)).unwrap();
    t.put_attachment(v.id, AttachmentId::new_v7().unwrap(), &ct)
        .unwrap();
    let second = t
        .wait_for_change(Some(&first), Duration::from_secs(5))
        .unwrap();
    assert_ne!(first, second);

    // A second login of the device ends the first session; logging out ends
    // the second.
    let second = login(&server, &a);
    assert_eq!(t.list_vaults().err(), Some(TransportError::Unauthorized));
    let mut t2 = HttpTransport::connect(&server.base, second.token).unwrap();
    assert!(t2.list_vaults().is_ok());
}

#[test]
fn login_failures_and_totp_flow() {
    use mosaic_server::Clock;
    use mosaic_totp::OtpAuth;

    let server = LoopbackServer::start();
    let a = register(&server, "bob");

    // A wrong password and an unknown email both end as LoginFailed.
    server.advance_ms(60_000);
    let wrong = mosaic_core::derive_password_keys(
        &MasterPassword::new("not the password".to_owned()),
        &a.secret_key,
        &mosaic_core::PasswordParams {
            account_id: a.session.account_id(),
            salt: mosaic_core::AccountSalt::from_bytes([0; 16]),
            profile: Argon2Profile::Baseline,
        },
    )
    .unwrap();
    for email in [a.email.as_str(), "nobody@example.com"] {
        let r = hosted::login(
            &server.base,
            email,
            a.session.account_id(),
            &wrong.opaque_password,
            a.device,
            "x",
        );
        assert_eq!(
            r.err(),
            Some(TransportError::Rejected(Rejection::LoginFailed))
        );
    }

    // Enable TOTP with the raw API (the client functions for it come with
    // the desktop's settings page).
    let grant = login(&server, &a);
    // The test's own client (the transport configures its own TLS).
    let _ = rustls::crypto::ring::default_provider().install_default();
    let client = reqwest::blocking::Client::builder()
        .no_proxy()
        .build()
        .unwrap();
    let bearer = |t: &SessionToken| {
        use base64ct::{Base64UrlUnpadded, Encoding};
        format!("Bearer {}", Base64UrlUnpadded::encode_string(t.expose()))
    };
    let setup = client
        .post(format!("{}/v1/account/totp/setup", server.base))
        .header("authorization", bearer(&grant.token))
        .send()
        .unwrap()
        .text()
        .unwrap();
    let setup: serde_json::Value = serde_json::from_str(&setup).unwrap();
    let uri = setup["otpauthUri"].as_str().unwrap().to_owned();
    let otp = OtpAuth::parse(&uri).unwrap();
    let code_now = || {
        otp.otp
            .code_at(server.clock.now_ms() / 1000)
            .unwrap()
            .expose_secret()
            .to_owned()
    };
    let confirm = client
        .post(format!("{}/v1/account/totp/confirm", server.base))
        .header("authorization", bearer(&grant.token))
        .header("content-type", "application/json")
        .body(format!("{{\"code\":\"{}\"}}", code_now()))
        .send()
        .unwrap();
    assert!(confirm.status().is_success());

    // The next login is pending until the code is given; a pending session
    // is refused on the sync routes.
    server.advance_ms(60_000);
    let grant = login(&server, &a);
    assert_eq!(grant.status, LoginStatus::TwoFactorRequired);
    let refused = client
        .get(format!("{}/v1/vaults", server.base))
        .header("authorization", bearer(&grant.token))
        .send()
        .unwrap();
    assert_eq!(refused.status().as_u16(), 401);
    assert_eq!(
        hosted::login_totp(&server.base, &grant.token, "000000").err(),
        Some(TransportError::Rejected(Rejection::TwoFactorInvalid))
    );
    // The code of the step that enabled 2FA is spent; the next step's is not.
    server.advance_ms(30_000);
    hosted::login_totp(&server.base, &grant.token, &code_now()).unwrap();
    let mut done = HttpTransport::connect(&server.base, grant.token).unwrap();
    assert_eq!(done.list_vaults().unwrap().len(), 0);
}

#[test]
fn join_new_device_over_the_wire() {
    let server = LoopbackServer::start();
    let a = register(&server, "carol");
    let _ = login(&server, &a);

    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path().join("b.db")).unwrap();
    let device_b = store.install_id().unwrap();
    server.advance_ms(60_000);
    let joined = hosted::join_login(
        &server.base,
        &a.email,
        &MasterPassword::new(PASSWORD.to_owned()),
        &a.secret_key,
        device_b,
        "Second device",
    )
    .unwrap();
    assert_eq!(joined.grant.status, LoginStatus::Ok);
    let account =
        hosted::join_fetch_account(&server.base, &joined.grant.token, &joined.keys).unwrap();
    assert!(!account.totp_enabled);
    let b = LockedSession::open(store)
        .join_account_with_keys(&account.wrapped_ak, joined.keys)
        .map_err(|e| e.error)
        .unwrap();
    assert_eq!(b.account_id(), a.session.account_id());

    // A wrong Secret Key fails at the login, as a wrong password.
    server.advance_ms(60_000);
    let other_key = mosaic_core::SecretKey::generate().unwrap();
    let r = hosted::join_login(
        &server.base,
        &a.email,
        &MasterPassword::new(PASSWORD.to_owned()),
        &other_key,
        InstallId::new_v7().unwrap(),
        "x",
    );
    assert_eq!(
        r.err(),
        Some(TransportError::Rejected(Rejection::LoginFailed))
    );
}
