//! `decode_hw_params`, the strict decoder of a hardware-key wrapping's
//! public parameters (CRYPTO_SPEC §6.3).
//!
//! Input: 16 bytes of `method_id`, then the parameter bytes. Whatever
//! decodes must re-encode to exactly the same bytes (the codec is
//! canonical), and the decoder must never panic.
//!
//! Needs `--cfg fuzzing` (set by `cargo fuzz`); to check without
//! cargo-fuzz: `RUSTFLAGS="--cfg fuzzing" cargo check --bins`.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_vault::session::fuzzing::decode_hw_params;

fuzz_target!(|data: &[u8]| {
    let Some((method_id, params)) = data.split_first_chunk::<16>() else {
        return;
    };
    if let Some(again) = decode_hw_params(method_id, params) {
        assert_eq!(again, params, "decoded parameters must re-encode exactly");
    }
});
