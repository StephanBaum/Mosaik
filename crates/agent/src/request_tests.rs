//! Request validation and injection (agent access design §7.2).

use std::fmt::Write as _;

use base64ct::{Base64, Encoding as _};
use mosaic_core::SecretString;
use mosaic_core::model::{AgentBasicUser, AgentBindingRecord, AgentInject, Text};
use proptest::prelude::*;

use crate::binding::{Binding, Inject};
use crate::error::AgentErrorCode;
use crate::request::{
    DEFAULT_MAX_BYTES, HttpRequestInput, MAX_AGENT_HEADER_VALUE, MAX_AGENT_HEADERS_TOTAL,
    MAX_REQUEST_BODY, PreparedRequest, validate,
};

const ORIGIN: &str = "https://api.example.com";
/// A value that shows up nowhere by accident.
const VALUE: &str = "SK-TEST-CANARY-0123456789abcdef";

fn record(inject: AgentInject) -> AgentBindingRecord {
    AgentBindingRecord::new("api_key.secret", &[ORIGIN], inject)
}

fn binding(inject: AgentInject) -> Binding {
    Binding::from_record(&record(inject)).unwrap()
}

fn bearer() -> Binding {
    binding(AgentInject::Bearer)
}

fn get(url: &str) -> HttpRequestInput {
    HttpRequestInput {
        method: "GET".to_owned(),
        url: url.to_owned(),
        headers: Vec::new(),
        body: None,
        max_bytes: None,
    }
}

fn with_header(name: &str, value: &str) -> HttpRequestInput {
    let mut input = get("https://api.example.com/v1");
    input.headers.push((name.to_owned(), value.to_owned()));
    input
}

fn secret() -> SecretString {
    SecretString::from_str_copy(VALUE).unwrap()
}

fn serialized(p: &PreparedRequest) -> String {
    let mut out = format!("{} {}\n", p.method.as_str(), p.url);
    for (n, v) in &p.headers {
        let _ = writeln!(out, "{n}: {v}");
    }
    out.push_str(p.body.as_deref().unwrap_or(""));
    out
}

fn header<'a>(p: &'a PreparedRequest, name: &str) -> Option<&'a str> {
    p.headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

#[test]
fn agent_headers_cannot_set_or_duplicate_the_injection_slot() {
    let bearer = bearer();
    for name in [
        "Authorization",
        "AUTHORIZATION",
        "authorization ",
        "Proxy-Authorization",
        "Host",
        "Cookie",
        "Content-Length",
        "Transfer-Encoding",
        "Connection",
        "Bad Name",
        "",
    ] {
        assert_eq!(
            validate(&bearer, &with_header(name, "x")).err(),
            Some(AgentErrorCode::HeaderNotAllowed),
            "{name:?}"
        );
    }
    // A value that smuggles a second header, a NUL, or another control character.
    for value in ["a\r\nX-Evil: 1", "a\nb", "a\0b", "a\u{1}b", "a\u{7f}b"] {
        assert_eq!(
            validate(&bearer, &with_header("X-A", value)).err(),
            Some(AgentErrorCode::HeaderNotAllowed),
            "{value:?}"
        );
    }
    // A tab and non-ASCII text are fine.
    assert!(validate(&bearer, &with_header("X-A", "a\tb \u{e4}")).is_ok());

    // Header injection: the configured name, in any case.
    let by_header = binding(AgentInject::Header {
        name: Text::new("X-Api-Key"),
        prefix: Text::new(""),
    });
    for name in ["x-api-key", "X-API-KEY", "X-Api-Key"] {
        assert_eq!(
            validate(&by_header, &with_header(name, "x")).err(),
            Some(AgentErrorCode::HeaderNotAllowed)
        );
    }
    // Authorization is free for a header binding.
    assert!(validate(&by_header, &with_header("Authorization", "x")).is_ok());

    // Basic holds Authorization as well.
    let basic = binding(AgentInject::Basic {
        user: AgentBasicUser::SecretIsUser,
    });
    assert_eq!(
        validate(&basic, &with_header("authorization", "x")).err(),
        Some(AgentErrorCode::HeaderNotAllowed)
    );

    // Query injection: the parameter may not be in the URL already.
    let by_query = binding(AgentInject::Query {
        name: Text::new("key"),
    });
    for url in [
        "https://api.example.com/v1?key=1",
        "https://api.example.com/v1?a=b&key=1",
        "https://api.example.com/v1?%6bey=1",
    ] {
        assert_eq!(
            validate(&by_query, &get(url)).err(),
            Some(AgentErrorCode::HeaderNotAllowed),
            "{url}"
        );
    }
    assert!(validate(&by_query, &get("https://api.example.com/v1?other=1")).is_ok());
}

