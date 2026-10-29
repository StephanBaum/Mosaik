//! The Enpass JSON importer: arbitrary input never panics, and whatever it
//! accepts makes valid items (`common::check`). Seeds: the synthetic export
//! in `crates/import/tests/fixtures/enpass.json`; the dictionary holds its
//! member names and field types.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    common::check(mosaic_import::enpass::parse(data));
});
