//! Binding validation (agent access design §6.1, §6.2).

use mosaic_core::model::{AgentBasicUser, AgentBindingRecord, AgentInject, Text};
use sha2::{Digest, Sha256};

use crate::binding::*;

fn bearer() -> AgentBindingRecord {
    let mut r = AgentBindingRecord::new(
        "api_key.secret",
        &["https://api.openai.com"],
        AgentInject::Bearer,
    );
    r.env = Some(Text::new("OPENAI_API_KEY"));
    r
}

fn header(name: &str, prefix: &str) -> AgentBindingRecord {
    AgentBindingRecord::new(
        "api_key.secret",
        &["https://api.example.com"],
        AgentInject::Header {
            name: Text::new(name),
            prefix: Text::new(prefix),
        },
    )
}

#[test]
fn valid_bearer_binding_parses_with_digest() {
    let b = Binding::from_record(&bearer()).unwrap();
    assert_eq!(b.origins.len(), 1);
    assert_eq!(b.origins[0].as_str(), "https://api.openai.com");
    assert_eq!(b.inject, Inject::Bearer);
    assert_eq!(b.inject.kind().as_str(), "bearer");
    assert_eq!(b.env.as_deref(), Some("OPENAI_API_KEY"));
    assert!(!b.run && b.methods.is_none() && b.paths.is_none());
    assert!(matches!(b.field, BindingField::Secret(_)));
    let canonical = br#"{"env":"OPENAI_API_KEY","field":"api_key.secret","inject":{"kind":"bearer"},"methods":null,"origins":["https://api.openai.com"],"paths":null,"run":false}"#;
    let expected: [u8; 32] = Sha256::digest(canonical).into();
    assert_eq!(b.digest, expected);
    // Any change of the policy changes the digest.
    let mut r = bearer();
    r.origins.push(Text::new("https://api.example.com"));
    assert_ne!(Binding::from_record(&r).unwrap().digest, b.digest);
}

#[test]
fn custom_fields_by_element_id_and_text_fields() {
    let custom = AgentBindingRecord::new(
        "field:0000000000000007.dev.0",
        &["https://a.example"],
        AgentInject::Bearer,
    );
    assert!(matches!(
        Binding::from_record(&custom).unwrap().field,
        BindingField::Custom(_)
    ));
    for field in [
        "login.username",
        "api_key.key_id",
        "api_key.host",
        "notes",
        "title",
        "",
        "field:",
        "nope",
    ] {
        let r = AgentBindingRecord::new(field, &["https://a.example"], AgentInject::Bearer);
        assert_eq!(
            Binding::from_record(&r).err(),
            Some(BindingError::Field),
            "{field}"
        );
    }
    for field in ["login.password", "api_key.secret", "card.cvv"] {
        let r = AgentBindingRecord::new(field, &["https://a.example"], AgentInject::Bearer);
        assert!(Binding::from_record(&r).is_ok(), "{field}");
    }
}

#[test]
fn text_fields_cannot_be_marked() {
    let r = AgentBindingRecord::new(
        "login.username",
        &["https://a.example"],
        AgentInject::Bearer,
    );
    assert_eq!(Binding::from_record(&r).err(), Some(BindingError::Field));
}

