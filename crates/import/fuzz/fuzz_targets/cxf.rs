//! The CXF v1.0 importer: arbitrary input never panics, and whatever it
//! accepts makes valid items and valid passkeys (`common::check`). Seeds: a
//! synthetic export with a passkey, a password and a note in
//! `crates/import/fuzz/seeds/cxf`.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    common::check(mosaic_import::cxf::parse(data));
});