#[test]
fn the_agents_accept_encoding_is_replaced_by_identity() {
    let mut input = get("https://api.example.com/v1");
    input
        .headers
        .push(("Accept-Encoding".to_owned(), "gzip, br".to_owned()));
    input.headers.push(("X-Keep".to_owned(), "1".to_owned()));
    let prepared = validate(&bearer(), &input)
        .unwrap()
        .inject(&Inject::Bearer, &secret(), None);
    let encodings: Vec<_> = prepared
        .headers
        .iter()
        .filter(|(n, _)| n.eq_ignore_ascii_case("accept-encoding"))
        .collect();
    assert_eq!(
        encodings,
        [&("accept-encoding".to_owned(), "identity".to_owned())]
    );
    assert_eq!(header(&prepared, "x-keep"), Some("1"));
}

#[test]
fn limits_hold() {
    let bearer = bearer();
    let mut input = get("https://api.example.com/v1");
    input.headers = (0..33)
        .map(|i| (format!("X-{i}"), "v".to_owned()))
        .collect();
    assert_eq!(
        validate(&bearer, &input).err(),
        Some(AgentErrorCode::RequestTooLarge)
    );
    input.headers.truncate(32);
    assert!(validate(&bearer, &input).is_ok());

    let big = "a".repeat(MAX_AGENT_HEADER_VALUE + 1);
    assert_eq!(
        validate(&bearer, &with_header("X-A", &big)).err(),
        Some(AgentErrorCode::RequestTooLarge)
    );
    assert!(validate(&bearer, &with_header("X-A", &big[1..])).is_ok());

    // 32 KiB in total, names and values: exactly the limit passes, one byte more does not.
    let mut input = get("https://api.example.com/v1");
    for i in 0..4 {
        input.headers.push((format!("X-{i}"), "a".repeat(8000)));
    }
    let used: usize = input.headers.iter().map(|(n, v)| n.len() + v.len()).sum();
    let rest = MAX_AGENT_HEADERS_TOTAL - used - "X-4".len();
    input.headers.push(("X-4".to_owned(), "b".repeat(rest)));
    assert!(validate(&bearer, &input).is_ok());
    input.headers[4].1.push('b');
    assert_eq!(
        validate(&bearer, &input).err(),
        Some(AgentErrorCode::RequestTooLarge)
    );

    let mut input = get("https://api.example.com/v1");
    input.body = Some("a".repeat(MAX_REQUEST_BODY));
    assert!(validate(&bearer, &input).is_ok());
    input.body = Some("a".repeat(MAX_REQUEST_BODY + 1));
    assert_eq!(
        validate(&bearer, &input).err(),
        Some(AgentErrorCode::RequestTooLarge)
    );

    let long_url = format!("https://api.example.com/{}", "a".repeat(8192));
    assert_eq!(
        validate(&bearer, &get(&long_url)).err(),
        Some(AgentErrorCode::DestinationNotAllowed)
    );

    let mut input = get("https://api.example.com/v1");
    for bad in [0, 1_048_577, u32::MAX] {
        input.max_bytes = Some(bad);
        assert_eq!(
            validate(&bearer, &input).err(),
            Some(AgentErrorCode::InvalidRequest)
        );
    }
    for good in [1, 1_048_576] {
        input.max_bytes = Some(good);
        assert_eq!(validate(&bearer, &input).unwrap().max_bytes, good as usize);
    }
    input.max_bytes = None;
    assert_eq!(
        validate(&bearer, &input).unwrap().max_bytes,
        DEFAULT_MAX_BYTES as usize
    );
}

#[test]
fn only_exact_origins_of_the_binding_pass() {
    let bearer = bearer();
    for url in [
        "http://api.example.com/v1",
        "https://api.example.com.evil.test/",
        "https://evil.test/",
        "https://api.example.com:8443/",
        "https://user:pw@api.example.com/",
        "https://api.example.com@evil.test/",
        "https://API.EXAMPLE.COM.evil.test/",
        "/v1",
        "not a url",
        "https://127.0.0.1/",
        "ftp://api.example.com/",
    ] {
        assert_eq!(
            validate(&bearer, &get(url)).err(),
            Some(AgentErrorCode::DestinationNotAllowed),
            "{url}"
        );
    }
    // The WHATWG parser lowercases the host and drops the default port.
    for url in [
        "https://API.example.com/v1",
        "https://api.example.com:443/v1",
        "https://api.example.com",
    ] {
        assert!(validate(&bearer, &get(url)).is_ok(), "{url}");
    }
    let ok = validate(&bearer, &get("https://api.example.com/v1/x?a=b#frag")).unwrap();
    assert_eq!(ok.origin, ORIGIN);
    assert_eq!(ok.path, "/v1/x");
}

