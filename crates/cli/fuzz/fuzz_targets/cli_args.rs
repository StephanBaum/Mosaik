//! The command-line parser (CLI spec §3.1): arbitrary argument vectors never
//! panic `spec::parse` or the help text built from what it returns.
//!
//! The input is split at NUL bytes into arguments (invalid UTF-8 is replaced).
#![no_main]

use std::ffi::OsString;

use libfuzzer_sys::fuzz_target;
use mosaic_cli::spec::{self, Request};

fuzz_target!(|data: &[u8]| {
    let args: Vec<OsString> = data
        .split(|b| *b == 0)
        .map(|part| OsString::from(String::from_utf8_lossy(part).into_owned()))
        .collect();
    match spec::parse(args) {
        Ok(Request::Run(parsed)) => {
            let _ = parsed.has("json");
            let _ = parsed.value("vault");
            let _ = parsed.values("field");
            let _ = spec::help_text(Some(parsed.cmd));
        }
        Ok(Request::Help(command)) => {
            let _ = spec::help_text(command);
        }
        Ok(Request::Version) => {}
        Err(_) => {}
    }
});
