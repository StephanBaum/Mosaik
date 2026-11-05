use super::*;

fn page(s: &str) -> PageOrigin {
    PageOrigin::parse(s).unwrap()
}

fn saved(url: &str, mode: UrlMatch, allow_http: bool) -> LoginUrl {
    let mut u = LoginUrl::new(url);
    u.match_mode = mode;
    u.allow_http = allow_http;
    u
}

fn exact(url: &str, origin: &str) -> Result<MatchKind, Refusal> {
    fill_allowed(&saved(url, UrlMatch::ExactOrigin, false), &page(origin))
}

fn etld1(url: &str, origin: &str) -> Result<MatchKind, Refusal> {
    fill_allowed(&saved(url, UrlMatch::Etld1, false), &page(origin))
}

const OK_EXACT: Result<MatchKind, Refusal> = Ok(MatchKind::ExactOrigin);
const OK_ETLD1: Result<MatchKind, Refusal> = Ok(MatchKind::Etld1);
const NO: Result<MatchKind, Refusal> = Err(Refusal::NoMatch);

#[test]
fn exact_origin_cases() {
    assert_eq!(
        exact("https://example.com", "https://example.com"),
        OK_EXACT
    );
    assert_eq!(
        exact("https://example.com/login?x=1#f", "https://example.com"),
        OK_EXACT,
        "path, query, fragment"
    );
    assert_eq!(
        exact("https://example.com:443", "https://example.com"),
        OK_EXACT,
        "default port written out"
    );
    assert_eq!(
        exact("HTTPS://EXAMPLE.COM/", "https://example.com"),
        OK_EXACT,
        "case"
    );
    assert_eq!(
        exact("https://example.com:8443", "https://example.com:8443"),
        OK_EXACT
    );
    assert_eq!(exact("https://example.com:8443", "https://example.com"), NO);
    assert_eq!(exact("https://example.com", "https://example.com:8443"), NO);
    assert_eq!(exact("https://example.com", "https://www.example.com"), NO);
    assert_eq!(exact("https://www.example.com", "https://example.com"), NO);
    assert_eq!(exact("https://example.com.", "https://example.com"), NO);
    assert_eq!(exact("https://example.com", "https://example.com."), NO);
    assert_eq!(
        exact("https://example.com.", "https://example.com."),
        OK_EXACT
    );
    assert_eq!(exact("not a url at all", "https://example.com"), NO);
    assert_eq!(exact("", "https://example.com"), NO);
    assert_eq!(exact("ftp://example.com", "https://example.com"), NO);
}

#[test]
fn userinfo_cannot_smuggle_a_host() {
    assert_eq!(
        exact("https://bank.example@evil.example", "https://bank.example"),
        NO
    );
    assert_eq!(
        exact("https://bank.example@evil.example", "https://evil.example"),
        OK_EXACT,
        "the real host is evil.example"
    );
    assert_eq!(
        exact(
            "https://bank.example:pw@evil.example/",
            "https://bank.example"
        ),
        NO
    );
    assert_eq!(
        exact("bank.example@evil.example", "https://bank.example"),
        NO,
        "no scheme"
    );
    assert_eq!(
        exact(
            "https://evil.example\\@bank.example",
            "https://bank.example"
        ),
        NO,
        "backslash ends the authority"
    );
}

#[test]
fn saved_urls_without_scheme_are_https() {
    assert_eq!(exact("example.com", "https://example.com"), OK_EXACT);
    assert_eq!(
        exact("  example.com/login  ", "https://example.com"),
        OK_EXACT
    );
    assert_eq!(
        exact("localhost:8443/app", "https://localhost:8443"),
        OK_EXACT
    );
    assert_eq!(exact("example.com", "http://example.com"), NO);
    assert_eq!(saved_origin("javascript:alert(1)"), None);
    assert_eq!(saved_origin("mailto://a@b.c"), None);
    assert_eq!(saved_origin("data:text/html,x"), None);
    assert_eq!(saved_origin("   "), None);
    assert_eq!(saved_origin(&"a".repeat(MAX_SAVED_URL_LEN + 1)), None);
    assert_eq!(
        saved_origin("example.com:8080").unwrap().serialize(),
        "https://example.com:8080"
    );
}

