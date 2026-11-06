//! A blob delete marker: arbitrary bytes never panic the decoder, and a body
//! that decodes after its MAC is computed re-encodes to the same bytes.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AccountId, AccountKey, AttachmentId, InstallId, VaultId};
use mosaic_sync::wire::blob_delete::{BlobDelete, BlobDeletePath};
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
    // The path names the three IDs the header holds (or fixed ones when the
    // header is too short), so the parser behind the path check is reached.
    let id = |at: usize, fallback: u8| -> [u8; 16] {
        data.get(at..at + 16)
            .and_then(|b| b.try_into().ok())
            .unwrap_or([fallback; 16])
    };
    let path = BlobDeletePath {
        attachment_id: AttachmentId::from_bytes(id(5, 1))
            .unwrap_or_else(|_| AttachmentId::from_bytes([1; 16]).expect("id")),
        vault_id: VaultId::from_bytes(id(21, 2))
            .unwrap_or_else(|_| VaultId::from_bytes([2; 16]).expect("id")),
        device_id: InstallId::from_bytes(id(37, 3))
            .unwrap_or_else(|_| InstallId::from_bytes([3; 16]).expect("id")),
    };
    let _ = BlobDelete::decode(data, &path, c);
    let mac = c.folder_mac(FolderMacPurpose::BlobDelete, data).expect("mac");
    let mut forged = data.to_vec();
    forged.extend_from_slice(&mac);
    if let Ok(marker) = BlobDelete::decode(&forged, &path, c) {
        assert_eq!(marker.encode(c).expect("encodes"), forged, "one marker, one encoding");
    }
});
