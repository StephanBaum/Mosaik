//! Content-based detection: arbitrary input never panics, and whatever
//! `detect` names parses without a panic or an undocumented error with the
//! format it chose (a KDBX file ends in `PasswordRequired`, a documented
//! error). Seeds: one small file per detectable format.
#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Some(d) = mosaic_import::detect(data) {
        common::check(mosaic_import::parse(d.format, data, None));
    }
});
