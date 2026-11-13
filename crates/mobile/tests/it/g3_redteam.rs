//! M10-G3 red team (Android): the autofill field and target decisions, the
//! link screen's asset-links status, and the Credential Manager passkey path.
//! A hole found here was `#[ignore = "M10-G3-<n>: ..."]` until its fix; the
//! other tests pin defenses that held.

use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

use mosaic_mobile::{
    AndroidCaller, AndroidPasskeyRequest, AssetLinksStatus, Attr, FieldNode, FieldRefusal,
    FillTarget, FillTargetKind, ItemKey, LoginDraft, MobileError, OtpEdit, UrlDraft,
};
use mosaic_passkey::b64url;
use mosaic_sync::TransportError;
use serde_json::Value;

use crate::common::Phone;

const CERT: [u8; 32] = [0x11; 32];
const ATTACKER_CERT: [u8; 32] = [0xa7; 32];
const PACKAGE: &str = "com.example.app";
const CHALLENGE: [u8; 32] = [7; 32];
const USER_HANDLE: [u8; 8] = [4; 8];
const DAY_MS: i64 = 24 * 60 * 60 * 1000;

fn url(u: &str, allow_http: bool) -> UrlDraft {
    UrlDraft {
        url: u.to_owned(),
        match_etld1: false,
        allow_http,
    }
}

fn login(phone: &Phone, title: &str, urls: Vec<UrlDraft>) -> ItemKey {
    phone
        .core
        .save_login(LoginDraft {
            vault_id: phone.vault_id(),
            item_id: None,
            title: title.to_owned(),
            username: "alice".to_owned(),
            password: Some(b"very-secret-Passw0rd".to_vec()),
            urls,
            notes: None,
            otp: OtpEdit::Keep,
            favorite: false,
        })
        .unwrap()
}

fn web(origin: &str) -> FillTarget {
    FillTarget {
        kind: FillTargetKind::Web {
            origin: origin.to_owned(),
        },
        display: String::new(),
        warning: None,
    }
}

fn app_target(package: &str, cert: [u8; 32]) -> FillTarget {
    FillTarget {
        kind: FillTargetKind::AndroidApp {
            package: package.to_owned(),
            certs: vec![cert.to_vec()],
        },
        display: String::new(),
        warning: None,
    }
}

fn caller(package: &str, cert: [u8; 32]) -> AndroidCaller {
    AndroidCaller {
        package: package.to_owned(),
        current_signers: vec![cert.to_vec()],
        web_domain: None,
        web_scheme: None,
    }
}

/// The first browser of Rust's own allowlist, with its real certificate.
fn browser() -> AndroidCaller {
    let phone = Phone::new();
    let list: Value = serde_json::from_str(&phone.core.privileged_browsers_json()).unwrap();
    let info = &list["apps"][0]["info"];
    let cert: Vec<u8> = info["signatures"][0]["cert_fingerprint_sha256"]
        .as_str()
        .unwrap()
        .split(':')
        .map(|h| u8::from_str_radix(h, 16).unwrap())
        .collect();
    AndroidCaller {
        package: info["package_name"].as_str().unwrap().to_owned(),
        current_signers: vec![cert],
        web_domain: None,
        web_scheme: None,
    }
}

/// A field node. `html` marks web content (an HTML `input` with an
/// `autocomplete` attribute), which a `WebView` reports whatever its domain.
fn node(id: u32, domain: Option<&str>, role: &str, html: bool, focused: bool) -> FieldNode {
    FieldNode {
        id,
        focused,
        visible: true,
        web_domain: domain.map(str::to_owned),
        input_type: 1,
        autofill_hints: if html { vec![] } else { vec![role.to_owned()] },
        html_tag: html.then(|| "input".to_owned()),
        html_attrs: if html {
            vec![Attr {
                name: "autocomplete".to_owned(),
                value: if role == "password" {
                    "current-password".to_owned()
                } else {
                    role.to_owned()
                },
            }]
        } else {
            vec![]
        },
        id_entry: None,
        hint: None,
    }
}

// ---- M10-G3-1: the page root is the first fillable node, not the target --------------------

