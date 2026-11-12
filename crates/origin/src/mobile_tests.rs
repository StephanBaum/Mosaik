//! Tests of the M10 modules: app URLs, the browser allowlist, targets, field
//! selection and asset links.

use base64ct::{Base64UrlUnpadded, Encoding};
use mosaic_core::model::LoginUrl;

use crate::android_browsers::{
    first_cert, is_trusted_browser, package_count, privileged_browsers_json,
};
use crate::app::{AndroidAppUrl, AppUrlError, app_fill_allowed, is_valid_package_name};
use crate::assetlinks::{
    AssetLinksError, MAX_ASSET_LINKS_LEN, MAX_STATEMENTS, Relation, app_is_delegated,
    app_is_delegated_for_passkeys, parse_asset_links,
};
use crate::fields::{FieldNode, FieldRefusal, FieldRole, InputKind, select_fill_fields};
use crate::target::{
    AndroidCaller, FillTarget, IosIdentifier, TargetError, android_fill_target, ios_fill_target,
};
use crate::{MatchKind, PageOrigin, Refusal, fill_allowed};

const CERT: [u8; 32] = [7; 32];
const OTHER_CERT: [u8; 32] = [8; 32];

fn app_url(package: &str, cert: &[u8; 32]) -> LoginUrl {
    LoginUrl::new(&format!(
        "androidapp://{package}?cert={}",
        Base64UrlUnpadded::encode_string(cert)
    ))
}

#[test]
fn android_app_url_parses_package_and_cert() {
    let url = app_url("com.example.app", &CERT);
    let parsed = AndroidAppUrl::parse(url.url.as_str()).unwrap();
    assert_eq!(parsed.package(), "com.example.app");
    assert_eq!(parsed.cert_sha256(), Some(&CERT));
    assert_eq!(parsed.canonical(), url.url.as_str());
    let bare = AndroidAppUrl::parse("androidapp://com.example.app").unwrap();
    assert_eq!(bare.cert_sha256(), None);
    assert_eq!(bare.canonical(), "androidapp://com.example.app");
    // Wrong certificate shapes.
    for bad in [
        "androidapp://a.b?cert=",
        "androidapp://a.b?cert=AAAA",
        "androidapp://a.b?cert=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        "androidapp://a.b?cert=BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc&x=1",
        "androidapp://a.b?other=1",
    ] {
        assert!(AndroidAppUrl::parse(bad).is_err(), "{bad}");
    }
    assert_eq!(
        AndroidAppUrl::parse("https://example.com"),
        Err(AppUrlError::NotAnAppUrl)
    );
}

#[test]
fn android_app_url_without_cert_never_matches() {
    let saved = LoginUrl::new("androidapp://com.example.app");
    assert_eq!(
        app_fill_allowed(&saved, "com.example.app", &[CERT]),
        Err(Refusal::NoMatch)
    );
    // And an app URL never matches a web page.
    let page = PageOrigin::parse("https://com.example.app").unwrap();
    assert_eq!(
        fill_allowed(&app_url("com.example.app", &CERT), &page),
        Err(Refusal::NoMatch)
    );
}

#[test]
fn android_app_matches_only_equal_package_and_current_signer() {
    let saved = app_url("com.example.app", &CERT);
    // G1-9: more than one current signer is ambiguous and refused.
    assert_eq!(
        app_fill_allowed(&saved, "com.example.app", &[OTHER_CERT, CERT]),
        Err(Refusal::NoMatch)
    );
    assert_eq!(
        app_fill_allowed(&saved, "com.example.app", &[CERT, CERT]),
        Err(Refusal::NoMatch)
    );
    assert_eq!(
        app_fill_allowed(&saved, "com.example.app", &[OTHER_CERT]),
        Err(Refusal::NoMatch)
    );
    assert_eq!(
        app_fill_allowed(&saved, "com.example.app", &[]),
        Err(Refusal::NoMatch)
    );
    assert_eq!(
        app_fill_allowed(&saved, "com.example.app2", &[CERT]),
        Err(Refusal::NoMatch)
    );
    assert_eq!(
        app_fill_allowed(&saved, "com.Example.app", &[CERT]),
        Err(Refusal::NoMatch)
    );
}

