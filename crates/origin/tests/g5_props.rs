//! M10-G5 property tests over the public API of `mosaic-origin`: field
//! selection, relying party IDs, app URLs, fill targets, the HTTP policy and
//! asset links.

#![expect(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "test helpers outside #[test] functions"
)]

use mosaic_core::model::LoginUrl;
use mosaic_origin::android_browsers::{is_trusted_browser, privileged_browsers_json};
use mosaic_origin::app::{AndroidAppUrl, app_fill_allowed, is_valid_package_name};
use mosaic_origin::assetlinks::{
    MAX_ASSET_LINKS_LEN, Relation, app_is_delegated, parse_asset_links,
};
use mosaic_origin::fields::{
    FieldNode, FieldRefusal, FieldScope, InputKind, MAX_NODES, select_fill_fields,
    select_scoped_fill_fields,
};
use mosaic_origin::target::{AndroidCaller, FillTarget, TargetError, android_fill_target};
use mosaic_origin::{
    HttpPolicy, MAX_RP_ID_LEN, PageOrigin, Refusal, canonical_rp_id, fill_allowed_with,
};
use proptest::prelude::*;

const HOST: &str = "login.example.com";

fn domain() -> impl Strategy<Value = Option<String>> {
    prop_oneof![
        Just(None),
        Just(Some(HOST.to_owned())),
        Just(Some("LOGIN.Example.COM".to_owned())),
        Just(Some("evil.example.net".to_owned())),
        Just(Some("login.example.com.evil.net".to_owned())),
        Just(Some(String::new())),
    ]
}

fn node() -> impl Strategy<Value = FieldNode> {
    (
        domain(),
        any::<bool>(),
        prop::collection::vec(
            prop::sample::select(vec![
                "username",
                "password",
                "newPassword",
                "smsOTPCode",
                "emailAddress",
                "login_field",
                "kennwort",
                "",
            ]),
            0..3,
        ),
        prop::option::of(prop::sample::select(vec![
            "one-time-code",
            "current-password",
            "section-x username",
            "off",
        ])),
        prop::sample::select(vec![InputKind::Text, InputKind::Password, InputKind::Other]),
        any::<bool>(),
        any::<bool>(),
    )
        .prop_map(
            |(web_domain, is_web, hints, autocomplete, input_kind, visible, focused)| FieldNode {
                web_domain,
                is_web,
                hints: hints.into_iter().map(str::to_owned).collect(),
                autocomplete: autocomplete.map(str::to_owned),
                input_kind,
                id_entry: None,
                visible,
                focused,
            },
        )
}

fn is_foreign_for_web(n: &FieldNode) -> bool {
    n.web_domain
        .as_deref()
        .is_none_or(|d| !d.eq_ignore_ascii_case(HOST))
}

