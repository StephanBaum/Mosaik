//! The Proton Pass importer (zip, data.json or PGP armor): arbitrary input never panics, no member is read past its
//! limit, and whatever it accepts makes valid items (`common::check`).
//! Seeds: synthetic exports built from the tests in
//! `crates/import/tests/it`.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use mosaic_import::Format;

fuzz_target!(|data: &[u8]| {
    common::check(mosaic_import::parse(Format::ProtonPass, data, None));
});