#[test]
fn android_app_url_rejects_bad_package_names() {
    assert!(is_valid_package_name("com.example.app_1"));
    assert!(is_valid_package_name(&"a".repeat(255)));
    assert!(!is_valid_package_name(""));
    assert!(!is_valid_package_name(&"a".repeat(256)));
    for bad in ["a b", "a/b", "a-b", "a:b", "a?b", "ä.b", "a\u{0}b"] {
        assert!(!is_valid_package_name(bad), "{bad:?}");
        assert_eq!(
            AndroidAppUrl::parse(&format!("androidapp://{bad}")),
            Err(AppUrlError::BadPackage),
            "{bad:?}"
        );
    }
    assert!(AndroidAppUrl::new("a/b", None).is_err());
}

#[test]
fn privileged_browser_allowlist_parses_and_has_chrome_and_firefox() {
    assert!(package_count() >= 10);
    assert!(privileged_browsers_json().contains("com.android.chrome"));
    let chrome = first_cert("com.android.chrome").unwrap();
    let firefox = first_cert("org.mozilla.firefox").unwrap();
    assert!(is_trusted_browser("com.android.chrome", &[chrome]));
    assert!(is_trusted_browser("org.mozilla.firefox", &[firefox]));
    // G1-9: the caller's identity is one current signer.
    assert!(!is_trusted_browser(
        "org.mozilla.firefox",
        &[OTHER_CERT, firefox]
    ));
    assert!(!is_trusted_browser(
        "org.mozilla.firefox",
        &[firefox, firefox]
    ));
    // Right package, wrong key; right key, wrong package; unknown package.
    assert!(!is_trusted_browser("com.android.chrome", &[OTHER_CERT]));
    assert!(!is_trusted_browser("com.android.chrome", &[]));
    assert!(!is_trusted_browser("org.mozilla.firefox", &[chrome]));
    assert!(!is_trusted_browser("com.example.app", &[chrome]));
}

fn caller<'a>(
    package: &'a str,
    certs: &'a [[u8; 32]],
    web_domain: Option<&'a str>,
    web_scheme: Option<&'a str>,
) -> AndroidCaller<'a> {
    AndroidCaller {
        package,
        certs,
        web_domain,
        web_scheme,
    }
}

#[test]
fn web_domain_from_untrusted_app_is_ignored() {
    let target = android_fill_target(&caller(
        "com.example.app",
        &[CERT],
        Some("github.com"),
        None,
    ))
    .unwrap();
    assert_eq!(
        target,
        FillTarget::AndroidApp {
            package: "com.example.app".to_owned(),
            certs: vec![CERT],
        }
    );
    // A browser package with the wrong key is not a browser either.
    let target = android_fill_target(&caller(
        "com.android.chrome",
        &[OTHER_CERT],
        Some("github.com"),
        None,
    ))
    .unwrap();
    assert!(matches!(target, FillTarget::AndroidApp { .. }));
    assert_eq!(
        android_fill_target(&caller("bad pkg", &[], None, None)),
        Err(TargetError::BadPackage)
    );
}

#[test]
fn web_domain_from_allowlisted_browser_is_page_origin() {
    let cert = first_cert("com.android.chrome").unwrap();
    let certs = [cert];
    let target = android_fill_target(&caller(
        "com.android.chrome",
        &certs,
        Some("github.com"),
        None,
    ))
    .unwrap();
    let FillTarget::Web {
        origin,
        idn_unicode,
    } = target
    else {
        panic!("expected a web target");
    };
    assert_eq!(origin.serialize(), "https://github.com");
    assert_eq!(idn_unicode, None);
    let target = android_fill_target(&caller(
        "com.android.chrome",
        &certs,
        Some("localhost:8080"),
        Some("http"),
    ))
    .unwrap();
    let FillTarget::Web { origin, .. } = target else {
        panic!("expected a web target");
    };
    assert_eq!(origin.serialize(), "http://localhost:8080");
    assert_eq!(
        android_fill_target(&caller(
            "com.android.chrome",
            &certs,
            Some("github.com"),
            Some("ftp")
        )),
        Err(TargetError::Scheme)
    );
    assert_eq!(
        android_fill_target(&caller(
            "com.android.chrome",
            &certs,
            Some("github.com/evil"),
            None
        )),
        Err(TargetError::NotAnOrigin)
    );
}

