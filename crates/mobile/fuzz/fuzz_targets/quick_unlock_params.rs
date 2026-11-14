//! The quick-unlock params record (CRYPTO_SPEC §6.9, ADR-0090): arbitrary bytes
//! never panic the strict decoder, and anything it accepts encodes back to
//! exactly the same bytes (no second encoding of one value).
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_mobile::Platform;
use mosaic_mobile::quick_unlock::params::Params;

fuzz_target!(|data: &[u8]| {
    for platform in [Platform::Ios, Platform::Android] {
        if let Ok(params) = Params::decode(data, platform) {
            assert_eq!(params.encode(), data, "decode/encode round trip");
            assert!((1..=30).contains(&params.max_age_days));
            let _ = params.key_alias();
            let _ = params.aad();
        }
    }
});
