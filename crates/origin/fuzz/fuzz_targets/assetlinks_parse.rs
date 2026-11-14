//! Digital Asset Links parsing (ADR-0092): arbitrary bytes never panic the
//! parser or the delegation check, an accepted file has at most 256
//! statements, and a delegation needs a statement for the exact package.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_origin::assetlinks::{
    MAX_ASSET_LINKS_LEN, MAX_STATEMENTS, Relation, Target, app_is_delegated,
    app_is_delegated_for_passkeys, parse_asset_links,
};

fuzz_target!(|data: &[u8]| {
    let Ok(statements) = parse_asset_links(data) else {
        return;
    };
    assert!(data.len() <= MAX_ASSET_LINKS_LEN);
    assert!(statements.len() <= MAX_STATEMENTS);
    let cert = [0x11u8; 32];
    for relation in [Relation::GetLoginCreds, Relation::HandleAllUrls] {
        if app_is_delegated(&statements, "com.example.app", &cert, relation) {
            assert!(statements.iter().any(|s| matches!(
                &s.target,
                Target::AndroidApp { package, .. } if package == "com.example.app"
            )));
        }
    }
    let _ = app_is_delegated_for_passkeys(&statements, "com.example.app", &cert);
});
