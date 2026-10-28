//! The decrypted KDBX 4 payload: inner header (stream cipher, binaries) and
//! the XML document with protected values. Arbitrary input never panics,
//! nesting and counts stay bounded, and whatever parses makes valid items
//! (`common::check`). Seeds: payloads of the synthetic sample database of
//! `crates/import/src/kdbx/tests.rs` (`ChaCha20`, `Salsa20` and no
//! stream); the dictionary holds the KeePass XML names.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    common::check(mosaic_import::fuzzing::kdbx_payload(data));
});
