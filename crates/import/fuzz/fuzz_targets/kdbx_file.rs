//! A whole KDBX 4 file opened with the password `fuzz`: arbitrary input
//! never panics or hangs (under `cfg(fuzzing)` the key-derivation limits
//! are tiny, so a header asking for more is refused before any work), and
//! whatever opens makes valid items (`common::check`). A mutated block
//! fails its HMAC, so the payload parser behind it is fuzzed directly by
//! `kdbx_payload`. Seeds: KDBX 4 files with the fuzzing-sized KDFs (AES-KDF
//! and Argon2d / Argon2id, AES-256 and ChaCha20).
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use mosaic_core::SecretString;

fuzz_target!(|data: &[u8]| {
    let password = SecretString::from_str_copy("fuzz").expect("password");
    common::check(mosaic_import::kdbx::parse(data, &password));
});