/// Attack (M10-G3-1): a hostile page `evil.example` puts a cross-origin
/// `bank.example` frame (its real login page) before its own fields. The Kotlin
/// reader takes the page root from the first **fillable** node (the container
/// that carries the top document's domain takes no value and is dropped), and
/// the core starts a web selection at the first node with a domain, whatever
/// the target says. So the frame becomes "the page", and a bank login is
/// offered and filled into a cross-origin frame (brief §4: no cross-origin
/// iframe fills). Defense wanted: the selection of a web target is rooted at
/// the target's own host; a node of any other domain, or of none, is foreign,
/// and a focus in one refuses the request.
#[test]
fn g3_1_web_fields_are_rooted_at_the_target_host() {
    let (phone, _) = Phone::with_account();
    // The target is the top document; the bank frame comes first in the tree.
    let selection = phone.core.select_fill_fields(
        vec![
            node(1, Some("bank.example"), "username", false, true),
            node(2, Some("bank.example"), "password", false, false),
            node(3, Some("evil.example"), "username", false, false),
        ],
        &web("https://evil.example"),
    );
    assert_eq!(
        selection.refused,
        Some(FieldRefusal::ForeignFrame),
        "a focus in a frame that is not the target must refuse the fill"
    );
    assert_eq!(selection.username, None);
    assert_eq!(selection.password, None);
    // A web target over a tree without any web node (the browser's own screen)
    // has no page field to fill.
    let selection = phone.core.select_fill_fields(
        vec![
            node(1, None, "username", false, false),
            node(2, None, "password", false, false),
        ],
        &web("https://bank.example"),
    );
    assert_eq!(selection.username, None, "a native field is not the page");
    assert_eq!(selection.password, None);
}

// ---- M10-G3-2: WebView content without a webDomain counts as native ------------------------

/// Attack (M10-G3-2): an app the user linked (a mail app, say) shows
/// attacker HTML in a `WebView` loaded without a base URL (`loadData`,
/// `about:blank`, `srcdoc`), so its nodes report no `webDomain`. The N1 fix
/// excludes web content only by its domain, so the phishing form looks native:
/// the app's login is offered in it and filled, and a plain HTML form submit
/// sends it to the attacker (no script needed). Defense wanted: a node with
/// HTML info is web content in an app target, whatever its domain, and a focus
/// in one refuses the fill.
#[test]
fn g3_2_webview_html_without_a_domain_is_never_an_apps_field() {
    let (phone, _) = Phone::with_account();
    let app = app_target(PACKAGE, CERT);
    let selection = phone.core.select_fill_fields(
        vec![
            node(1, None, "username", true, false),
            node(2, None, "password", true, false),
        ],
        &app,
    );
    assert_eq!(selection.username, None, "HTML is not a native field");
    assert_eq!(selection.password, None, "HTML is not a native field");
    let selection = phone.core.select_fill_fields(
        vec![
            node(1, None, "username", false, false),
            node(2, None, "password", false, false),
            node(3, None, "password", true, true),
        ],
        &app,
    );
    assert_eq!(selection.refused, Some(FieldRefusal::ForeignFrame));
}

// ---- M10-G3-3: "Verified by <primary host>" for another host's file ------------------------

fn assetlinks(package: &str, cert: &[u8; 32]) -> Vec<u8> {
    let fingerprint = cert
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":");
    format!(
        "[{{\"relation\":[\"delegate_permission/common.get_login_creds\"],\
         \"target\":{{\"namespace\":\"android_app\",\"package_name\":\"{package}\",\
         \"sha256_cert_fingerprints\":[\"{fingerprint}\"]}}}}]"
    )
    .into_bytes()
}

/// Attack (M10-G3-3): the bank login also carries a URL of a second domain
/// that the attacker controls (a lapsed domain, a shared-hosting subdomain, a
/// URL a shared-vault member added). Its `assetlinks.json` lists the
/// attacker's app. `asset_links_status` answers `Verified` when **any** https
/// URL of the login lists the app, and the link screen
/// (`LinkAppScreen.kt:161`) shows that as "Verified by <primary host>", the
/// bank. The one trust signal of the link decision names the wrong site.
/// Defense wanted (ADR-0092: "the asset-links status of each web domain"):
/// `Verified` only for the host the screen names, or the status carries the
/// verifying host.
#[test]
fn g3_3_link_status_is_verified_only_by_the_host_it_names() {
    let (phone, _) = Phone::android_with_account();
    let key = login(
        &phone,
        "Bank",
        vec![
            url("https://bank.example", false),
            url("https://helper.example", false),
        ],
    );
    let primary = phone
        .core
        .item(&key.vault_id, &key.item_id)
        .unwrap()
        .summary
        .primary_host;
    assert_eq!(primary.as_deref(), Some("bank.example"));
    phone.core.test_hooks().set_asset_links_fetch(|host| {
        if host == "helper.example" {
            Ok(assetlinks("com.attacker.app", &ATTACKER_CERT))
        } else {
            Err(TransportError::NotFound)
        }
    });
    let status = phone
        .core
        .asset_links_status(
            &key.vault_id,
            &key.item_id,
            caller("com.attacker.app", ATTACKER_CERT),
        )
        .unwrap();
    // The status names the host whose file lists the app, so the screen can
    // never say "Verified by bank.example" for helper.example's file.
    assert_eq!(
        status,
        AssetLinksStatus::Verified {
            host: "helper.example".to_owned()
        }
    );
}

