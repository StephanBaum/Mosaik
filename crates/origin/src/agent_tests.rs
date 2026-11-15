//! Agent destinations (agent access design §6.3, §10.3).

use super::*;

fn decide(saved: &AgentOrigin, s: &str) -> Result<Url, AgentRefusal> {
    parse_agent_url(s).and_then(|u| agent_destination_allowed(saved, &u).map(|()| u))
}

#[test]
fn destination_must_equal_a_saved_origin_exactly() {
    let saved = AgentOrigin::from_input("https://api.example.com").unwrap();
    let ok = [
        "https://api.example.com/v1/x?q=1#f",
        "https://API.Example.COM/v1",
        "https://api.example.com:443/v1",
        "https://api.example.com:0443/",
        // `\` ends the authority in a special URL: the host is api.example.com.
        r"https://api.example.com\@evil.example/",
        "https://api%2Eexample.com/",
        "https://api.example.com",
    ];
    for s in ok {
        let url = decide(&saved, s).unwrap_or_else(|e| panic!("{s}: {e:?}"));
        assert_eq!(url.host_str(), Some("api.example.com"), "{s}");
        assert_eq!(url.scheme(), "https", "{s}");
        assert!(url.username().is_empty() && url.password().is_none(), "{s}");
    }

    let punycode = "https://xn--pi-6cd.example.com/";
    let long = format!("https://api.example.com/{}", "a".repeat(MAX_AGENT_URL_LEN));
    let rows: [(&str, AgentRefusal); 25] = [
        ("https://api.example.com:8443/", AgentRefusal::NotAllowed),
        ("http://api.example.com/", AgentRefusal::NotHttps),
        ("https://evil.example/", AgentRefusal::NotAllowed),
        ("https://sub.api.example.com/", AgentRefusal::NotAllowed),
        ("https://example.com/", AgentRefusal::NotAllowed),
        (
            "https://api.example.com.evil.example/",
            AgentRefusal::NotAllowed,
        ),
        (
            "https://api.example.com@evil.example/",
            AgentRefusal::Userinfo,
        ),
        ("https://u:p@api.example.com/", AgentRefusal::Userinfo),
        ("https://u@api.example.com/", AgentRefusal::Userinfo),
        (
            r"https://evil.example\@api.example.com/",
            AgentRefusal::NotAllowed,
        ),
        ("https://api.example.com./", AgentRefusal::NotAllowed),
        // Cyrillic "а" for the first letter, and its punycode form.
        ("https://\u{430}pi.example.com/", AgentRefusal::NotAllowed),
        (punycode, AgentRefusal::NotAllowed),
        ("https://93.184.216.34/", AgentRefusal::NotAllowed),
        ("https://0x5db8d822/", AgentRefusal::NotAllowed),
        ("https://[::1]/", AgentRefusal::NotAllowed),
        ("https://localhost/", AgentRefusal::NotAllowed),
        ("ftp://api.example.com/", AgentRefusal::NotHttps),
        ("//api.example.com/", AgentRefusal::Malformed),
        ("api.example.com", AgentRefusal::Malformed),
        ("https:///", AgentRefusal::Malformed),
        ("javascript:alert(1)", AgentRefusal::NotHttps),
        ("https://api.example.com:99999/", AgentRefusal::Malformed),
        ("", AgentRefusal::Malformed),
        (long.as_str(), AgentRefusal::TooLong),
    ];
    for (s, expected) in rows {
        assert_eq!(decide(&saved, s).err(), Some(expected), "{s:.80}");
    }
    assert!(long.len() > MAX_AGENT_URL_LEN);
}

#[test]
fn an_idn_destination_matches_only_its_punycode_host() {
    let saved = AgentOrigin::from_input("https://bücher.example").unwrap();
    assert_eq!(saved.as_str(), "https://xn--bcher-kva.example");
    assert!(decide(&saved, "https://bücher.example/x").is_ok());
    assert!(decide(&saved, "https://xn--bcher-kva.example/x").is_ok());
    assert_eq!(
        decide(&saved, "https://bucher.example/").err(),
        Some(AgentRefusal::NotAllowed)
    );
}

