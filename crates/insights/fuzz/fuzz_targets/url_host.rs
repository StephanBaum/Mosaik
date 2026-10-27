//! Login-URL parsing and the 2FA Directory lookup.
//!
//! - `url::parse` never panics, on any UTF-8 input.
//! - A parsed host is non-empty, at most 253 bytes, without a trailing `.`,
//!   and contains no URL delimiter or whitespace; the scheme is lowercase.
//! - The loopback and local-network checks never panic.
//! - A directory match is the normalized host itself or one of its parent
//!   domains (never an unrelated site).
#![no_main]

#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use libfuzzer_sys::fuzz_target;
use mosaic_insights::{totp_site, totp_sites_for_issuer, url};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Some(parts) = url::parse(text) {
        let host = parts.host();
        assert!(!host.is_empty() && host.len() <= url::MAX_HOST_LEN, "{host:?}");
        assert!(!host.ends_with('.'), "{host:?}");
        assert!(
            !host.contains(['/', '\\', '?', '#', '@', '%']) && !host.chars().any(char::is_whitespace),
            "{host:?}"
        );
        if let Some(scheme) = parts.scheme() {
            assert_eq!(scheme, scheme.to_ascii_lowercase());
            assert_eq!(parts.is_http(), scheme == "http");
        }
        let _ = (url::is_loopback(host), url::is_local_network(host));
        check_lookup(host);
    }
    check_lookup(text);
    for site in totp_sites_for_issuer(text) {
        assert!(!site.domain().is_empty());
    }
});

fn check_lookup(host: &str) {
    if let Some(site) = totp_site(host) {
        let normalized = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
        assert!(
            site.domains()
                .any(|d| normalized == d || normalized.ends_with(&format!(".{d}"))),
            "{host:?} matched unrelated {:?}",
            site.domain()
        );
        assert!(site.documentation().is_none_or(|d| d.starts_with("https://")));
    }
}