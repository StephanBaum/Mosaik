//! The CLI session file and its keystore value (CRYPTO_SPEC §6.8).
//!
//! - Mode 0: the rest of the input as a keystore value: it parses only if it
//!   is exactly the canonical 64-character form, and then round-trips.
//! - Mode `0xFE`: seal a session, apply the fuzzer's edits (`data[1..]`) to
//!   the file, and require `AK` if and only if the bytes are unchanged;
//!   every change is refused.
//! - Otherwise: arbitrary bytes as a file never open under a fixed key.
#![no_main]

mod common;

use common::{TAMPER, mutate};
use libfuzzer_sys::fuzz_target;
use mosaic_core::kdf::AccountSalt;
use mosaic_core::{
    CliSessionBinding, CliSessionFile, CliSessionKey, Error, InstallId, open_cli_session,
    seal_cli_session_for_fuzzing,
};

const NOW: u64 = 1_700_000_000_000;

fn binding() -> CliSessionBinding {
    CliSessionBinding {
        account_id: common::account(),
        install_id: InstallId::from_bytes([0x0F; 16]).expect("id"),
        ak_epoch: 1,
        salt: AccountSalt::from_bytes([9; 16]),
    }
}

fuzz_target!(|data: &[u8]| {
    match data.split_first() {
        Some((&0, rest)) => {
            let Ok(text) = std::str::from_utf8(rest) else {
                return;
            };
            if let Ok(key) = CliSessionKey::from_keystore_value(text) {
                let back = key.to_keystore_value().expect("encode");
                assert_eq!(back.expose_secret(), text, "keystore value is canonical");
            }
        }
        Some((&TAMPER, ops)) => {
            let ak = common::ak();
            let (key, file) = seal_cli_session_for_fuzzing(&ak, &binding(), NOW, 300_000).expect("seal");
            let bytes = mutate(file.as_bytes(), ops);
            let opened = CliSessionFile::from_bytes(&bytes)
                .and_then(|f| open_cli_session(&key, &f, &binding(), NOW + 1));
            if bytes == file.as_bytes() {
                assert!(opened.expect("unchanged session opens") == ak);
            } else {
                let e = opened.err().expect("a modified session must not open");
                assert!(
                    matches!(e, Error::Malformed | Error::Decryption | Error::Expired),
                    "unexpected error {e:?}"
                );
            }
        }
        _ => {
            let key = CliSessionKey::from_keystore_value(&"A".repeat(64)).expect("fixed key");
            if let Ok(file) = CliSessionFile::from_bytes(data) {
                assert!(
                    open_cli_session(&key, &file, &binding(), NOW).is_err(),
                    "arbitrary bytes opened as a session under a fixed key"
                );
            }
        }
    }
});
