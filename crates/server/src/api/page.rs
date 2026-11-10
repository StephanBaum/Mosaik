//! The static security-key page, `GET /webauthn/` (CRYPTO_SPEC §9.3).
//!
//! Track T3 supplies the real page (`crates/server/assets/webauthn/`). Until
//! then the four files are minimal placeholders compiled into the binary;
//! what this module owns is the set of strict headers every one of them is
//! served with.

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

const INDEX: &[u8] = b"<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
<title>Mosaic</title><link rel=\"stylesheet\" href=\"/webauthn/page.css\"></head>\
<body><main id=\"app\"></main><script src=\"/webauthn/page.js\"></script></body></html>";
const SCRIPT: &[u8] = b"// placeholder: the handoff page script arrives with M9 T3\n";
const STYLE: &[u8] = b"/* placeholder: the handoff page styles arrive with M9 T3 */\n";
const STRINGS: &[u8] = b"{}";

/// The headers of every page response.
const HEADERS: [(&str, &str); 6] = [
    (
        "content-security-policy",
        "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; \
         img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    ),
    ("referrer-policy", "no-referrer"),
    ("cache-control", "no-store"),
    ("x-content-type-options", "nosniff"),
    ("cross-origin-opener-policy", "same-origin"),
    (
        "permissions-policy",
        "publickey-credentials-get=(self), publickey-credentials-create=(self)",
    ),
];

fn page(content_type: &'static str, body: &'static [u8]) -> Response {
    let mut r = (StatusCode::OK, body).into_response();
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
    for (k, v) in HEADERS {
        h.insert(
            header::HeaderName::from_static(k),
            HeaderValue::from_static(v),
        );
    }
    r
}

pub async fn index() -> Response {
    page("text/html; charset=utf-8", INDEX)
}

pub async fn script() -> Response {
    page("text/javascript; charset=utf-8", SCRIPT)
}

pub async fn style() -> Response {
    page("text/css; charset=utf-8", STYLE)
}

pub async fn strings() -> Response {
    page("application/json", STRINGS)
}