// ---- M10-G3-4: loopback http is trusted on a phone -----------------------------------------

/// Attack (M10-G3-4): on a phone, any installed app can listen on
/// `127.0.0.1:<port>`. A hostile app serves a fake form on the port of a
/// login saved for `http://localhost:<port>` (a router, NAS or dev tool synced
/// from the desktop) and a page navigates the browser there. Loopback is exempt
/// from `allow_http` (ADR-0037, a desktop rule: same-user processes are out of
/// scope there), so Mosaic offers the login. Defense wanted on mobile: a
/// loopback `http` page needs `allow_http` like any other `http` page.
#[test]
fn g3_4_loopback_http_on_a_phone_needs_allow_http() {
    let (phone, _) = Phone::android_with_account();
    login(&phone, "NAS", vec![url("http://localhost:8080", false)]);
    let answer = phone.core.fill_candidates(web("http://localhost:8080"));
    assert_eq!(
        answer.map(|c| c.candidates.len()).err(),
        Some(MobileError::InsecureOrigin)
    );
}

// ---- M10-G3-5: a browser package is an app target with a foreign signer --------------------

/// Attack (M10-G3-5), end to end: `link_android_app` refuses browser packages
/// (M10-G2 H1), but an `androidapp://<browser>?cert=…` URL can still reach a
/// login by typing, import or sync. A browser package with that signer (a fake
/// where the browser is not installed, or a key the vendored list does not
/// know yet, R135) is an app target, its `webDomain` is ignored, and the login
/// is offered on every page it shows. Defense wanted: a browser package is
/// never an app target or an app match.
#[test]
fn g3_5_a_browser_package_is_never_an_app_login() {
    let (phone, _) = Phone::android_with_account();
    let chrome = browser();
    login(
        &phone,
        "Bank",
        vec![url(
            &format!(
                "androidapp://{}?cert={}",
                chrome.package,
                b64url(&ATTACKER_CERT)
            ),
            false,
        )],
    );
    let mut fake = caller(&chrome.package, ATTACKER_CERT);
    fake.web_domain = Some("anything.example".to_owned());
    if let Ok(target) = phone.core.fill_target_android(fake) {
        let found = phone.core.fill_candidates(target).unwrap();
        assert!(
            found.candidates.is_empty(),
            "a browser package must never fill as an app"
        );
    }
}

// ---- Credential Manager passkeys: defenses that held ---------------------------------------

fn get_json(rp_id: &str) -> String {
    format!(
        "{{\"challenge\":\"{}\",\"rpId\":\"{rp_id}\",\"allowCredentials\":[],\"userVerification\":\"required\"}}",
        b64url(&CHALLENGE)
    )
}

fn create_json(rp_id: &str) -> String {
    format!(
        "{{\"challenge\":\"{}\",\"rp\":{{\"id\":\"{rp_id}\",\"name\":\"Example\"}},\
         \"user\":{{\"id\":\"{}\",\"name\":\"alice@example.com\",\"displayName\":\"Alice\"}},\
         \"pubKeyCredParams\":[{{\"type\":\"public-key\",\"alg\":-7}}],\
         \"authenticatorSelection\":{{\"residentKey\":\"required\"}}}}",
        b64url(&CHALLENGE),
        b64url(&USER_HANDLE)
    )
}

fn request(json: String, caller: AndroidCaller) -> AndroidPasskeyRequest {
    AndroidPasskeyRequest {
        request_json: json,
        caller,
        platform_origin: None,
        client_data_hash: None,
    }
}

fn browser_request(json: String, origin: &str) -> AndroidPasskeyRequest {
    AndroidPasskeyRequest {
        request_json: json,
        caller: browser(),
        platform_origin: Some(origin.to_owned()),
        client_data_hash: Some(vec![0x33; 32]),
    }
}

/// Stores a passkey for `rp` through a privileged browser on that origin.
fn register_in_browser(phone: &Phone, rp: &str) {
    let session = phone
        .core
        .passkey_android_create_begin(browser_request(create_json(rp), &format!("https://{rp}")))
        .unwrap();
    phone
        .core
        .passkey_android_create_finish(session.session_id, phone.vault_id(), true)
        .unwrap();
}

