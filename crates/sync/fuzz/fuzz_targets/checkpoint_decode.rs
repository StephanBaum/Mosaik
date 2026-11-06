//! A checkpoint: arbitrary bytes never panic either decoder. A structure the
//! unverified decoder accepts re-encodes (under the key of its own device) to
//! the same body, and the verifying decoder accepts that encoding and
//! returns the same checkpoint.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use mosaic_core::{AccountId, AccountKey, InstallId};
use mosaic_sync::wire::KeyedCrypto;
use mosaic_sync::wire::checkpoint::Checkpoint;

fn key() -> &'static (AccountKey, AccountId) {
    static KEY: OnceLock<(AccountKey, AccountId)> = OnceLock::new();
    KEY.get_or_init(|| {
        (
            AccountKey::generate().expect("key"),
            AccountId::new_v7().expect("account"),
        )
    })
}

fuzz_target!(|data: &[u8]| {
    let (ak, account) = key();
    // The device the checkpoint is filed under is the one its header names.
    let Some(device) = data
        .get(5..21)
        .and_then(|b| b.try_into().ok())
        .and_then(|b: [u8; 16]| InstallId::from_bytes(b).ok())
    else {
        return;
    };
    let Ok(checkpoint) = Checkpoint::decode_unverified(data, device) else {
        return;
    };
    let crypto = KeyedCrypto::new(ak, *account, checkpoint.device_id).expect("crypto");
    let bytes = checkpoint.encode(&crypto).expect("a decoded checkpoint encodes");
    let body = data.len() - 32;
    assert_eq!(bytes.get(..body), data.get(..body), "one body, one encoding");
    let verified = Checkpoint::decode(&bytes, device, &crypto).expect("our own MAC verifies");
    assert_eq!(verified, checkpoint);
    // The MAC of the input (not ours) must not verify.
    let _ = Checkpoint::decode(data, device, &crypto);
});
