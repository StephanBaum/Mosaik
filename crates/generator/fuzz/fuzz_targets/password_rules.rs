//! `passwordrules` parsing and rule-based generation.
//!
//! - Parsing never panics, on any UTF-8 input (invalid UTF-8 is decoded
//!   lossily, as a browser would hand over an attribute value).
//! - The canonical display parses back to the same rules.
//! - Generation with a fixed seed (so a crash reproduces from its input)
//!   either fails with a rules error or yields a password the rules accept.
//!
//! The first input byte is the preferred length; the rest is the rule text.
#![no_main]

#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use libfuzzer_sys::fuzz_target;
use mosaic_generator::fuzzing::generate_with_rules_seeded;
use mosaic_generator::{Error, PasswordRules};

fuzz_target!(|data: &[u8]| {
    let Some((&preferred, text)) = data.split_first() else {
        return;
    };
    let text = String::from_utf8_lossy(text);
    let rules = PasswordRules::parse(&text);
    let shown = rules.to_string();
    assert_eq!(
        PasswordRules::parse(&shown),
        rules,
        "display/parse round trip: {shown}"
    );

    match generate_with_rules_seeded(&rules, u32::from(preferred), 0x6d6f_7361_6963) {
        Ok(p) => {
            assert!(
                rules.is_satisfied_by(p.expose_secret()),
                "generated password violates {shown}"
            );
            let entropy = rules.entropy(u32::from(preferred)).expect("entropy");
            assert!(entropy.bits() >= 0.0);
        }
        Err(e) => assert!(
            matches!(
                e,
                Error::RulesUnsatisfiable | Error::ConstraintsTooStrict | Error::RetryLimit
            ),
            "unexpected error {e:?}"
        ),
    }
});
