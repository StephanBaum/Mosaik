//! The setup code parser (CRYPTO_SPEC §6.9.7): arbitrary bytes never panic
//! it, and any code it accepts builds again to exactly the same bytes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::setup_code::{MAX_SETUP_CODE_LEN, build, parse};

/// A stand-in for the sync crate's canonicalization: only plain ASCII
/// strings are canonical, and they are their own canonical form.
fn identity(server: &str, email: &str) -> Option<(String, String)> {
    (server.is_ascii() && email.is_ascii()).then(|| (server.to_owned(), email.to_owned()))
}

fuzz_target!(|data: &[u8]| {
    let Ok(code) = parse(data, identity) else {
        return;
    };
    assert!(data.len() <= MAX_SETUP_CODE_LEN);
    let again = build(&code).expect("a parsed code builds");
    assert_eq!(again.as_bytes(), data, "parse/build round trip");
});