#[test]
fn methods_and_paths_narrow_when_set() {
    let mut rec = record(AgentInject::Bearer);
    rec.methods = Some(vec![Text::new("GET"), Text::new("POST")]);
    rec.paths = Some(vec![Text::new("/v1"), Text::new("/ok/")]);
    let narrow = Binding::from_record(&rec).unwrap();
    for (method, url, expected) in [
        ("GET", "https://api.example.com/v1", None),
        ("GET", "https://api.example.com/v1/x", None),
        ("POST", "https://api.example.com/v1/x/y?q=1", None),
        ("GET", "https://api.example.com/ok/a", None),
        (
            "GET",
            "https://api.example.com/v10",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "GET",
            "https://api.example.com/",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "GET",
            "https://api.example.com/ok",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "GET",
            "https://api.example.com/v1/../admin",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "GET",
            "https://api.example.com/v1/%2e%2e/admin",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "GET",
            "https://api.example.com/v1/..%2Fadmin",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "GET",
            "https://api.example.com/v1/a%5Cb",
            Some(AgentErrorCode::DestinationNotAllowed),
        ),
        (
            "DELETE",
            "https://api.example.com/v1",
            Some(AgentErrorCode::MethodNotAllowed),
        ),
        (
            "PUT",
            "https://api.example.com/v1",
            Some(AgentErrorCode::MethodNotAllowed),
        ),
        (
            "get",
            "https://api.example.com/v1",
            Some(AgentErrorCode::MethodNotAllowed),
        ),
    ] {
        let mut input = get(url);
        input.method = method.to_owned();
        assert_eq!(validate(&narrow, &input).err(), expected, "{method} {url}");
    }
    // Without a list, any of the six passes and nothing else does.
    let open = bearer();
    for method in ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"] {
        let mut input = get("https://api.example.com/x");
        input.method = method.to_owned();
        assert!(validate(&open, &input).is_ok(), "{method}");
    }
    for method in ["CONNECT", "TRACE", "OPTIONS", "get", "", "GET "] {
        let mut input = get("https://api.example.com/x");
        input.method = method.to_owned();
        assert_eq!(
            validate(&open, &input).err(),
            Some(AgentErrorCode::MethodNotAllowed),
            "{method:?}"
        );
    }
}

#[test]
fn inject_places_the_value_in_exactly_one_slot() {
    let input = {
        let mut i = get("https://api.example.com/v1/items?a=b");
        i.body = Some("{\"k\":1}".to_owned());
        i.headers.push(("X-Other".to_owned(), "o".to_owned()));
        i
    };
    let secret = secret();

    // Bearer.
    let b = bearer();
    let p = validate(&b, &input)
        .unwrap()
        .inject(&b.inject, &secret, None);
    assert_eq!(
        p.secret_header
            .as_ref()
            .map(|(n, v)| (n.as_str(), v.as_str())),
        Some(("authorization", "Bearer SK-TEST-CANARY-0123456789abcdef"))
    );
    assert!(!serialized(&p).contains(VALUE), "canary found");

    // Header with a prefix.
    let b = binding(AgentInject::Header {
        name: Text::new("X-Api-Key"),
        prefix: Text::new("token "),
    });
    let p = validate(&b, &input)
        .unwrap()
        .inject(&b.inject, &secret, None);
    assert_eq!(
        p.secret_header
            .as_ref()
            .map(|(n, v)| (n.as_str(), v.as_str())),
        Some(("X-Api-Key", "token SK-TEST-CANARY-0123456789abcdef"))
    );
    assert!(!serialized(&p).contains(VALUE), "canary found");

    // Basic with a user name, and with the secret as the user.
    let b = binding(AgentInject::Basic {
        user: AgentBasicUser::Literal(Text::new("svc")),
    });
    let p = validate(&b, &input)
        .unwrap()
        .inject(&b.inject, &secret, Some("svc"));
    let want = format!(
        "Basic {}",
        Base64::encode_string(format!("svc:{VALUE}").as_bytes())
    );
    assert_eq!(
        p.secret_header.as_ref().map(|(_, v)| v.as_str()),
        Some(want.as_str())
    );
    assert!(!serialized(&p).contains(VALUE), "canary found");
    let b = binding(AgentInject::Basic {
        user: AgentBasicUser::SecretIsUser,
    });
    let p = validate(&b, &input)
        .unwrap()
        .inject(&b.inject, &secret, None);
    let want = format!(
        "Basic {}",
        Base64::encode_string(format!("{VALUE}:").as_bytes())
    );
    assert_eq!(
        p.secret_header.as_ref().map(|(_, v)| v.as_str()),
        Some(want.as_str())
    );

    // Query: appended form-encoded to the checked URL; no header is added.
    let b = binding(AgentInject::Query {
        name: Text::new("api_key"),
    });
    let mut odd = input.clone();
    odd.url = "https://api.example.com/v1/items".to_owned();
    let p = validate(&b, &odd).unwrap().inject(&b.inject, &secret, None);
    assert!(p.secret_header.is_none());
    assert_eq!(
        p.url.as_str(),
        "https://api.example.com/v1/items?api_key=SK-TEST-CANARY-0123456789abcdef"
    );
    assert!(!format!("{p:?}").contains(VALUE), "canary found");
    // With a query already present the value is appended, the rest untouched.
    let p = validate(&b, &input)
        .unwrap()
        .inject(&b.inject, &secret, None);
    assert_eq!(
        p.url.as_str(),
        "https://api.example.com/v1/items?a=b&api_key=SK-TEST-CANARY-0123456789abcdef"
    );
    assert!(
        p.headers.iter().all(|(_, v)| !v.contains(VALUE)),
        "canary found"
    );
    assert_eq!(p.body.as_deref(), Some("{\"k\":1}"));
}

