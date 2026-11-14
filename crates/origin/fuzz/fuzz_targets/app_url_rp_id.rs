//! Android app URLs, relying party IDs and Android fill targets (ADR-0092):
//! arbitrary text never panics them; an accepted app URL prints back to its
//! trimmed input; an accepted rp ID is canonical and stable; a web target
//! built from a claimed `webDomain` is an `http(s)` origin without anything
//! but a host and port.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_origin::app::AndroidAppUrl;
use mosaic_origin::target::{
    AndroidCaller, FillTarget, IosIdentifier, android_fill_target, ios_fill_target,
};
use mosaic_origin::{canonical_rp_id, webauthn_rp_id, PageOrigin};

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(url) = AndroidAppUrl::parse(text) {
        assert_eq!(url.canonical(), text.trim(), "app url round trip");
    }
    if let Some(rp) = canonical_rp_id(text) {
        assert_eq!(canonical_rp_id(&rp).as_deref(), Some(rp.as_str()));
        assert!(rp.is_ascii() && rp == rp.to_ascii_lowercase());
        // A page of that very host may use it as its rp ID.
        if let Ok(origin) = PageOrigin::parse(&format!("https://{rp}")) {
            let _ = webauthn_rp_id(&origin, Some(&rp));
        }
    }
    // The text is split in a package name and a claimed domain.
    let (package, domain) = text.split_once('\n').unwrap_or(("com.example.app", text));
    let caller = AndroidCaller {
        package,
        certs: &[[7; 32]],
        web_domain: Some(domain),
        web_scheme: None,
    };
    if let Ok(FillTarget::Web { origin, .. }) = android_fill_target(&caller) {
        let s = origin.serialize();
        assert!(s.starts_with("http://") || s.starts_with("https://"));
    }
    // `Web` only ever comes from a trusted browser: an app is never a page.
    if let Ok(FillTarget::Web { .. }) = android_fill_target(&AndroidCaller {
        package: "com.example.not.a.browser",
        certs: &[[7; 32]],
        web_domain: Some(domain),
        web_scheme: Some("https"),
    }) {
        panic!("web target from an ordinary app");
    }
    for identifier in [IosIdentifier::Domain(text), IosIdentifier::Url(text)] {
        if let Ok(FillTarget::Web { origin, .. }) = ios_fill_target(identifier) {
            assert!(PageOrigin::parse(&origin.serialize()).is_ok());
        }
    }
});
