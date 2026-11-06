//! A blob delete marker: arbitrary bytes never panic the decoder, and a body
//! that decodes after its MAC is computed re-encodes to the same bytes.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AccountId, AccountKey, AttachmentId, InstallId};
use mosaic_sync::wire::blob_delete::BlobDelete;
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
    let attachment = data
        .get(5..21)
        .and_then(|b| b.try_into().ok())
        .and_then(|b: [u8; 16]| AttachmentId::from_bytes(b).ok())
        .unwrap_or_else(|| AttachmentId::from_bytes([1; 16]).expect("id"));
    let _ = BlobDelete::decode(data, attachment, c);
    let mac = c.folder_mac(FolderMacPurpose::BlobDelete, data).expect("mac");
    let mut forged = data.to_vec();
    forged.extend_from_slice(&mac);
    if let Ok(marker) = BlobDelete::decode(&forged, attachment, c) {
        assert_eq!(marker.encode(c).expect("encodes"), forged, "one marker, one encoding");
    }
});