#[test]
fn idn_unicode_and_punycode_are_equal() {
    assert_eq!(
        exact("https://bücher.example", "https://xn--bcher-kva.example"),
        OK_EXACT
    );
    assert_eq!(
        exact(
            "https://xn--bcher-kva.example",
            "https://xn--bcher-kva.example"
        ),
        OK_EXACT
    );
    assert_eq!(
        exact("https://bucher.example", "https://xn--bcher-kva.example"),
        NO
    );
    // Cyrillic "а" in place of Latin "a" is a different host.
    assert_eq!(
        exact("https://apple.com", "https://xn--pple-43d.com"),
        NO,
        "homograph"
    );
}

#[test]
fn http_and_https_never_match_each_other() {
    assert_eq!(exact("http://example.com", "https://example.com"), NO);
    assert_eq!(exact("https://example.com", "http://example.com"), NO);
    assert_eq!(etld1("http://example.com", "https://www.example.com"), NO);
    assert_eq!(etld1("https://example.com", "http://www.example.com"), NO);
}

#[test]
fn etld1_matches_registrable_domains_only_when_opted_in() {
    assert_eq!(exact("https://example.com", "https://www.example.com"), NO);
    assert_eq!(
        etld1("https://example.com", "https://www.example.com"),
        OK_ETLD1
    );
    assert_eq!(
        etld1("https://www.example.co.uk", "https://login.example.co.uk"),
        OK_ETLD1
    );
    assert_eq!(
        etld1("https://login.example.co.uk", "https://www.example.co.uk"),
        OK_ETLD1
    );
    assert_eq!(
        etld1("https://example.com:8443", "https://example.com"),
        OK_ETLD1,
        "port ignored"
    );
    assert_eq!(
        etld1("https://example.com", "https://example.com"),
        OK_EXACT
    );
    assert_eq!(etld1("https://example.com", "https://example.org"), NO);
    assert_eq!(etld1("https://example.co.uk", "https://other.co.uk"), NO);
    assert_eq!(
        etld1("https://a.github.io", "https://b.github.io"),
        NO,
        "private suffix"
    );
    assert_eq!(
        etld1("https://a.github.io", "https://a.github.io"),
        OK_EXACT
    );
    assert_eq!(
        etld1("https://example.com", "https://notexample.com"),
        NO,
        "suffix string match is not enough"
    );
    assert_eq!(
        etld1("https://co.uk", "https://example.co.uk"),
        NO,
        "bare suffix"
    );
    assert_eq!(etld1("https://co.uk", "https://co.uk"), OK_EXACT);
}

#[test]
fn ips_and_single_labels_match_exactly_only() {
    assert_eq!(etld1("https://192.168.0.1", "https://10.0.0.1"), NO);
    assert_eq!(
        etld1("https://192.168.0.1", "https://192.168.0.1"),
        OK_EXACT
    );
    assert_eq!(etld1("https://192.168.0.1:8443", "https://192.168.0.1"), NO);
    assert_eq!(etld1("https://[::1]", "https://[::1]:8443"), NO);
    assert_eq!(etld1("https://localhost", "https://app.localhost"), NO);
    assert_eq!(etld1("https://intranet", "https://intranet"), OK_EXACT);
    assert_eq!(etld1("https://wiki.intranet", "https://git.intranet"), NO);
    assert_eq!(registrable_domain("192.168.0.1"), None);
    assert_eq!(registrable_domain("127.1"), None);
    assert_eq!(registrable_domain("[::1]"), None);
    assert_eq!(registrable_domain("localhost"), None);
    assert_eq!(registrable_domain("com"), None);
    assert_eq!(registrable_domain("co.uk"), None);
    assert_eq!(registrable_domain("example.com."), None);
    assert_eq!(registrable_domain(""), None);
}