#[test]
fn mosaic_refs_in_agent_input_are_not_expanded() {
    let reference = "mosaic://V/I/api_key.secret";
    let mut input = get(&format!("https://api.example.com/v1?r={reference}"));
    input
        .headers
        .push(("X-Ref".to_owned(), reference.to_owned()));
    input.body = Some(format!("{{\"r\":\"{reference}\"}}"));
    let b = bearer();
    let p = validate(&b, &input)
        .unwrap()
        .inject(&b.inject, &secret(), None);
    assert_eq!(header(&p, "x-ref"), Some(reference));
    assert_eq!(
        p.body.as_deref(),
        Some(format!("{{\"r\":\"{reference}\"}}").as_str())
    );
    assert!(p.url.as_str().contains("mosaic"));
    assert!(!serialized(&p).contains(VALUE), "canary found");
}

#[test]
fn debug_output_holds_no_value_header_or_body() {
    let mut input = with_header("X-A", "HEADER-CANARY");
    input.body = Some("BODY-CANARY".to_owned());
    let text = format!("{input:?}");
    assert!(!text.contains("HEADER-CANARY") && !text.contains("BODY-CANARY"));
    let b = bearer();
    let validated = validate(&b, &input).unwrap();
    assert_eq!(format!("{validated:?}"), "[REDACTED]");
    let prepared = validated.inject(&b.inject, &secret(), None);
    assert_eq!(format!("{prepared:?}"), "[REDACTED]");
}

proptest! {
    /// The URL that is sent has the host and port of the URL that was checked, as an independent
    /// parser (`http::Uri`) reads it too.
    #[test]
    fn the_sent_url_is_the_checked_url(
        path in "[a-zA-Z0-9/._~%@:!$&'()*+,;=-]{0,40}",
        query in "[a-zA-Z0-9=&%._~/?@:-]{0,40}",
    ) {
        let url = format!("https://api.example.com/{path}?{query}");
        let by_query = binding(AgentInject::Query { name: Text::new("key") });
        for b in [bearer(), by_query] {
            if let Ok(validated) = validate(&b, &get(&url)) {
                let checked_host = validated.url.host_str().map(str::to_owned);
                let checked_port = validated.url.port_or_known_default();
                let prepared = validated.inject(&b.inject, &secret(), None);
                prop_assert_eq!(prepared.url.host_str().map(str::to_owned), checked_host.clone());
                prop_assert_eq!(checked_host.as_deref(), Some("api.example.com"));
                let uri = http::Uri::try_from(prepared.url.as_str()).unwrap();
                prop_assert_eq!(uri.host(), Some("api.example.com"));
                prop_assert_eq!(uri.port_u16().or(Some(443)), checked_port);
                prop_assert_eq!(uri.scheme_str(), Some("https"));
            }
        }
    }
}
