//! What the server writes to disk and to its log (CRYPTO_SPEC §12): no client
//! IP address, no secret, no plaintext of a vault.

use std::path::Path;

use axum::http::{Method, StatusCode};
use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::{AttachmentId, ItemId};
use mosaic_server::ApiError;
use mosaic_sync::wire::json::RegistrationMode;

use crate::common::{LoginResult, TestApp, TestVault, User};

/// The peer of the test requests. The port is five digits that no timestamp or
/// duration in a log line or a database page is likely to contain, and it is
/// looked for only with its address or after a colon.
// The port lies below every ephemeral range (Linux 32768+, macOS and Windows 49152+): a real
// loopback socket of the test, logged as `127.0.0.1:<port>`, can never carry it by chance.
const PEER: &str = "203.0.113.77:21873";
const MARKER: &str = "PLAINTEXT-MARKER-4e1f9b2c-item-title";

pub(crate) fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// Like [`contains`], but the match must not continue with another digit (a whole number), and a
/// needle that starts with `:` (a port) must follow an address: a digit or `]`. A JSON field
/// such as `"elapsed":21873` is not the port.
fn contains_number(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && haystack.windows(needle.len()).enumerate().any(|(i, w)| {
            let after_address = needle[0] != b':'
                || i.checked_sub(1)
                    .and_then(|p| haystack.get(p))
                    .is_some_and(|b| b.is_ascii_digit() || *b == b']');
            w == needle
                && after_address
                && !haystack
                    .get(i + needle.len())
                    .is_some_and(u8::is_ascii_digit)
        })
}

/// Every file under `dir` that is a database file, its WAL or a blob.
pub(crate) fn stored_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                stack.push(e.path());
            } else {
                out.push((
                    e.path().display().to_string(),
                    std::fs::read(e.path()).unwrap(),
                ));
            }
        }
    }
    out
}

/// The detector finds a peer address in the places a log line or a page would
/// hold it, and does not fire on digits alone.
#[test]
fn detector_catches_a_logged_peer_address() {
    let needles: [&[u8]; 3] = [b"203.0.113.77", b"203.0.113.77:21873", b":21873"];
    for line in [
        &b"INFO request peer=203.0.113.77:21873 status=200"[..],
        b"connection from 203.0.113.77:21873",
        b"addr: [203.0.113.77]:21873",
    ] {
        assert!(
            needles.iter().any(|n| contains(line, n)),
            "{}",
            String::from_utf8_lossy(line)
        );
    }
    for line in [
        &b"ms=21873 route=/v1/login/start"[..],
        b"1800000021873 duration=5555",
    ] {
        assert!(needles.iter().all(|n| !contains(line, n)));
    }
    // The whole-number port check: after an address only, never inside a longer number.
    assert!(contains_number(b"peer=203.0.113.77:21873 ok", b":21873"));
    assert!(contains_number(b"addr: [::1]:21873", b":21873"));
    assert!(!contains_number(
        br#"{"elapsed":21873,"route":"/v1"}"#,
        b":21873"
    ));
    assert!(!contains_number(b"x 10.0.0.1:218734 y", b":21873"));
}

#[tokio::test(start_paused = true)]
async fn no_plaintext_and_no_ip_on_disk_or_in_logs() {
    let logs = crate::common::init_logs();
    let app = TestApp::with_file_db(RegistrationMode::Open);
    let u = User::new("marker@marker-domain-9d3c.example");
    let r = app.register_from(&u, None, PEER).await;
    assert_eq!(r.status, StatusCode::CREATED);
    let LoginResult::Ok { token, .. } = app
        .login_with(&u.email, &u.opw, u.account_id, u.device, PEER)
        .await
    else {
        panic!("login failed");
    };
    app.send(
        Method::POST,
        "/v1/prelogin",
        None,
        PEER,
        br#"{"email":"someone@else.example"}"#.to_vec(),
    )
    .await;

    let v = TestVault::new();
    assert_eq!(
        app.create_vault(&token, &v).await.status,
        StatusCode::CREATED
    );
    let item = ItemId::new_v7().unwrap();
    let r = app
        .push(&token, &v, vec![v.item_record(item, MARKER, None)])
        .await;
    assert_eq!(r.status, StatusCode::OK);
    let att = AttachmentId::new_v7().unwrap();
    let body = v.attachment(item, att, 2000);
    let r = app
        .send(
            Method::PUT,
            &format!("/v1/vaults/{}/attachments/{att}", v.id),
            Some(&token),
            PEER,
            body,
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
    app.send(Method::GET, "/v1/vaults", Some(&token), PEER, Vec::new())
        .await;

    let forbidden: Vec<(&str, Vec<u8>)> = vec![
        ("peer IP text", b"203.0.113.77".to_vec()),
        ("peer IP prefix", b"203.0.113.".to_vec()),
        ("peer IP octets", vec![203, 0, 113, 77]),
        ("peer address with port", b"203.0.113.77:21873".to_vec()),
        ("peer port after a colon", b":21873".to_vec()),
        ("item plaintext", MARKER.as_bytes().to_vec()),
        ("OPAQUE password", u.opw.to_vec()),
        ("session token", token.to_vec()),
        (
            "session token b64",
            Base64UrlUnpadded::encode_string(&token).into_bytes(),
        ),
    ];

    // The log: routes yes, user data no.
    let log = logs.contents();
    let text = String::from_utf8_lossy(&log).into_owned();
    assert!(
        text.contains("/v1/login/start") && text.contains("/v1/vaults/{vault_id}/records"),
        "{text}"
    );
    for (what, bytes) in &forbidden {
        // A port is a whole number: ":21873" inside a longer number (a duration such as
        // "elapsed":218734) is not the port, and matched it at random.
        let found = if bytes.ends_with(b"21873") {
            contains_number(&log, bytes)
        } else {
            contains(&log, bytes)
        };
        assert!(!found, "log contains the {what}");
    }
    for s in [
        u.email.as_str(),
        "marker-domain-9d3c",
        &v.id.to_string(),
        &u.account_id.to_string(),
        &u.device.to_string(),
        &item.to_string(),
        &att.to_string(),
    ] {
        assert!(!text.contains(s), "log contains {s}");
    }

    // Disk: the database file, its WAL and the blobs.
    let scan = |app: &TestApp| {
        let files = stored_bytes(app.dir.path());
        assert!(
            files.iter().any(|(n, _)| n.ends_with(".sqlite")),
            "the database file"
        );
        let mut email_seen = false;
        for (name, bytes) in &files {
            for (what, needle) in &forbidden {
                assert!(!contains(bytes, needle), "{name} contains the {what}");
            }
            email_seen |= contains(bytes, b"marker-domain-9d3c");
        }
        email_seen
    };
    // The email is stored (it is the login identifier); everything above is not.
    assert!(scan(&app), "the scan reads the data: the email is found");
    // After a checkpoint the pages are in the main file.
    app.state
        .db()
        .run(
            |c| {
                Ok::<_, ApiError>(c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?)
            },
        )
        .unwrap();
    assert!(scan(&app));
}