#[test]
fn registrable_domains() {
    assert_eq!(
        registrable_domain("www.example.com").as_deref(),
        Some("example.com")
    );
    assert_eq!(
        registrable_domain("EXAMPLE.com").as_deref(),
        Some("example.com")
    );
    assert_eq!(
        registrable_domain("a.b.example.co.uk").as_deref(),
        Some("example.co.uk")
    );
    assert_eq!(
        registrable_domain("a.github.io").as_deref(),
        Some("a.github.io")
    );
    assert_eq!(
        registrable_domain("xn--bcher-kva.example").as_deref(),
        Some("xn--bcher-kva.example")
    );
    assert_eq!(
        registrable_domain("bücher.example").as_deref(),
        Some("xn--bcher-kva.example")
    );
}

#[test]
fn http_pages_need_the_override_unless_loopback() {
    let plain = |url: &str, origin: &str, allow| {
        fill_allowed(&saved(url, UrlMatch::ExactOrigin, allow), &page(origin))
    };
    assert_eq!(
        plain("http://example.com", "http://example.com", false),
        Err(Refusal::Insecure)
    );
    assert_eq!(
        plain("http://example.com", "http://example.com", true),
        OK_EXACT
    );
    assert_eq!(plain("http://example.com", "http://other.com", false), NO);
    for host in [
        "localhost:3000",
        "app.localhost",
        "127.0.0.1:8080",
        "127.5.5.5",
        "[::1]:3000",
    ] {
        let origin = format!("http://{host}");
        assert_eq!(plain(&origin, &origin, false), OK_EXACT, "{host}");
    }
    assert_eq!(
        plain("http://192.168.0.1", "http://192.168.0.1", false),
        Err(Refusal::Insecure)
    );
    // The flag does not widen what matches.
    assert_eq!(plain("http://a.com", "http://b.com", true), NO);
    // eTLD+1 on http needs the flag too.
    assert_eq!(
        fill_allowed(
            &saved("http://example.com", UrlMatch::Etld1, false),
            &page("http://www.example.com")
        ),
        Err(Refusal::Insecure)
    );
    assert_eq!(
        fill_allowed(
            &saved("http://example.com", UrlMatch::Etld1, true),
            &page("http://www.example.com")
        ),
        OK_ETLD1
    );
}

#[test]
fn parse_accepts_canonical_origins_only() {
    for ok in [
        "https://example.com",
        "http://example.com",
        "https://example.com:8443",
        "http://127.0.0.1:3000",
        "http://[::1]:3000",
        "https://xn--bcher-kva.example",
        "https://example.com.",
    ] {
        assert_eq!(page(ok).serialize(), ok);
    }
    for (bad, why) in [
        ("https://a/", OriginError::NotCanonical),
        ("https://a/b", OriginError::NotCanonical),
        ("https://a?x", OriginError::NotCanonical),
        ("https://a#x", OriginError::NotCanonical),
        ("https://u@a", OriginError::NotCanonical),
        ("HTTPS://a", OriginError::NotCanonical),
        ("https://A", OriginError::NotCanonical),
        ("https://a:443", OriginError::NotCanonical),
        ("http://a:80", OriginError::NotCanonical),
        ("https://bücher.example", OriginError::NotCanonical),
        ("null", OriginError::Malformed),
        ("", OriginError::Malformed),
        ("example.com", OriginError::Malformed),
        ("file:///etc/passwd", OriginError::Scheme),
        (
            "chrome-extension://abcdefghijklmnopabcdefghijklmnop",
            OriginError::Scheme,
        ),
        ("data:text/html,x", OriginError::Scheme),
        ("about:blank", OriginError::Scheme),
        ("blob:https://example.com/uuid", OriginError::Scheme),
        ("ftp://example.com", OriginError::Scheme),
    ] {
        assert_eq!(PageOrigin::parse(bad), Err(why), "{bad:?}");
    }
    assert_eq!(
        PageOrigin::parse(&format!("https://{}", "a".repeat(MAX_ORIGIN_LEN))),
        Err(OriginError::TooLong)
    );
}