#[test]
fn origins_must_be_canonical_https_and_1_to_16() {
    let rec =
        |origins: &[&str]| AgentBindingRecord::new("api_key.secret", origins, AgentInject::Bearer);
    let err = |origins: &[&str]| Binding::from_record(&rec(origins)).err();

    assert_eq!(err(&[]), Some(BindingError::Origins));
    assert_eq!(
        err(&["https://a.example", "https://a.example"]),
        Some(BindingError::Origins)
    );
    for bad in [
        "https://A.example",
        "https://a.example/",
        "a.example",
        "https://a.example:443",
    ] {
        assert!(
            matches!(err(&[bad]), Some(BindingError::Origin(_)),),
            "{bad}"
        );
    }
    assert!(matches!(
        err(&["http://a.example"]),
        Some(BindingError::Origin(
            mosaic_origin::AgentOriginError::NotHttps
        ))
    ));
    assert!(matches!(
        err(&["https://10.0.0.1"]),
        Some(BindingError::Origin(
            mosaic_origin::AgentOriginError::IpLiteral
        ))
    ));
    let names: Vec<String> = (0..16).map(|i| format!("https://h{i}.example")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    assert_eq!(Binding::from_record(&rec(&refs)).unwrap().origins.len(), 16);
    let mut more = names.clone();
    more.push("https://h16.example".to_owned());
    let refs: Vec<&str> = more.iter().map(String::as_str).collect();
    assert_eq!(err(&refs), Some(BindingError::Origins));
}

#[test]
fn forbidden_and_non_token_header_names_are_refused() {
    for name in [
        "Host",
        "host",
        "Content-Length",
        "transfer-encoding",
        "Connection",
        "TE",
        "Upgrade",
        "proxy-authorization",
        "Proxy-Anything",
        "Cookie",
        "set-cookie",
        "x api key",
        "x-api-key\r",
        "x-api-key\n",
        "",
        "a:b",
        "ü",
    ] {
        assert_eq!(
            Binding::from_record(&header(name, "")).err(),
            Some(BindingError::HeaderName),
            "{name:?}"
        );
    }
    for name in ["x-api-key", "Authorization", "X-Custom_1.2", "api-key"] {
        assert!(Binding::from_record(&header(name, "")).is_ok(), "{name}");
    }
    assert!(!is_token(&"a".repeat(65)));
    assert!(is_token(&"a".repeat(64)));
}

#[test]
fn header_prefix_rules() {
    assert!(Binding::from_record(&header("Authorization", "token ")).is_ok());
    assert!(Binding::from_record(&header("Authorization", &"a".repeat(32))).is_ok());
    for prefix in [&"a".repeat(33)[..], "a\r\nb", "tab\t", "ü", "del\u{7f}"] {
        assert_eq!(
            Binding::from_record(&header("Authorization", prefix)).err(),
            Some(BindingError::HeaderPrefix),
            "{prefix:?}"
        );
    }
}

#[test]
fn query_and_basic_rules() {
    let q = |name: &str| {
        AgentBindingRecord::new(
            "api_key.secret",
            &["https://a.example"],
            AgentInject::Query {
                name: Text::new(name),
            },
        )
    };
    for ok in ["key", "api_key", "a-b.c~d", &"k".repeat(64)] {
        assert!(Binding::from_record(&q(ok)).is_ok(), "{ok}");
    }
    for bad in ["", "a b", "a&b", "a=b", "ü", &"k".repeat(65), "a/b", "a%20"] {
        assert_eq!(
            Binding::from_record(&q(bad)).err(),
            Some(BindingError::QueryName),
            "{bad:?}"
        );
    }

    let basic = |user: AgentBasicUser| {
        AgentBindingRecord::new(
            "api_key.secret",
            &["https://a.example"],
            AgentInject::Basic { user },
        )
    };
    assert!(Binding::from_record(&basic(AgentBasicUser::SecretIsUser)).is_ok());
    assert!(Binding::from_record(&basic(AgentBasicUser::Literal(Text::new("me")))).is_ok());
    assert!(
        Binding::from_record(&basic(AgentBasicUser::Field(Text::new("api_key.key_id")))).is_ok()
    );
    for bad in [
        AgentBasicUser::Literal(Text::new("")),
        AgentBasicUser::Literal(Text::new("a:b")),
        AgentBasicUser::Literal(Text::new("a\nb")),
        AgentBasicUser::Literal(Text::new(&"u".repeat(257))),
        AgentBasicUser::Field(Text::new("api_key.secret")),
        AgentBasicUser::Field(Text::new("nope")),
    ] {
        assert_eq!(
            Binding::from_record(&basic(bad)).err(),
            Some(BindingError::BasicUser)
        );
    }
}

#[test]
fn env_deny_list_holds_case_insensitively() {
    for bad in [
        "mosaic_x",
        "MOSAIC_PASSWORD",
        "Path",
        "PATH",
        "pathext",
        "ComSpec",
        "ld_preload",
        "LD_LIBRARY_PATH",
        "DYLD_x",
        "dyld_insert_libraries",
        "NODE_OPTIONS",
        "node_options",
        "PYTHONPATH",
        "GIT_SSH_COMMAND",
        "1ABC",
        "A-B",
        "A B",
        "",
        "ünï",
        &"A".repeat(129),
    ] {
        assert!(!env_name_allowed(bad), "{bad:?}");
        let mut r = bearer();
        r.env = Some(Text::new(bad));
        assert_eq!(Binding::from_record(&r).err(), Some(BindingError::Env));
    }
    for ok in [
        "OPENAI_API_KEY",
        "_x",
        "a1",
        &"A".repeat(128),
        "MOSAICX",
        "LDX",
    ] {
        assert!(env_name_allowed(ok), "{ok}");
    }
}

#[test]
fn methods_and_paths_rules() {
    let with = |methods: Option<Vec<&str>>, paths: Option<Vec<&str>>| {
        let mut r = bearer();
        r.methods = methods.map(|m| m.into_iter().map(Text::new).collect());
        r.paths = paths.map(|p| p.into_iter().map(Text::new).collect());
        Binding::from_record(&r)
    };
    let b = with(Some(vec!["GET", "POST"]), Some(vec!["/v1", "/v2/x"])).unwrap();
    assert_eq!(b.methods, Some(vec![Method::Get, Method::Post]));
    assert_eq!(b.paths, Some(vec!["/v1".to_owned(), "/v2/x".to_owned()]));
    for m in [
        vec![],
        vec!["get"],
        vec!["GET", "GET"],
        vec!["TRACE"],
        vec!["CONNECT"],
    ] {
        assert_eq!(with(Some(m), None).err(), Some(BindingError::Methods));
    }
    let long = format!("/{}", "a".repeat(256));
    let many: Vec<String> = (0..17).map(|i| format!("/p{i}")).collect();
    for p in [
        vec![],
        vec!["v1"],
        vec!["/a?b"],
        vec!["/a#b"],
        vec!["/a\nb"],
        vec![long.as_str()],
        many.iter().map(String::as_str).collect(),
    ] {
        assert_eq!(with(None, Some(p)).err(), Some(BindingError::Paths));
    }
    for m in ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"] {
        assert_eq!(Method::parse(m).unwrap().as_str(), m);
    }
}

