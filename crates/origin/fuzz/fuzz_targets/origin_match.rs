//! The origin matcher (brief §4.1 to §4.3): two arbitrary URLs never panic any
//! entry point, and the rules keep their invariants.
//!
//! The input is split at the first NUL byte into a page string and a saved
//! URL string.
//!
//! - A page origin that parses is canonical: serializing and parsing it again
//!   gives the same origin.
//! - Same-origin is symmetric.
//! - An exact-origin match never succeeds across two different origins.
//! - The eTLD+1 rule is symmetric for two parsed origins.
//! - The IDN and lookalike checks are total.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::model::{LoginUrl, UrlMatch};
use mosaic_origin::{
    MatchKind, PageOrigin, canonical_rp_id, fill_allowed, idn_unicode, lookalike_of,
    registrable_domain, saved_origin, webauthn_rp_id,
};

fn saved(url: &str, mode: UrlMatch) -> LoginUrl {
    let mut u = LoginUrl::new(url);
    u.match_mode = mode;
    // Isolate the origin rules from the plain-HTTP refusal.
    u.allow_http = true;
    u
}

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let (a, b) = text.split_once('\0').unwrap_or((text, text));

    let pa = PageOrigin::parse(a);
    let pb = PageOrigin::parse(b);
    for p in [&pa, &pb].into_iter().flatten() {
        let again = PageOrigin::parse(&p.serialize()).expect("a canonical origin re-parses");
        assert_eq!(&again, p, "serialize round-trips");
        let host = p.host_str();
        let _ = idn_unicode(&host);
        let _ = registrable_domain(&host);
        let _ = lookalike_of(&host, [a, b, "example.com", "github.com"]);
        let _ = webauthn_rp_id(p, Some(b));
        let _ = webauthn_rp_id(p, None);
    }
    let _ = canonical_rp_id(a);
    let _ = saved_origin(a);
    let _ = saved_origin(b);

    if let (Ok(x), Ok(y)) = (&pa, &pb) {
        assert_eq!(x == y, y == x, "same-origin is symmetric");
        let exact = fill_allowed(&saved(&x.serialize(), UrlMatch::ExactOrigin), y);
        assert_eq!(
            exact == Ok(MatchKind::ExactOrigin),
            x == y,
            "an exact match is the same origin"
        );
        let xy = fill_allowed(&saved(&x.serialize(), UrlMatch::Etld1), y);
        let yx = fill_allowed(&saved(&y.serialize(), UrlMatch::Etld1), x);
        assert_eq!(
            xy.is_ok(),
            yx.is_ok(),
            "the eTLD+1 rule is symmetric for parsed origins"
        );
    }
    if let Ok(page) = &pb {
        // Any saved string, parsed or not: a refusal or a match, never a panic.
        for mode in [UrlMatch::ExactOrigin, UrlMatch::Etld1] {
            let _ = fill_allowed(&saved(a, mode), page);
        }
    }
});
