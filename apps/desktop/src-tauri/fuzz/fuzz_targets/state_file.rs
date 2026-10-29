//! The state file parser (`persist::parse`): arbitrary bytes never panic,
//! and whatever parses is normalized, keeps an auto-lock trigger on
//! (CRYPTO_SPEC §11) and survives a render and parse round trip.
#![no_main]
#![allow(dead_code, unused_imports)]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/contract/mod.rs"]
mod contract;
#[path = "../../src/persist.rs"]
mod persist;

fuzz_target!(|data: &[u8]| {
    if let Some(s) = persist::parse(data) {
        assert!(persist::has_lock_trigger(&s.settings));
        assert_eq!(persist::normalize(s.settings.clone()), s.settings);
        let rendered = persist::render(&s).expect("render");
        assert_eq!(persist::parse(&rendered), Some(s));
    }
});