#[test]
fn origin_accessors() {
    let p = page("http://[::1]:3000");
    assert!(p.is_http() && p.is_loopback());
    assert_eq!(p.host_str(), "[::1]");
    let p = page("https://xn--bcher-kva.example");
    assert!(!p.is_http() && !p.is_loopback());
    assert_eq!(p.host_str(), "xn--bcher-kva.example");
    assert!(!page("https://localhost.example.com").is_loopback());
    assert!(!page("https://notlocalhost").is_loopback());
    assert!(page("https://x.localhost").is_loopback());
    assert!(page("http://localhost.").is_loopback());
}

#[test]
fn idn_unicode_shows_the_unicode_form() {
    assert_eq!(
        idn_unicode("xn--bcher-kva.example").as_deref(),
        Some("bücher.example")
    );
    assert_eq!(
        idn_unicode("www.xn--bcher-kva.example").as_deref(),
        Some("www.bücher.example")
    );
    assert_eq!(
        idn_unicode("XN--BCHER-KVA.example").as_deref(),
        Some("bücher.example")
    );
    assert_eq!(idn_unicode("example.com"), None);
    assert_eq!(idn_unicode("xn.example.com"), None);
    assert_eq!(idn_unicode(""), None);
    assert!(idn_unicode("xn--invalid-.example").is_some(), "still warns");
}

#[test]
fn lookalikes() {
    let saved = ["github.com", "gitlab.com", "example.org"];
    assert_eq!(lookalike_of("githuh.com", saved), Some("github.com"));
    assert_eq!(lookalike_of("github.co", saved), Some("github.com"));
    assert_eq!(lookalike_of("www.github.co", saved), Some("github.com"));
    assert_eq!(
        lookalike_of("gitub.com", saved),
        Some("github.com"),
        "deletion"
    );
    assert_eq!(
        lookalike_of("githhub.com", saved),
        Some("github.com"),
        "insertion"
    );
    assert_eq!(
        lookalike_of("gihtub.com", saved),
        Some("github.com"),
        "swap"
    );
    assert_eq!(
        lookalike_of("gitlab.com", ["github.com"]),
        None,
        "distance 2"
    );
    assert_eq!(
        lookalike_of("github.com", saved),
        None,
        "the saved domain itself"
    );
    assert_eq!(lookalike_of("login.github.com", saved), None);
    assert_eq!(lookalike_of("other.net", saved), None);
    assert_eq!(
        lookalike_of("gitlab.com", ["github.com", "gitlab.com"]),
        None
    );
    assert_eq!(lookalike_of("github.com", Vec::<&str>::new()), None);
    // IPs, single labels and suffixes are never lookalikes.
    assert_eq!(lookalike_of("192.168.0.1", saved), None);
    assert_eq!(lookalike_of("localhost", saved), None);
    assert_eq!(
        lookalike_of("github.com", ["192.168.0.1", "localhost", "com"]),
        None
    );
}

#[test]
fn short_names_have_no_distance_lookalikes() {
    assert_eq!(lookalike_of("abc.com", ["abd.com"]), None, "3 characters");
    assert_eq!(lookalike_of("ab.com", ["abc.com"]), None);
    assert_eq!(
        lookalike_of("abcd.com", ["abce.com"]),
        Some("abce.com"),
        "4 characters"
    );
    // The same name under another suffix warns whatever the length.
    assert_eq!(lookalike_of("ab.co", ["ab.com"]), Some("ab.com"));
}