fn is_foreign_for_app(n: &FieldNode) -> bool {
    n.web_domain.is_some() || n.is_web
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// An app target never selects a web or foreign node, and a focus in one
    /// refuses the request with no fields.
    #[test]
    fn app_scope_selects_only_native_visible_nodes(
        nodes in prop::collection::vec(node(), 0..24),
    ) {
        let s = select_scoped_fill_fields(&nodes, FieldScope::App);
        if nodes.iter().any(|n| n.focused && is_foreign_for_app(n)) {
            prop_assert_eq!(s.refused, Some(FieldRefusal::ForeignFrame));
            prop_assert!(s.fields.is_empty());
        } else {
            prop_assert_eq!(s.refused, None);
        }
        for f in &s.fields {
            let n = &nodes[f.index];
            prop_assert!(!is_foreign_for_app(n));
            prop_assert!(n.visible);
        }
    }

    /// A web target never selects a node outside the target host.
    #[test]
    fn web_scope_selects_only_nodes_of_the_target_host(
        nodes in prop::collection::vec(node(), 0..24),
    ) {
        let s = select_scoped_fill_fields(&nodes, FieldScope::Web { host: HOST });
        if nodes.iter().any(|n| n.focused && is_foreign_for_web(n)) {
            prop_assert_eq!(s.refused, Some(FieldRefusal::ForeignFrame));
            prop_assert!(s.fields.is_empty());
        } else {
            prop_assert_eq!(s.refused, None);
        }
        for f in &s.fields {
            let n = &nodes[f.index];
            prop_assert!(!is_foreign_for_web(n));
            prop_assert!(n.visible);
        }
    }

    /// The same nodes in reverse order give the same fields (by node),
    /// whatever the scope: independence from ordering.
    #[test]
    fn selection_does_not_depend_on_node_order(
        nodes in prop::collection::vec(node(), 0..16),
        web in any::<bool>(),
    ) {
        let scope = if web { FieldScope::Web { host: HOST } } else { FieldScope::App };
        let forward = select_scoped_fill_fields(&nodes, scope);
        let reversed: Vec<FieldNode> = nodes.iter().rev().cloned().collect();
        let backward = select_scoped_fill_fields(&reversed, scope);
        prop_assert_eq!(forward.refused, backward.refused);
        let mut a: Vec<_> = forward.fields.iter().map(|f| (f.index, f.role)).collect();
        let mut b: Vec<_> = backward
            .fields
            .iter()
            .map(|f| (nodes.len() - 1 - f.index, f.role))
            .collect();
        a.sort_by_key(|(i, _)| *i);
        b.sort_by_key(|(i, _)| *i);
        prop_assert_eq!(a, b);
    }

    /// The unscoped selector never picks a node of another domain than the
    /// root's, nor an invisible one.
    #[test]
    fn unscoped_selection_stays_in_the_root_domain(
        nodes in prop::collection::vec(node(), 0..24),
    ) {
        let s = select_fill_fields(&nodes);
        prop_assert_eq!(&s.root_domain, &nodes.first().and_then(|n| n.web_domain.clone()));
        if s.refused.is_some() {
            prop_assert!(s.fields.is_empty());
        }
        for f in &s.fields {
            prop_assert_eq!(&nodes[f.index].web_domain, &s.root_domain);
            prop_assert!(nodes[f.index].visible);
        }
    }
}

#[test]
fn nodes_beyond_the_limit_are_never_selected() {
    let fill = |domain: Option<&str>| FieldNode {
        web_domain: domain.map(str::to_owned),
        is_web: false,
        hints: vec!["password".to_owned()],
        autocomplete: None,
        input_kind: InputKind::Password,
        id_entry: None,
        visible: true,
        focused: false,
    };
    let mut nodes: Vec<FieldNode> = (0..MAX_NODES + 10).map(|_| fill(None)).collect();
    // A focused foreign node past the limit is not even looked at.
    nodes[MAX_NODES + 1].focused = true;
    nodes[MAX_NODES + 1].web_domain = Some("evil.net".to_owned());
    let s = select_scoped_fill_fields(&nodes, FieldScope::App);
    assert_eq!(s.refused, None);
    assert_eq!(s.fields.len(), MAX_NODES);
    assert!(s.fields.iter().all(|f| f.index < MAX_NODES));
    let s = select_fill_fields(&nodes);
    assert_eq!(s.fields.len(), MAX_NODES);
}

