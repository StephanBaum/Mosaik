//! M10-G3 red team (Android): hostile `webDomain`s and schemes from a trusted
//! browser, look-alike browser packages, tree bounds, asset-links strictness and
//! rp-ID tricks. Every test states the attack; a hole found here is
//! `#[ignore = "M10-G3-<n>: ..."]` until the fix round removes the attribute.

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::{LoginUrl, UrlMatch};

use crate::android_browsers::{first_cert, is_browser_package};
use crate::app::app_fill_allowed;
use crate::assetlinks::{AssetLinksError, Relation, app_is_delegated, parse_asset_links};
use crate::fields::{FieldNode, FieldScope, InputKind, MAX_NODES, select_scoped_fill_fields};
use crate::target::{AndroidCaller, FillTarget, TargetError, android_fill_target};
use crate::{Refusal, canonical_rp_id, fill_allowed};

const ATTACKER_CERT: [u8; 32] = [0xa7; 32];

fn chrome_cert() -> [u8; 32] {
    first_cert("com.android.chrome").unwrap()
}

fn browser_target(domain: &str, scheme: Option<&str>) -> Result<FillTarget, TargetError> {
    let certs = [chrome_cert()];
    android_fill_target(&AndroidCaller {
        package: "com.android.chrome",
        certs: &certs,
        web_domain: Some(domain),
        web_scheme: scheme,
    })
}

fn web_origin(target: &FillTarget) -> String {
    match target {
        FillTarget::Web { origin, .. } => origin.serialize(),
        FillTarget::AndroidApp { .. } => panic!("expected a web target"),
    }
}

/// Attack: a page (or a confused browser) reports a `webDomain` that a lenient
/// parser would read as another host: userinfo, a path, a backslash, a fragment,
/// a default port, whitespace, an integer IP, a Unicode host. Defense: only the
/// exact canonical origin is a target; anything else is refused, never re-read.
#[test]
fn g3_web_domain_tricks_from_a_trusted_browser_never_name_another_host() {
    for refused in [
        "bank.example@evil.example",
        "evil.example/bank.example",
        "evil.example\\bank.example",
        "evil.example#bank.example",
        "evil.example?bank.example",
        "bank.example:443",
        "bank.example:",
        "bank.example:99999",
        " bank.example",
        "bank.example ",
        "bank\t.example",
        "bank\n.example",
        "",
        "0x7f.1",
        "2130706433",
        "b\u{fc}cher.example",
        "[::1",
        "javascript:alert(1)",
        "data:text/html,x",
    ] {
        assert!(
            browser_target(refused, Some("https")).is_err(),
            "{refused:?} must not become a target"
        );
    }
    // What is accepted is exactly the host the browser named (lowercased).
    for (domain, origin) in [
        ("Bank.Example", "https://bank.example"),
        ("bank.example:8443", "https://bank.example:8443"),
        ("127.0.0.1", "https://127.0.0.1"),
        ("bank.example.", "https://bank.example."),
    ] {
        assert_eq!(
            web_origin(&browser_target(domain, Some("https")).unwrap()),
            origin
        );
    }
    // A trailing dot is another origin: it never matches the saved site, not
    // even with the eTLD+1 opt-in.
    let dotted = browser_target("bank.example.", Some("https")).unwrap();
    let FillTarget::Web { origin, .. } = &dotted else {
        panic!("expected a web target");
    };
    let mut saved = LoginUrl::new("https://bank.example");
    assert_eq!(fill_allowed(&saved, origin), Err(Refusal::NoMatch));
    saved.match_mode = UrlMatch::Etld1;
    assert_eq!(fill_allowed(&saved, origin), Err(Refusal::NoMatch));
    // Punycode is accepted and carries the warning.
    let FillTarget::Web { idn_unicode, .. } =
        browser_target("xn--bcher-kva.example", Some("https")).unwrap()
    else {
        panic!("expected a web target");
    };
    assert_eq!(idn_unicode.as_deref(), Some("b\u{fc}cher.example"));
}

/// Attack: a `webScheme` that is not `http` or `https` (`javascript`, `data`,
/// `file`, an upper-case or padded `https`). Defense: refused as a scheme.
#[test]
fn g3_web_scheme_tricks_are_refused() {
    for scheme in [
        "javascript",
        "data",
        "file",
        "about",
        "blob",
        "chrome",
        "intent",
        "content",
        "android-app",
        "HTTPS",
        "Http",
        "https ",
        " https",
        "",
        "wss",
        "ftp",
    ] {
        assert_eq!(
            browser_target("bank.example", Some(scheme)),
            Err(TargetError::Scheme),
            "{scheme:?}"
        );
    }
}

