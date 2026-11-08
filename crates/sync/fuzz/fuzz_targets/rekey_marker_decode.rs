//! A re-key marker: arbitrary bytes never panic the decoder, and a body that
//! decodes after its MAC is computed re-encodes to the same bytes. The path is
//! read from the header, so the parser behind the path check is reached.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AccountId, AccountKey, InstallId};
use mosaic_sync::wire::rekey_marker::{LEN, RekeyMarker, RekeyMarkerPath};
use mosaic_sync::wire::{KeyedCrypto, SyncCrypto};

fn crypto() -> &'static KeyedCrypto {
    static CRYPTO: OnceLock<KeyedCrypto> = OnceLock::new();
    CRYPTO.get_or_init(|| {
        let ak = AccountKey::generate().expect("key");
        let account = AccountId::new_v7().expect("account");
        let install = InstallId::new_v7().expect("install");
        KeyedCrypto::new(&ak, account, install).expect("crypto")
    })
}

fuzz_target!(|data: &[u8]| {
    let c = crypto();
    let id = |at: usize, fallback: u8| -> [u8; 16] {
        data.get(at..at + 16)
            .and_then(|b| b.try_into().ok())
            .unwrap_or([fallback; 16])
    };
    let from_epoch = data
        .get(21..25)
        .and_then(|b| b.try_into().ok())
        .map_or(1, u32::from_be_bytes);
    let path = RekeyMarkerPath {
        account_id: AccountId::from_bytes(id(5, 1))
            .unwrap_or_else(|_| AccountId::from_bytes([1; 16]).expect("id")),
        from_epoch,
        device_id: InstallId::from_bytes(id(37, 3))
            .unwrap_or_else(|_| InstallId::from_bytes([3; 16]).expect("id")),
    };
    let _ = RekeyMarker::decode(data, &path, c);
    // The same body with a valid MAC (the whole input, or its first LEN - 32
    // bytes): whatever decodes re-encodes to the same bytes.
    for body in [data, data.get(..LEN - 32).unwrap_or(data)] {
        let mac = c.folder_mac(FolderMacPurpose::Rekey, body).expect("mac");
        let mut forged = body.to_vec();
        forged.extend_from_slice(&mac);
        if let Ok(marker) = RekeyMarker::decode(&forged, &path, c) {
            assert_eq!(marker.encode(c).expect("encodes"), forged, "one marker, one encoding");
        }
    }
});