/// Attack: an app without asset links (or with a file that lists another app,
/// an empty file, a redirect) asks the passkey of `bank.example`. Defense: the
/// file is read from the rp ID itself, and nothing but a listing of this app and
/// signer opens a ceremony; a failed read is "could not check", never verified.
#[test]
fn g3_a_bank_passkey_is_never_offered_to_an_app_without_asset_links() {
    let (phone, _) = Phone::android_with_account();
    register_in_browser(&phone, "bank.example");
    let in_browser = phone
        .core
        .passkey_android_get_begin(browser_request(
            get_json("bank.example"),
            "https://bank.example",
        ))
        .unwrap();
    assert_eq!(in_browser.candidates.len(), 1);

    let hosts = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let ask = |answer: fn() -> Result<Vec<u8>, TransportError>| {
        let seen = hosts.clone();
        phone.core.test_hooks().set_asset_links_fetch(move |host| {
            seen.lock().unwrap().push(host.to_owned());
            answer()
        });
        phone
            .core
            .passkey_android_get_begin(request(
                get_json("bank.example"),
                caller("com.attacker.app", ATTACKER_CERT),
            ))
            .err()
    };
    assert_eq!(
        ask(|| Err(TransportError::NotFound)),
        Some(MobileError::AppNotVerified)
    );
    assert_eq!(
        ask(|| Ok(b"[]".to_vec())),
        Some(MobileError::AppNotVerified)
    );
    assert_eq!(
        ask(|| Ok(assetlinks("com.bank.app", &CERT))),
        Some(MobileError::AppNotVerified)
    );
    // The attacker's package under the bank's key, and the bank's package
    // under the attacker's key.
    assert_eq!(
        ask(|| Ok(assetlinks("com.attacker.app", &CERT))),
        Some(MobileError::AppNotVerified)
    );
    // A redirect (or any status but 200 and 404) is a protocol error.
    assert_eq!(
        ask(|| Err(TransportError::Protocol)),
        Some(MobileError::AssetLinksUnavailable)
    );
    assert!(hosts.lock().unwrap().iter().all(|h| h == "bank.example"));
    assert_eq!(hosts.lock().unwrap().len(), 5);
}