#[test]
fn allowlisted_browser_without_web_domain_is_refused() {
    let cert = first_cert("com.android.chrome").unwrap();
    let certs = [cert];
    assert_eq!(
        android_fill_target(&caller("com.android.chrome", &certs, None, None)),
        Err(TargetError::BrowserWithoutDomain)
    );
    assert!(crate::android_browsers::is_browser_package(
        "com.android.chrome"
    ));
    assert!(!crate::android_browsers::is_browser_package(
        "com.example.app"
    ));
}

#[test]
fn web_scheme_http_from_browser_is_refused_without_allow_http() {
    let cert = first_cert("com.android.chrome").unwrap();
    let certs = [cert];
    let FillTarget::Web { origin, .. } = android_fill_target(&caller(
        "com.android.chrome",
        &certs,
        Some("example.com"),
        Some("http"),
    ))
    .unwrap() else {
        panic!("expected a web target");
    };
    let mut saved = LoginUrl::new("http://example.com");
    assert_eq!(fill_allowed(&saved, &origin), Err(Refusal::Insecure));
    saved.allow_http = true;
    assert_eq!(fill_allowed(&saved, &origin), Ok(MatchKind::ExactOrigin));
}

#[test]
fn ios_domain_identifier_is_https_origin() {
    let FillTarget::Web { origin, .. } =
        ios_fill_target(IosIdentifier::Domain("GitHub.com")).unwrap()
    else {
        panic!("expected a web target");
    };
    assert_eq!(origin.serialize(), "https://github.com");
    for bad in [
        "",
        "1.2.3.4",
        "a.com:8080",
        "a.com/x",
        "https://a.com",
        "a b.com",
    ] {
        assert!(
            ios_fill_target(IosIdentifier::Domain(bad)).is_err(),
            "{bad:?}"
        );
    }
}

#[test]
fn ios_url_identifier_keeps_scheme_host_port() {
    let FillTarget::Web { origin, .. } = ios_fill_target(IosIdentifier::Url(
        "http://user:pw@Example.com:8080/login?x=1#f",
    ))
    .unwrap() else {
        panic!("expected a web target");
    };
    assert_eq!(origin.serialize(), "http://example.com:8080");
    let FillTarget::Web { origin, .. } =
        ios_fill_target(IosIdentifier::Url("https://example.com:443/")).unwrap()
    else {
        panic!("expected a web target");
    };
    assert_eq!(origin.serialize(), "https://example.com");
    assert_eq!(
        ios_fill_target(IosIdentifier::Url("ftp://example.com")),
        Err(TargetError::Scheme)
    );
    assert!(ios_fill_target(IosIdentifier::Url("not a url")).is_err());
}

#[test]
fn ios_punycode_identifier_carries_idn_warning() {
    let FillTarget::Web {
        origin,
        idn_unicode,
    } = ios_fill_target(IosIdentifier::Domain("bücher.example")).unwrap()
    else {
        panic!("expected a web target");
    };
    assert_eq!(origin.serialize(), "https://xn--bcher-kva.example");
    assert_eq!(idn_unicode.as_deref(), Some("bücher.example"));
}

fn node(domain: Option<&str>, hints: &[&str]) -> FieldNode {
    FieldNode {
        web_domain: domain.map(str::to_owned),
        hints: hints.iter().map(|h| (*h).to_owned()).collect(),
        autocomplete: None,
        input_kind: InputKind::Text,
        id_entry: None,
        visible: true,
        focused: false,
    }
}