fn in_ldh_alphabet(s: &str) -> bool {
    s.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// `canonical_rp_id` never panics, its result is lowercase LDH, idempotent
    /// and within the limits.
    #[test]
    fn canonical_rp_id_is_idempotent_and_ldh(raw in "\\PC{0,80}") {
        if let Some(rp) = canonical_rp_id(&raw) {
            prop_assert!(rp.len() <= MAX_RP_ID_LEN);
            prop_assert!(in_ldh_alphabet(&rp));
            prop_assert_eq!(rp.to_ascii_lowercase(), rp.clone());
            prop_assert!(rp.contains('.'), "a bare public suffix has no registrable domain");
            for label in rp.split('.') {
                prop_assert!(!label.is_empty() && label.len() <= 63);
                prop_assert!(!label.starts_with('-') && !label.ends_with('-'));
            }
            prop_assert_eq!(canonical_rp_id(&rp), Some(rp));
        }
    }

    /// Anything outside letters, digits, hyphen and dot is refused.
    #[test]
    fn canonical_rp_id_refuses_foreign_characters(
        prefix in "[a-z]{1,8}", bad in prop::sample::select(vec![
            '*', '!', '$', '_', ' ', '/', ':', '@', '\u{e4}', '\u{202e}', '\u{0}', '%', '\\',
        ]),
    ) {
        let raw = format!("{prefix}{bad}x.example.com");
        prop_assert_eq!(canonical_rp_id(&raw), None);
    }

    /// The only rewrites are lowercasing and removing one trailing dot.
    #[test]
    fn canonical_rp_id_normalizes_case_and_one_trailing_dot(
        label in "[a-z][a-z0-9]{0,10}",
        upper in any::<bool>(),
    ) {
        let want = format!("{label}.example.com");
        let raw = if upper { want.to_ascii_uppercase() } else { want.clone() };
        prop_assert_eq!(canonical_rp_id(&raw), Some(want.clone()));
        prop_assert_eq!(canonical_rp_id(&format!("{raw}.")), Some(want));
        prop_assert_eq!(canonical_rp_id(&format!("{raw}..")), None);
    }

    #[test]
    fn canonical_rp_id_refuses_long_labels(n in 64usize..80) {
        let label = "a".repeat(n);
        prop_assert_eq!(canonical_rp_id(&format!("{label}.example.com")), None);
    }
}

#[test]
fn canonical_rp_id_refuses_names_over_the_length_limit() {
    let long = format!("{}.com", ["abcdefghi"; 30].join("."));
    assert!(long.len() > MAX_RP_ID_LEN);
    assert_eq!(canonical_rp_id(&long), None);
}

#[test]
fn canonical_rp_id_known_answers() {
    for (raw, want) in [
        ("example.com", Some("example.com")),
        ("Example.COM.", Some("example.com")),
        ("login.example.co.uk", Some("login.example.co.uk")),
        ("xn--bcher-kva.example", Some("xn--bcher-kva.example")),
        ("com", None),
        ("co.uk", None),
        ("", None),
        (".", None),
        ("127.0.0.1", None),
        ("[::1]", None),
        ("-a.example.com", None),
        ("a-.example.com", None),
        ("a..example.com", None),
        ("https://example.com", None),
        ("b\u{fc}cher.example", None),
    ] {
        assert_eq!(canonical_rp_id(raw).as_deref(), want, "{raw:?}");
    }
}