/// Attack: an app whose package imitates an allowlisted browser (a suffix, a
/// near miss, upper case, a homoglyph, a zero-width space) and even presents
/// the browser's real certificate hash. Defense: only the exact package with an
/// allowlisted signer is a browser; the rest are apps, or not a package at all,
/// and their `webDomain` is ignored.
#[test]
fn g3_look_alike_browser_packages_are_apps_and_never_web_targets() {
    let real = [chrome_cert()];
    for package in [
        "com.android.chrome.evil",
        "com.android.chromf",
        "COM.ANDROID.CHROME",
        "com.android.chrome_",
        "com.android",
    ] {
        let target = android_fill_target(&AndroidCaller {
            package,
            certs: &real,
            web_domain: Some("bank.example"),
            web_scheme: None,
        })
        .unwrap();
        assert!(
            matches!(target, FillTarget::AndroidApp { .. }),
            "{package} must stay an app"
        );
        assert!(!is_browser_package(package), "{package}");
    }
    for package in [
        "com.android.chrome ",
        "com.android.chrom\u{435}",
        "com.android.chrome\u{200b}",
        "com.android.chrome/evil",
    ] {
        assert_eq!(
            android_fill_target(&AndroidCaller {
                package,
                certs: &real,
                web_domain: Some("bank.example"),
                web_scheme: None,
            }),
            Err(TargetError::BadPackage),
            "{package:?}"
        );
    }
}

/// Attack (M10-G3-5): the real browser package with a foreign signer (a
/// sideloaded fake where the browser is not installed, or a rotated key the
/// vendored list does not know yet, R135) meets a login that carries an
/// `androidapp://<browser>?cert=…` URL. That URL cannot be made by
/// `link_android_app`, which refuses browsers, but it can be typed, imported or
/// synced. Defense wanted: a browser package is never an app login target
/// (`is_browser_package`'s own contract), so the "app" would get the login on
/// every page it shows (the M10-G2 H1 hole through another door).
#[test]
fn g3_browser_package_with_a_foreign_signer_never_matches_an_app_url() {
    let url = LoginUrl::new(&format!(
        "androidapp://com.android.chrome?cert={}",
        Base64UrlUnpadded::encode_string(&ATTACKER_CERT)
    ));
    let certs = [ATTACKER_CERT];
    match android_fill_target(&AndroidCaller {
        package: "com.android.chrome",
        certs: &certs,
        web_domain: Some("bank.example"),
        web_scheme: None,
    }) {
        Err(_) => {}
        Ok(FillTarget::AndroidApp { package, certs }) => {
            assert_eq!(
                app_fill_allowed(&url, &package, &certs),
                Err(Refusal::NoMatch),
                "a browser package must never fill as an app"
            );
        }
        Ok(FillTarget::Web { .. }) => panic!("a foreign signer is no browser"),
    }
}

fn field(domain: Option<&str>, hint: &str) -> FieldNode {
    FieldNode {
        web_domain: domain.map(str::to_owned),
        is_web: false,
        hints: vec![hint.to_owned()],
        autocomplete: None,
        input_kind: InputKind::Text,
        id_entry: None,
        visible: true,
        focused: false,
    }
}

/// Attack: a huge tree pushes a field (or a focused foreign field) past the cap.
/// Defense: nothing past [`MAX_NODES`] is ever selected (fewer fills, never
/// more), and fields of a foreign frame are never selected wherever the focus is.
#[test]
fn g3_nodes_past_the_cap_are_never_selected() {
    let mut nodes: Vec<FieldNode> = (0..MAX_NODES)
        .map(|_| field(Some("bank.example"), "nothing"))
        .collect();
    nodes.push(field(Some("bank.example"), "password"));
    let mut foreign = field(Some("ads.example"), "password");
    foreign.focused = true;
    nodes.push(foreign);
    for scope in [
        FieldScope::Web {
            host: "bank.example",
        },
        FieldScope::App,
    ] {
        let selection = select_scoped_fill_fields(&nodes, scope);
        assert!(selection.fields.is_empty(), "{scope:?}");
        assert!(selection.fields.iter().all(|f| f.index < MAX_NODES));
    }
}

/// Attack: the focus is outside the reported tree (no node is focused), with a
/// foreign frame next to the page form. Defense: only the page frame's fields
/// are selected; the foreign frame gets nothing.
#[test]
fn g3_no_focus_in_the_tree_selects_only_the_page_frame() {
    let nodes = vec![
        field(Some("bank.example"), "username"),
        field(Some("bank.example"), "password"),
        field(Some("ads.example"), "username"),
        field(Some("ads.example"), "password"),
    ];
    let selection = select_scoped_fill_fields(
        &nodes,
        FieldScope::Web {
            host: "bank.example",
        },
    );
    assert_eq!(selection.refused, None);
    let indexes: Vec<usize> = selection.fields.iter().map(|f| f.index).collect();
    assert_eq!(indexes, vec![0, 1]);
}

fn statement(relation: &str, package: &str, fingerprint: &str) -> String {
    format!(
        "{{\"relation\":[\"{relation}\"],\"target\":{{\"namespace\":\"android_app\",\
         \"package_name\":\"{package}\",\"sha256_cert_fingerprints\":[\"{fingerprint}\"]}}}}"
    )
}

