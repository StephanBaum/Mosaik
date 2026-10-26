//! Domain lookup in the vendored quirks data: never panics, and a match is
//! the normalized host itself or one of its parent domains.
#![no_main]

#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use libfuzzer_sys::fuzz_target;
use mosaic_generator::{PasswordRules, rules_for_domain};

fuzz_target!(|data: &[u8]| {
    let Ok(host) = std::str::from_utf8(data) else {
        return;
    };
    if let Some(site) = rules_for_domain(host) {
        let normalized = host.strip_suffix('.').unwrap_or(host).to_ascii_lowercase();
        let domain = site.domain();
        assert!(
            normalized == domain || normalized.ends_with(&format!(".{domain}")),
            "{host:?} matched unrelated {domain:?}"
        );
        let rules = site.rules();
        assert_eq!(&PasswordRules::parse(&rules.to_string()), rules);
    }
});