fn package() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,10}(\\.[A-Za-z][A-Za-z0-9_]{0,10}){0,4}"
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn app_url_round_trips(pkg in package(), cert in prop::option::of(any::<[u8; 32]>())) {
        let url = AndroidAppUrl::new(&pkg, cert).unwrap();
        let text = url.canonical();
        let back = AndroidAppUrl::parse(&text).unwrap();
        prop_assert_eq!(&back, &url);
        prop_assert_eq!(back.canonical(), text);
        prop_assert_eq!(back.package(), pkg.as_str());
        prop_assert_eq!(back.cert_sha256(), cert.as_ref());
    }

    /// A mutated app URL is refused, or it is canonical (it prints back as
    /// the mutated text).
    #[test]
    fn a_mutated_app_url_is_rejected_or_canonical(
        pkg in package(), cert in any::<[u8; 32]>(),
        pos in any::<prop::sample::Index>(), delta in 1u8..=255,
    ) {
        let text = AndroidAppUrl::new(&pkg, Some(cert)).unwrap().canonical();
        let mut bytes = text.into_bytes();
        let i = pos.index(bytes.len());
        bytes[i] ^= delta;
        if let Ok(mutated) = String::from_utf8(bytes)
            && let Ok(parsed) = AndroidAppUrl::parse(&mutated)
        {
            prop_assert_eq!(parsed.canonical(), mutated.trim().to_owned());
        }
    }

    #[test]
    fn package_names_outside_the_alphabet_are_invalid(
        pkg in package(), bad in prop::sample::select(vec!['-', '/', ' ', '\u{e4}', '?', '#', ':']),
    ) {
        prop_assert!(is_valid_package_name(&pkg));
        let bad_name = format!("{pkg}{bad}");
        prop_assert!(!is_valid_package_name(&bad_name));
    }

    /// Only the exact package with exactly the one saved certificate fills.
    #[test]
    fn app_fill_needs_the_same_package_and_the_one_signer(
        pkg in package(), other in package(),
        cert in any::<[u8; 32]>(), other_cert in any::<[u8; 32]>(),
    ) {
        let saved = LoginUrl::new(&AndroidAppUrl::new(&pkg, Some(cert)).unwrap().canonical());
        prop_assert!(app_fill_allowed(&saved, &pkg, &[cert]).is_ok());
        prop_assert!(app_fill_allowed(&saved, &pkg, &[]).is_err());
        prop_assert!(app_fill_allowed(&saved, &pkg, &[cert, cert]).is_err());
        prop_assert!(app_fill_allowed(&saved, &pkg, &[cert, other_cert]).is_err());
        if other != pkg {
            prop_assert!(app_fill_allowed(&saved, &other, &[cert]).is_err());
        }
        if other_cert != cert {
            prop_assert!(app_fill_allowed(&saved, &pkg, &[other_cert]).is_err());
        }
        // Without a certificate nothing matches.
        let bare = LoginUrl::new(&AndroidAppUrl::new(&pkg, None).unwrap().canonical());
        prop_assert!(app_fill_allowed(&bare, &pkg, &[cert]).is_err());
    }

    /// An app that is not an allowlisted browser never gets a web target,
    /// whatever `webDomain` it claims.
    #[test]
    fn a_web_domain_from_an_ordinary_app_is_ignored(
        pkg in "x[a-z]{1,8}\\.app", certs in prop::collection::vec(any::<[u8; 32]>(), 0..3),
        domain in prop::option::of("[a-z]{1,8}\\.example\\.com"),
        scheme in prop::option::of(prop::sample::select(vec!["http", "https", "javascript"])),
    ) {
        let caller = AndroidCaller {
            package: &pkg,
            certs: &certs,
            web_domain: domain.as_deref(),
            web_scheme: scheme,
        };
        match android_fill_target(&caller).unwrap() {
            FillTarget::AndroidApp { package, certs: c } => {
                prop_assert_eq!(package, pkg);
                prop_assert_eq!(c, certs);
            }
            FillTarget::Web { .. } => prop_assert!(false, "web target from an ordinary app"),
        }
    }
}