fn colon_hex(cert: &[u8; 32]) -> String {
    cert.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

const LOGIN: &str = "delegate_permission/common.get_login_creds";

/// Attack: an asset-links file that almost names the app: a package prefix or
/// suffix, a fingerprint without colons, a SHA-1, padded, a relation in another
/// case, a `web` target. Defense: only the exact package and the full colon-hex
/// SHA-256 (case-insensitive) with the exact relation count.
#[test]
fn g3_asset_links_need_the_exact_package_fingerprint_and_relation() {
    let cert = [0x5c; 32];
    let hex = colon_hex(&cert);
    let delegated = |file: String, package: &str| {
        parse_asset_links(format!("[{file}]").as_bytes())
            .is_ok_and(|st| app_is_delegated(&st, package, &cert, Relation::GetLoginCreds))
    };
    assert!(delegated(
        statement(LOGIN, "com.bank.app", &hex),
        "com.bank.app"
    ));
    assert!(delegated(
        statement(LOGIN, "com.bank.app", &hex.to_lowercase()),
        "com.bank.app"
    ));
    for package in [
        "com.bank.app.evil",
        "com.bank",
        "com.bank.ap",
        "Com.bank.app",
    ] {
        assert!(
            !delegated(statement(LOGIN, "com.bank.app", &hex), package),
            "{package}"
        );
    }
    for fingerprint in [
        hex.replace(':', ""),
        format!(" {hex}"),
        format!("{hex} "),
        colon_hex(&cert)[..59].to_owned(),
        format!("{hex}:00"),
    ] {
        assert!(
            !delegated(
                statement(LOGIN, "com.bank.app", &fingerprint),
                "com.bank.app"
            ),
            "{fingerprint:?}"
        );
    }
    assert!(!delegated(
        statement(
            "Delegate_Permission/common.get_login_creds",
            "com.bank.app",
            &hex
        ),
        "com.bank.app"
    ));
    let web = "{\"relation\":[\"delegate_permission/common.get_login_creds\"],\
               \"target\":{\"namespace\":\"web\",\"site\":\"https://com.bank.app\"}}";
    assert!(!delegated(web.to_owned(), "com.bank.app"));
}

/// Attack: shapes a lenient parser might half-accept: an `include` statement, a
/// byte-order mark, trailing data, a relation that is a string. Defense: the
/// whole file is refused (nothing is delegated).
#[test]
fn g3_odd_asset_links_files_are_refused_whole() {
    let hex = colon_hex(&[0x5c; 32]);
    let good = statement(LOGIN, "com.bank.app", &hex);
    for file in [
        format!("[{good},{{\"include\":\"https://evil.example/.well-known/assetlinks.json\"}}]"),
        format!("\u{feff}[{good}]"),
        format!("[{good}] []"),
        format!("[{good}],"),
        good.replace(&format!("[\"{LOGIN}\"]"), &format!("\"{LOGIN}\"")),
        format!("{{\"statements\":[{good}]}}"),
    ] {
        assert!(
            matches!(
                parse_asset_links(file.as_bytes()),
                Err(AssetLinksError::Malformed | AssetLinksError::UnsupportedNamespace)
            ),
            "{file}"
        );
    }
}

/// Attack: an app asks a passkey for an rp ID that is not one registrable site:
/// with a port, a public suffix (also a private one such as `github.io`),
/// `localhost`, an IP, a path, userinfo, a wildcard, a Unicode host, an empty
/// label. Defense: no canonical rp ID, so no asset-links fetch and no ceremony.
#[test]
fn g3_rp_id_tricks_have_no_canonical_form() {
    for rp in [
        "bank.example:443",
        "github.io",
        "com",
        "co.uk",
        "localhost",
        "127.0.0.1",
        "[::1]",
        "bank.example/",
        "bank.example/.well-known",
        "user@bank.example",
        "b\u{fc}cher.example",
        "bank..example",
        ".bank.example",
        "bank.example..",
        "",
        " bank.example",
    ] {
        assert_eq!(canonical_rp_id(rp), None, "{rp:?}");
    }
    assert_eq!(
        canonical_rp_id("BANK.example.").as_deref(),
        Some("bank.example")
    );
    assert_eq!(
        canonical_rp_id("someone.github.io").as_deref(),
        Some("someone.github.io")
    );
}

/// Attack (M10-G3-6): rp IDs with characters no DNS host name has (`*`, `!`,
/// `$`, `(`, `'`, `=`, `~`). The WHATWG host parser accepts them, so
/// `canonical_rp_id` (the app path and imports) calls them canonical: an app can
/// make Mosaic ask for `https://*.bank.example/.well-known/assetlinks.json`, and
/// an import can store such an rp ID. Defense wanted: an rp ID is a DNS name
/// (letters, digits, hyphens, dots).
#[test]
fn g3_rp_id_must_be_a_dns_name() {
    for rp in [
        "*.bank.example",
        "bank!.example",
        "ba$nk.example",
        "bank(.example",
        "bank'.example",
        "bank=.example",
        "bank~.example",
    ] {
        assert_eq!(canonical_rp_id(rp), None, "{rp:?}");
    }
}