#[test]
fn saved_origin_rules() {
    let ok = |s: &str| AgentOrigin::from_input(s).map(|o| o.as_str().to_owned());
    assert_eq!(ok("api.openai.com").unwrap(), "https://api.openai.com");
    assert_eq!(
        ok("https://api.openai.com/").unwrap(),
        "https://api.openai.com"
    );
    assert_eq!(
        ok("  https://api.openai.com  ").unwrap(),
        "https://api.openai.com"
    );
    assert_eq!(
        ok("https://API.openai.com:443").unwrap(),
        "https://api.openai.com"
    );
    assert_eq!(
        ok("https://api.example.com:8443").unwrap(),
        "https://api.example.com:8443"
    );
    assert_eq!(
        ok("https://bücher.example").unwrap(),
        "https://xn--bcher-kva.example"
    );
    assert_eq!(
        AgentOrigin::from_input("https://API.x.test")
            .unwrap()
            .host(),
        "api.x.test"
    );

    let bad: [(&str, AgentOriginError); 14] = [
        ("https://api.openai.com/v1", AgentOriginError::HasPath),
        ("https://api.openai.com?x=1", AgentOriginError::HasPath),
        ("https://api.openai.com/#f", AgentOriginError::HasPath),
        ("http://api.openai.com", AgentOriginError::NotHttps),
        ("ftp://api.openai.com", AgentOriginError::NotHttps),
        ("https://10.0.0.1", AgentOriginError::IpLiteral),
        ("https://[::1]", AgentOriginError::IpLiteral),
        ("https://localhost", AgentOriginError::Localhost),
        ("https://a.localhost", AgentOriginError::Localhost),
        ("https://a.example.", AgentOriginError::TrailingDot),
        ("https://u@a.example", AgentOriginError::Userinfo),
        ("https://:p@a.example", AgentOriginError::Userinfo),
        ("", AgentOriginError::Malformed),
        ("https://", AgentOriginError::Malformed),
    ];
    for (s, e) in bad {
        assert_eq!(AgentOrigin::from_input(s).err(), Some(e), "{s}");
    }
    let long = format!("https://{}.example", "a".repeat(MAX_AGENT_URL_LEN));
    assert_eq!(
        AgentOrigin::from_input(&long).err(),
        Some(AgentOriginError::TooLong)
    );
}

#[test]
fn stored_origins_must_be_canonical() {
    assert!(AgentOrigin::parse_canonical("https://api.example.com").is_ok());
    assert!(AgentOrigin::parse_canonical("https://api.example.com:8443").is_ok());
    for s in [
        "https://API.example.com",
        "https://api.example.com/",
        "https://api.example.com:443",
        "api.example.com",
        " https://api.example.com",
        "https://bücher.example",
    ] {
        let e = AgentOrigin::parse_canonical(s).err();
        assert!(
            matches!(
                e,
                Some(AgentOriginError::NotCanonical | AgentOriginError::Malformed)
            ),
            "{s}: {e:?}"
        );
    }
    assert_eq!(
        AgentOrigin::parse_canonical("https://API.example.com").err(),
        Some(AgentOriginError::NotCanonical)
    );
}

#[test]
fn errors_name_no_input() {
    for e in [
        AgentOriginError::TooLong,
        AgentOriginError::Malformed,
        AgentOriginError::NotHttps,
        AgentOriginError::IpLiteral,
        AgentOriginError::Localhost,
        AgentOriginError::Userinfo,
        AgentOriginError::HasPath,
        AgentOriginError::TrailingDot,
        AgentOriginError::NotCanonical,
    ] {
        assert!(!format!("{e}").contains("://"));
    }
}