#[test]
fn select_fields_excludes_foreign_frames() {
    let nodes = [
        node(Some("a.com"), &[]),
        node(Some("a.com"), &["username"]),
        node(Some("evil.com"), &["password"]),
        node(Some("a.com"), &["password"]),
    ];
    let s = select_fill_fields(&nodes);
    assert_eq!(s.root_domain.as_deref(), Some("a.com"));
    assert_eq!(s.refused, None);
    let picked: Vec<_> = s.fields.iter().map(|f| (f.index, f.role)).collect();
    assert_eq!(picked, [(1, FieldRole::Username), (3, FieldRole::Password)]);
}

#[test]
fn select_fields_refuses_focus_in_foreign_frame() {
    let mut nodes = [
        node(Some("a.com"), &["username"]),
        node(Some("evil.com"), &["password"]),
    ];
    nodes[1].focused = true;
    let s = select_fill_fields(&nodes);
    assert_eq!(s.refused, Some(FieldRefusal::ForeignFrame));
    assert_eq!(s.fields, []);
    // A native root with a web child is foreign too.
    let mut nodes = [node(None, &[]), node(Some("evil.com"), &["password"])];
    nodes[1].focused = true;
    assert_eq!(
        select_fill_fields(&nodes).refused,
        Some(FieldRefusal::ForeignFrame)
    );
}

#[test]
fn select_fields_by_hints_autocomplete_and_input_type() {
    let mut by_autocomplete = node(None, &[]);
    by_autocomplete.autocomplete = Some("section-login username webauthn".to_owned());
    let mut by_type = node(None, &[]);
    by_type.input_kind = InputKind::Password;
    let mut new_pw = node(None, &[]);
    new_pw.autocomplete = Some("new-password".to_owned());
    let nodes = [
        node(None, &["emailAddress"]),
        by_autocomplete,
        by_type,
        new_pw,
        node(None, &["newPassword"]),
        node(None, &["color"]),
    ];
    let roles: Vec<_> = select_fill_fields(&nodes)
        .fields
        .iter()
        .map(|f| f.role)
        .collect();
    assert_eq!(
        roles,
        [
            FieldRole::Username,
            FieldRole::Username,
            FieldRole::Password,
            FieldRole::NewPassword,
            FieldRole::NewPassword
        ]
    );
}

#[test]
fn select_fields_ignores_invisible_nodes() {
    let mut hidden = node(None, &["password"]);
    hidden.visible = false;
    let nodes = [node(None, &["username"]), hidden];
    assert_eq!(select_fill_fields(&nodes).fields.len(), 1);
}

#[test]
fn select_fields_finds_otp_by_one_time_code() {
    let mut by_autocomplete = node(None, &[]);
    by_autocomplete.autocomplete = Some("one-time-code".to_owned());
    let nodes = [node(None, &["smsOTPCode"]), by_autocomplete];
    let roles: Vec<_> = select_fill_fields(&nodes)
        .fields
        .iter()
        .map(|f| f.role)
        .collect();
    assert_eq!(roles, [FieldRole::Otp, FieldRole::Otp]);
}

#[test]
fn select_fields_de_and_en_keywords() {
    let ids = [
        ("input_username", FieldRole::Username),
        ("benutzername", FieldRole::Username),
        ("login_email", FieldRole::Username),
        ("et_password", FieldRole::Password),
        ("passwort", FieldRole::Password),
        ("kennwort_neu", FieldRole::NewPassword),
        ("new_password", FieldRole::NewPassword),
        ("otp_input", FieldRole::Otp),
        ("einmalcode", FieldRole::Otp),
    ];
    for (id, role) in ids {
        let mut n = node(None, &[]);
        n.id_entry = Some(id.to_owned());
        let s = select_fill_fields(&[n]);
        assert_eq!(s.fields.first().map(|f| f.role), Some(role), "{id}");
    }
    let mut n = node(None, &[]);
    n.id_entry = Some("search_box".to_owned());
    assert_eq!(select_fill_fields(&[n]).fields, []);
}