/// The first allowlisted browser: its package and one release certificate.
fn chrome() -> (String, [u8; 32]) {
    let v: serde_json::Value = serde_json::from_str(privileged_browsers_json()).unwrap();
    let info = &v["apps"][0]["info"];
    let pkg = info["package_name"].as_str().unwrap().to_owned();
    let fp = info["signatures"][0]["cert_fingerprint_sha256"]
        .as_str()
        .unwrap();
    let bytes: Vec<u8> = fp
        .split(':')
        .map(|h| u8::from_str_radix(h, 16).unwrap())
        .collect();
    (pkg, bytes.try_into().unwrap())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(128))]

    /// A trusted browser's page origin is the lowercase domain; the scheme
    /// is only http or https.
    #[test]
    fn a_trusted_browser_yields_the_page_origin_or_a_refusal(
        domain in "[A-Za-z0-9.:/@?#%-]{0,24}",
        scheme in prop::option::of(prop::sample::select(vec!["http", "https", "ftp", "HTTPS", ""])),
    ) {
        let (pkg, cert) = chrome();
        let certs = [cert];
        let caller = AndroidCaller {
            package: &pkg,
            certs: &certs,
            web_domain: Some(&domain),
            web_scheme: scheme,
        };
        match android_fill_target(&caller) {
            Ok(FillTarget::Web { origin, .. }) => {
                let s = origin.serialize();
                prop_assert!(s.starts_with("http://") || s.starts_with("https://"));
                prop_assert_eq!(s.clone(), s.to_ascii_lowercase());
                let after_scheme = s.split_once("://").unwrap().1;
                prop_assert!(!after_scheme.contains(['/', '@', '?', '#']));
            }
            Ok(FillTarget::AndroidApp { .. }) => prop_assert!(false, "browser as app"),
            Err(e) => prop_assert!(matches!(
                e,
                TargetError::Scheme | TargetError::NotAnOrigin
            )),
        }
    }

    /// Under the strict policy every `http` page needs `allow_http`, loopback
    /// or not; under the exempt one only loopback is free.
    #[test]
    fn the_http_policy_gates_loopback(
        port in 1024u16..65535, allow in any::<bool>(),
        host in prop::sample::select(vec!["localhost", "127.0.0.1", "[::1]", "example.com"]),
    ) {
        let origin = format!("http://{host}:{port}");
        let page = PageOrigin::parse(&origin).unwrap();
        let mut saved = LoginUrl::new(&origin);
        saved.allow_http = allow;
        let loopback = host != "example.com";
        let strict = fill_allowed_with(&saved, &page, HttpPolicy::LoopbackNeedsAllowHttp);
        prop_assert_eq!(strict.is_ok(), allow);
        if !allow {
            prop_assert_eq!(strict, Err(Refusal::Insecure));
        }
        let exempt = fill_allowed_with(&saved, &page, HttpPolicy::LoopbackExempt);
        prop_assert_eq!(exempt.is_ok(), allow || loopback);
    }

    /// The allowlist trusts a browser only for exactly one known signer.
    #[test]
    fn the_allowlist_needs_exactly_one_known_signer(extra in any::<[u8; 32]>()) {
        let (pkg, cert) = chrome();
        prop_assert!(is_trusted_browser(&pkg, &[cert]));
        prop_assert!(!is_trusted_browser(&pkg, &[]));
        prop_assert!(!is_trusted_browser(&pkg, &[cert, extra]));
        prop_assert!(!is_trusted_browser(&pkg, &[extra]));
        prop_assert!(!is_trusted_browser("com.example.not.a.browser", &[cert]));
    }

    /// The asset links parser never panics; a file delegates an app only for
    /// the relation, package and certificate it names.
    #[test]
    fn asset_links_parse_never_panics_and_matches_exactly(
        data in prop::collection::vec(any::<u8>(), 0..300),
        pkg in package(), cert in any::<[u8; 32]>(), other in any::<[u8; 32]>(),
    ) {
        let _ = parse_asset_links(&data);
        let fp: Vec<String> = cert.iter().map(|b| format!("{b:02x}")).collect();
        let file = serde_json::json!([{
            "relation": [Relation::GetLoginCreds.as_str()],
            "target": {
                "namespace": "android_app",
                "package_name": pkg,
                "sha256_cert_fingerprints": [fp.join(":")],
            },
        }])
        .to_string();
        let statements = parse_asset_links(file.as_bytes()).unwrap();
        prop_assert!(app_is_delegated(&statements, &pkg, &cert, Relation::GetLoginCreds));
        prop_assert!(!app_is_delegated(&statements, &pkg, &cert, Relation::HandleAllUrls));
        if other != cert {
            prop_assert!(!app_is_delegated(&statements, &pkg, &other, Relation::GetLoginCreds));
        }
        let longer = format!("{pkg}x");
        prop_assert!(!app_is_delegated(&statements, &longer, &cert, Relation::GetLoginCreds));
    }
}

#[test]
fn asset_links_size_limit_is_exact() {
    let pad = |n: usize| {
        let mut s = String::from("[]");
        s.push_str(&" ".repeat(n - 2));
        s
    };
    assert!(parse_asset_links(pad(MAX_ASSET_LINKS_LEN).as_bytes()).is_ok());
    assert!(parse_asset_links(pad(MAX_ASSET_LINKS_LEN + 1).as_bytes()).is_err());
}