/// Attack: the asset-links answer as a cache to poison or outlive. Defense: a
/// positive answer lives 24 hours (the residual: a site that withdraws an app
/// is obeyed within a day), a negative one too, and a failed read is never
/// cached, so a network error cannot lock in either answer.
#[test]
fn g3_asset_links_answers_expire_and_failures_are_not_cached() {
    let (phone, _) = Phone::android_with_account();
    let mode = Arc::new(AtomicU8::new(0));
    let calls = Arc::new(AtomicUsize::new(0));
    let (m, c) = (mode.clone(), calls.clone());
    phone.core.test_hooks().set_asset_links_fetch(move |_| {
        c.fetch_add(1, Ordering::SeqCst);
        match m.load(Ordering::SeqCst) {
            0 => Ok(assetlinks(PACKAGE, &CERT)),
            1 => Ok(b"[]".to_vec()),
            _ => Err(TransportError::Unreachable),
        }
    });
    let begin = || {
        phone
            .core
            .passkey_android_get_begin(request(get_json("example.com"), caller(PACKAGE, CERT)))
            .map(|_| ())
    };
    assert_eq!(begin(), Ok(()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // The site withdraws the app: the cached answer still holds within a day.
    mode.store(1, Ordering::SeqCst);
    assert_eq!(begin(), Ok(()));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    phone.core.test_hooks().set_clock_offset_ms(DAY_MS + 1_000);
    assert_eq!(begin(), Err(MobileError::AppNotVerified));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    // No network after the negative answer ran out: "could not check".
    mode.store(2, Ordering::SeqCst);
    phone
        .core
        .test_hooks()
        .set_clock_offset_ms(2 * DAY_MS + 2_000);
    assert_eq!(begin(), Err(MobileError::AssetLinksUnavailable));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    // The failure was not cached: the next request reads the file again.
    mode.store(0, Ordering::SeqCst);
    assert_eq!(begin(), Ok(()));
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

/// Attack: an app asks for an rp ID that is not one registrable site, to reach
/// a public suffix, a local host or a path on someone else's server. Defense:
/// refused before any fetch.
#[test]
fn g3_rp_id_tricks_from_an_app_are_refused_before_any_fetch() {
    let (phone, _) = Phone::android_with_account();
    let calls = Arc::new(AtomicUsize::new(0));
    let c = calls.clone();
    phone.core.test_hooks().set_asset_links_fetch(move |_| {
        c.fetch_add(1, Ordering::SeqCst);
        Ok(assetlinks(PACKAGE, &CERT))
    });
    for rp in [
        "bank.example:443",
        "github.io",
        "com",
        "localhost",
        "127.0.0.1",
        "bank.example/",
        "bank.example/x?y",
        "user@bank.example",
        "b\u{fc}cher.example",
        ".bank.example",
        "",
    ] {
        assert_eq!(
            phone
                .core
                .passkey_android_get_begin(request(get_json(rp), caller(PACKAGE, CERT)))
                .err(),
            Some(MobileError::RpIdInvalid),
            "{rp:?}"
        );
    }
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

/// Attack: a privileged browser (or an app with its package) drops the origin
/// to sign a client-data hash of its choosing, or to use the app path for a
/// site that delegates only to its own app. Defense: a hash needs a checked
/// browser origin; without an origin a browser is an app like any other.
#[test]
fn g3_a_browser_cannot_drop_its_origin_to_sign_a_hash_or_act_as_the_sites_app() {
    let (phone, _) = Phone::android_with_account();
    phone
        .core
        .test_hooks()
        .set_asset_links_fetch(|_| Ok(assetlinks("com.bank.app", &CERT)));
    let mut hashed = request(get_json("bank.example"), browser());
    hashed.client_data_hash = Some(vec![0x33; 32]);
    assert_eq!(
        phone.core.passkey_android_get_begin(hashed).err(),
        Some(MobileError::InvalidInput)
    );
    assert_eq!(
        phone
            .core
            .passkey_android_get_begin(request(get_json("bank.example"), browser()))
            .err(),
        Some(MobileError::AppNotVerified)
    );
    // The browser's package with another key is no browser.
    let mut fake = browser_request(get_json("bank.example"), "https://bank.example");
    fake.caller.current_signers = vec![ATTACKER_CERT.to_vec()];
    assert_eq!(
        phone.core.passkey_android_get_begin(fake).err(),
        Some(MobileError::AppNotVerified)
    );
}

/// Attack: replay an entry's ceremony (a second finish, a key of another rp,
/// a create session in a get finish). Defense: a ceremony is taken once, only
/// for the passkeys it listed.
#[test]
fn g3_a_passkey_ceremony_is_single_use_and_bound_to_its_candidates() {
    let (phone, _) = Phone::android_with_account();
    register_in_browser(&phone, "bank.example");
    register_in_browser(&phone, "other.example");
    let begin = |rp: &str| {
        phone
            .core
            .passkey_android_get_begin(browser_request(get_json(rp), &format!("https://{rp}")))
            .unwrap()
    };
    let other_key = begin("other.example").candidates[0].key.clone();
    let session = begin("bank.example");
    let bank_key = session.candidates[0].key.clone();
    phone
        .core
        .passkey_android_get_finish(session.session_id.clone(), bank_key.clone(), true)
        .unwrap();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(session.session_id, bank_key.clone(), true)
            .err(),
        Some(MobileError::PasskeySessionExpired)
    );
    let session = begin("bank.example");
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(session.session_id.clone(), other_key, true)
            .err(),
        Some(MobileError::PasskeyNotFound)
    );
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(session.session_id, bank_key.clone(), true)
            .err(),
        Some(MobileError::PasskeySessionExpired)
    );
    let create = phone
        .core
        .passkey_android_create_begin(browser_request(
            create_json("bank.example"),
            "https://bank.example",
        ))
        .unwrap();
    assert_eq!(
        phone
            .core
            .passkey_android_get_finish(create.session_id, bank_key, true)
            .err(),
        Some(MobileError::InvalidInput)
    );
}

/// Info (M10-G3): the list built for the system picker opens a fill session per
/// request and never fills from it. `fill_release` forgets it at once, and a
/// release of an unknown session is harmless.
#[test]
fn a_released_fill_session_is_gone() {
    let (phone, _) = Phone::android_with_account();
    let key = login(&phone, "Bank", vec![url("https://bank.example", false)]);
    let found = phone
        .core
        .fill_candidates(web("https://bank.example"))
        .unwrap();
    phone.core.fill_release(found.session_id.clone());
    phone.core.fill_release("unknown".to_owned());
    assert_eq!(
        phone.core.fill_values(found.session_id, key).err(),
        Some(MobileError::NotFound)
    );
}