fn links(cert_fingerprint: &str, relation: &str) -> Vec<u8> {
    format!(
        r#"[{{"relation":["{relation}"],"target":{{"namespace":"android_app","package_name":"com.example.app","sha256_cert_fingerprints":["{cert_fingerprint}"],"extra":1}}}},
{{"relation":["delegate_permission/common.handle_all_urls"],"target":{{"namespace":"web","site":"https://example.com"}}}}]"#
    )
    .into_bytes()
}

fn colon_hex_of(cert: &[u8; 32]) -> String {
    cert.iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(":")
}

#[test]
fn assetlinks_get_login_creds_matches_package_and_cert() {
    let fp = colon_hex_of(&CERT);
    let statements =
        parse_asset_links(&links(&fp, "delegate_permission/common.get_login_creds")).unwrap();
    assert_eq!(statements.len(), 2);
    assert!(app_is_delegated(
        &statements,
        "com.example.app",
        &CERT,
        Relation::GetLoginCreds
    ));
    // Lowercase fingerprints compare equal.
    let lower = parse_asset_links(&links(
        &fp.to_lowercase(),
        "delegate_permission/common.get_login_creds",
    ))
    .unwrap();
    assert!(app_is_delegated(
        &lower,
        "com.example.app",
        &CERT,
        Relation::GetLoginCreds
    ));
}

#[test]
fn assetlinks_handle_all_urls_accepted_for_passkeys_only() {
    let fp = colon_hex_of(&CERT);
    let statements =
        parse_asset_links(&links(&fp, "delegate_permission/common.handle_all_urls")).unwrap();
    assert!(!app_is_delegated(
        &statements,
        "com.example.app",
        &CERT,
        Relation::GetLoginCreds
    ));
    assert!(app_is_delegated(
        &statements,
        "com.example.app",
        &CERT,
        Relation::HandleAllUrls
    ));
    assert!(app_is_delegated_for_passkeys(
        &statements,
        "com.example.app",
        &CERT
    ));
}

#[test]
fn assetlinks_wrong_cert_or_relation_does_not_match() {
    let fp = colon_hex_of(&CERT);
    let statements =
        parse_asset_links(&links(&fp, "delegate_permission/common.get_login_creds")).unwrap();
    assert!(!app_is_delegated_for_passkeys(
        &statements,
        "com.example.app",
        &OTHER_CERT
    ));
    assert!(!app_is_delegated_for_passkeys(
        &statements,
        "com.other.app",
        &CERT
    ));
    // The web statement does not delegate an app, whatever its relation.
    let other = parse_asset_links(&links(&fp, "delegate_permission/common.other")).unwrap();
    assert!(!app_is_delegated_for_passkeys(
        &other,
        "com.example.app",
        &CERT
    ));
}

#[test]
fn assetlinks_rejects_malformed_and_oversized() {
    for bad in [
        &b""[..],
        b"{}",
        b"[1]",
        b"[{}]",
        b"[{\"relation\":\"x\",\"target\":{}}]",
        b"[{\"relation\":[],\"target\":{\"namespace\":\"ios_app\"}}]",
        b"[{\"relation\":[],\"target\":{\"namespace\":\"android_app\",\"package_name\":1}}]",
        b"[{\"relation\":[]",
    ] {
        assert!(
            parse_asset_links(bad).is_err(),
            "{:?}",
            String::from_utf8_lossy(bad)
        );
    }
    assert_eq!(
        parse_asset_links(&vec![b' '; MAX_ASSET_LINKS_LEN + 1]),
        Err(AssetLinksError::TooLarge)
    );
    let one = r#"{"relation":[],"target":{"namespace":"web","site":"https://a.com"}}"#;
    let many = format!("[{}]", vec![one; MAX_STATEMENTS + 1].join(","));
    assert_eq!(
        parse_asset_links(many.as_bytes()),
        Err(AssetLinksError::TooManyStatements)
    );
    let ok = format!("[{}]", vec![one; MAX_STATEMENTS].join(","));
    assert_eq!(
        parse_asset_links(ok.as_bytes()).unwrap().len(),
        MAX_STATEMENTS
    );
    assert_eq!(parse_asset_links(b"[]").unwrap().len(), 0);
}