#[test]
fn the_first_lookalike_in_sorted_order_wins() {
    let saved = ["githuc.com", "githua.com", "githud.com"];
    assert_eq!(lookalike_of("github.com", saved), Some("githua.com"));
    assert_eq!(
        lookalike_of("github.com", saved.iter().rev().copied()),
        Some("githua.com")
    );
}

#[test]
fn osa_distance_one() {
    for (a, b, one) in [
        ("abcd", "abcd", false),
        ("abcd", "abce", true),
        ("abcd", "abdc", true),
        ("abcd", "bacd", true),
        ("abcd", "bcad", false),
        ("abcd", "abc", true),
        ("abcd", "bcd", true),
        ("abcd", "acd", true),
        ("abc", "abcd", true),
        ("abcd", "ab", false),
        ("abcd", "badc", false),
        ("abcd", "wxyz", false),
        ("", "a", true),
        ("", "", false),
    ] {
        assert_eq!(osa_distance_is_one(a, b), one, "{a} {b}");
    }
}

mod props {
    use super::*;
    use proptest::prelude::*;

    fn host() -> impl Strategy<Value = String> {
        prop_oneof![
            "[a-z0-9]{1,12}(\\.[a-z0-9]{1,12}){0,3}\\.(com|org|co\\.uk|github\\.io|de)",
            "[a-z]{1,8}",
            "(25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])(\\.(25[0-5]|2[0-4][0-9]|1?[0-9]?[0-9])){3}",
            "xn--[a-z0-9]{2,10}\\.(com|example)",
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

        #[test]
        fn parse_serialize_round_trip(
            https in any::<bool>(),
            h in host(),
            port in proptest::option::of(1u16..),
        ) {
            let scheme = if https { "https" } else { "http" };
            let mut url = format!("{scheme}://{h}");
            if let Some(p) = port {
                url.push(':');
                url.push_str(&p.to_string());
            }
            let Ok(origin) = Url::parse(&url) else { return Ok(()); };
            let canonical = origin.origin().ascii_serialization();
            let parsed = PageOrigin::parse(&canonical).unwrap();
            prop_assert_eq!(parsed.serialize(), canonical.clone());
            prop_assert_eq!(PageOrigin::parse(&parsed.serialize()).unwrap(), parsed);
        }

        #[test]
        fn parse_never_panics(s in ".{0,200}") {
            let _ = PageOrigin::parse(&s);
            let _ = saved_origin(&s);
            let _ = registrable_domain(&s);
            let _ = idn_unicode(&s);
        }

        #[test]
        fn a_saved_origin_always_matches_itself(h in host(), https in any::<bool>()) {
            let scheme = if https { "https" } else { "http" };
            let Some(origin) = saved_origin(&format!("{scheme}://{h}")) else { return Ok(()); };
            let page = PageOrigin::parse(&origin.serialize()).unwrap();
            let url = saved(&origin.serialize(), UrlMatch::ExactOrigin, true);
            prop_assert_eq!(fill_allowed(&url, &page), Ok(MatchKind::ExactOrigin));
        }

        #[test]
        fn exact_match_implies_equal_origin(a in host(), b in host()) {
            let (Some(sa), Some(pb)) = (
                saved_origin(&format!("https://{a}")),
                PageOrigin::parse(&format!("https://{b}")).ok(),
            ) else { return Ok(()); };
            let url = saved(&sa.serialize(), UrlMatch::ExactOrigin, true);
            prop_assert_eq!(fill_allowed(&url, &pb).is_ok(), sa == pb);
        }

        #[test]
        fn lookalike_is_never_the_page_itself(h in host(), saved_host in host()) {
            if let Some(found) = lookalike_of(&h, [saved_host.as_str()]) {
                prop_assert_eq!(found, saved_host.as_str());
                prop_assert_ne!(registrable_domain(&h), registrable_domain(found));
            }
        }
    }
}
