//! The state file parser (`persist::parse_checked`): arbitrary bytes never
//! panic; the locked view is normalized, keeps an auto-lock trigger on
//! (CRYPTO_SPEC §11) and is never weaker than the defaults (§11.1); the
//! file's own values and MAC survive a render and parse round trip.
#![no_main]
#![allow(
    dead_code,
    unused_imports,
    reason = "the target includes the app modules and uses a subset"
)]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/contract/mod.rs"]
mod contract;
#[path = "../../src/persist.rs"]
mod persist;

fuzz_target!(|data: &[u8]| {
    if let Some((s, file)) = persist::parse_checked(data) {
        // The settings that count while locked: a lock trigger, normalized,
        // never weaker than the defaults, and the SSH agent off (§11.1).
        assert!(persist::has_lock_trigger(&s.settings, persist::SCREEN_LOCK_POSSIBLE));
        assert_eq!(persist::normalize(s.settings.clone()), s.settings);
        assert_eq!(persist::stricter_than_default(&s.settings), s.settings);
        assert!(!s.ssh_agent_enabled);
        // The file's own values and MAC survive a render and parse round trip.
        let rendered = persist::render(&file.state, file.mac.as_ref()).expect("render");
        let (again, again_file) = persist::parse_checked(&rendered).expect("reparse");
        assert_eq!(again, s);
        assert_eq!(again_file.state, file.state);
        assert_eq!(again_file.mac, file.mac);
    }
});
