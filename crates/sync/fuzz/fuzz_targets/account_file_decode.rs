//! An account file: arbitrary bytes never panic the decoders. A file the
//! join decoder (no MAC) accepts re-encodes, under a key, to the same body.
//! The path is read from the header, so the parser behind it is reached.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use mosaic_core::{AccountId, AccountKey, InstallId};
use mosaic_sync::wire::KeyedCrypto;
use mosaic_sync::wire::account_file::{AccountFile, AccountFilePath, LEN, LEN_V1};

fn key() -> &'static (AccountKey, AccountId) {
    static KEY: OnceLock<(AccountKey, AccountId)> = OnceLock::new();
    KEY.get_or_init(|| {
        (
            AccountKey::generate().expect("key"),
            AccountId::new_v7().expect("account"),
        )
    })
}

fn path_of(data: &[u8]) -> Option<AccountFilePath> {
    let id = |at: usize| data.get(at..at + 16)?.try_into().ok();
    // v2 carries the epoch between the version and the device.
    let device_at = if data.get(4) == Some(&2) { 33 } else { 29 };
    Some(AccountFilePath {
        account_id: AccountId::from_bytes(id(5)?).ok()?,
        account_version: u64::from_be_bytes(data.get(21..29)?.try_into().ok()?),
        device_id: InstallId::from_bytes(id(device_at)?).ok()?,
    })
}

fuzz_target!(|data: &[u8]| {
    let (ak, account) = key();
    let install = InstallId::from_bytes([9; 16]).expect("install");
    let crypto = KeyedCrypto::new(ak, *account, install).expect("crypto");
    let Some(path) = path_of(data) else {
        return;
    };
    let _ = AccountFile::decode(data, &path, &crypto);
    let _ = AccountFile::verify_mac(data, &crypto);
    if let Ok(file) = AccountFile::decode_for_join(data, &path) {
        let bytes = file.encode(&crypto).expect("a decoded file encodes");
        assert_eq!(bytes.len(), LEN);
        // A v1 file re-encodes as v2 (epoch 1); a v2 file is one encoding.
        if data.len() == LEN {
            assert_eq!(bytes.get(..LEN - 32), data.get(..LEN - 32), "one file, one encoding");
        } else {
            assert_eq!(data.len(), LEN_V1);
            assert_eq!(file.ak_epoch, 1);
        }
        assert_eq!(
            AccountFile::decode(&bytes, &path, &crypto).expect("our own MAC verifies"),
            file
        );
    }
});
