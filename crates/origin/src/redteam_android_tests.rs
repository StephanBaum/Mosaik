//! RT-android red team (Android, round 4): the conservative defaults of the
//! Android fill target. Every test states the attack; a hole found here is
//! `#[ignore = "RT-android-<n>: ..."]` until the fix round removes the
//! attribute, and the other tests pin defenses that held.

use crate::android_browsers::first_cert;
use crate::target::{AndroidCaller, FillTarget, TargetError, android_fill_target};

fn chrome_cert() -> [u8; 32] {
    let cert = first_cert("com.android.chrome");
    assert!(cert.is_some(), "the allowlist names Chrome");
    cert.unwrap_or_default()
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

fn web_origin(target: &FillTarget) -> Option<String> {
    match target {
        FillTarget::Web { origin, .. } => Some(origin.serialize()),
        FillTarget::AndroidApp { .. } => None,
    }
}

/// Attack (RT-android-1): a network attacker serves `http://bank.example` (a
/// hotspot, a captive portal, a first visit before HSTS) in an allowlisted
/// browser that reports the page's `webDomain` but no `webScheme`
/// (`ViewStructure.setWebScheme` is optional, and the reader passes the scheme
/// only from the very node that reports the domain, M10-G2 round 2 N5). The
/// core reads the missing scheme as `https` (`target.rs`: `unwrap_or("https")`),
/// so the saved https login is offered and, after the tap, typed into the
/// cleartext page. Defense wanted: a missing scheme is refused (fail closed), or
/// at most read as `http`, which only a login saved with `allow_http` matches.
#[test]
#[ignore = "RT-android-1: a trusted browser's webDomain without a webScheme is read as https (target.rs unwrap_or(\"https\")); a plain-http page in a browser that omits the scheme is offered the https login"]
fn rt_android_1_a_missing_web_scheme_is_not_read_as_https() {
    let target = browser_target("bank.example", None);
    let upgraded =
        target.as_ref().ok().and_then(web_origin) == Some("https://bank.example".to_owned());
    assert!(!upgraded, "a missing scheme must not become https");
}

/// Held: an explicit scheme is taken exactly as given. `http` stays `http` (the
/// saved login then needs `allow_http`), and anything that is not exactly
/// `http` or `https` (case, trailing colon, whitespace, another scheme) is
/// refused rather than normalized.
#[test]
fn rt_android_held_an_explicit_scheme_is_kept_exactly() {
    assert_eq!(
        browser_target("bank.example", Some("http"))
            .ok()
            .as_ref()
            .and_then(web_origin),
        Some("http://bank.example".to_owned())
    );
    assert_eq!(
        browser_target("bank.example", Some("https"))
            .ok()
            .as_ref()
            .and_then(web_origin),
        Some("https://bank.example".to_owned())
    );
    for bad in [
        "HTTPS",
        "Https",
        "https:",
        "https://",
        " https",
        "https ",
        "ftp",
        "file",
        "content",
        "javascript",
        "android-app",
        "",
    ] {
        assert!(
            browser_target("bank.example", Some(bad)).is_err(),
            "{bad:?} must not be a scheme"
        );
    }
}

/// Held: a browser whose current signer is not on the allowlist is neither a
/// page (its `webDomain` is ignored) nor an app (it is never a login target by
/// package), whatever scheme it claims.
#[test]
fn rt_android_held_a_foreign_signer_gets_no_target_for_any_scheme() {
    let certs = [[0xa7; 32]];
    for scheme in [None, Some("http"), Some("https")] {
        assert!(
            android_fill_target(&AndroidCaller {
                package: "com.android.chrome",
                certs: &certs,
                web_domain: Some("bank.example"),
                web_scheme: scheme,
            })
            .is_err(),
            "{scheme:?}"
        );
    }
}
