//! The 1Password 1PUX importer (zip archive plus `export.data` JSON):
//! arbitrary input never panics, no member is read past its limit, and
//! whatever it accepts makes valid items (`common::check`). Seeds: small
//! archives built from `crates/import/tests/fixtures/1pux-export.data.json`.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    common::check(mosaic_import::onepux::parse(data));
});