#[test]
fn value_rules() {
    assert_eq!(check_value("short"), Err(BindingError::ValueTooShort));
    assert_eq!(check_value("12345678901"), Err(BindingError::ValueTooShort));
    assert_eq!(check_value("123456789012"), Ok(()));
    assert_eq!(
        check_value("ü12345678901"),
        Ok(()),
        "12 bytes, not 12 chars"
    );
    assert_eq!(
        check_value("abcdefghijkl\n"),
        Err(BindingError::ValueHasControl)
    );
    assert_eq!(
        check_value("abcdefghijkl\r"),
        Err(BindingError::ValueHasControl)
    );
    assert_eq!(
        check_value("abcdefghijkl\0"),
        Err(BindingError::ValueHasControl)
    );
    assert_eq!(check_value("abc def\tghijkl"), Ok(()));
}

#[test]
fn binding_error_text_never_holds_input() {
    let canary = "CANARY-sk-test-0123456789";
    let mut texts: Vec<String> = Vec::new();
    let mut note = |e: BindingError| texts.push(format!("{e} {e:?}"));
    note(check_value("CANARY").unwrap_err());
    note(check_value(&format!("{canary}\n")).unwrap_err());
    note(
        Binding::from_record(&AgentBindingRecord::new(
            canary,
            &["https://a.example"],
            AgentInject::Bearer,
        ))
        .unwrap_err(),
    );
    note(
        Binding::from_record(&AgentBindingRecord::new(
            "api_key.secret",
            &[canary],
            AgentInject::Bearer,
        ))
        .unwrap_err(),
    );
    note(Binding::from_record(&header(&format!("{canary} x"), "")).unwrap_err());
    note(Binding::from_record(&header("Authorization", &format!("{canary}\n"))).unwrap_err());
    let mut r = bearer();
    r.env = Some(Text::new(&format!("MOSAIC_{canary}")));
    note(Binding::from_record(&r).unwrap_err());
    for t in texts {
        assert!(!t.contains("CANARY"), "input found in an error text");
    }
}
