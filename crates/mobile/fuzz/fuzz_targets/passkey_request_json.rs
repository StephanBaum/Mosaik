//! The passkey request JSON of Android Credential Manager (ADR-0093): arbitrary
//! bytes never panic the strict parser, an accepted request is within its
//! bounds, and the response builders always produce valid JSON.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_mobile::passkeys::json::{
    MAX_REQUEST_LEN, authentication_response, parse_create, parse_get, registration_response,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(get) = parse_get(data) {
        assert!(data.len() <= MAX_REQUEST_LEN);
        assert!(get.challenge.len() <= 1024);
        assert!(get.allow.len() <= 64);
        assert!(get.allow.iter().all(|id| id.len() <= 1023));
    }
    if let Ok(create) = parse_create(data) {
        assert!(data.len() <= MAX_REQUEST_LEN);
        assert!(create.challenge.len() <= 1024);
        assert!(create.user_handle.len() <= 64);
        assert!(create.exclude.len() <= 64);
    }
    // The builders take arbitrary bytes and emit JSON a parser accepts.
    let json = authentication_response(data, Some(data), data, data, data);
    assert!(json.starts_with('{') && !json.contains('='));
    let json = registration_response(data, None, data, data, data, -7);
    assert!(json.starts_with('{') && !json.contains('='));
});
