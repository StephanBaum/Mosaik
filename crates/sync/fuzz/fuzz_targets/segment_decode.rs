//! A folder segment: arbitrary bytes never panic the decoder, and a body that
//! decodes after its MAC is computed (the fuzzer does not know the key, so
//! the target computes it) re-encodes to exactly the same bytes: one segment
//! has one encoding. The IDs and sequence number the path would name are read
//! from the header, so the parser behind the path check is reached.
#![no_main]

use std::sync::OnceLock;

use libfuzzer_sys::fuzz_target;
use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{AccountId, AccountKey, InstallId, VaultId};
use mosaic_sync::wire::segment::{Segment, SegmentPath};
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

/// The path the header names, if it names one.
fn path_of(data: &[u8]) -> Option<SegmentPath> {
    let id = |at: usize| data.get(at..at + 16)?.try_into().ok();
    Some(SegmentPath {
        account_id: AccountId::from_bytes(id(6)?).ok()?,
        vault_id: VaultId::from_bytes(id(22)?).ok()?,
        device_id: InstallId::from_bytes(id(38)?).ok()?,
        seq: u64::from_be_bytes(data.get(54..62)?.try_into().ok()?),
    })
}

fuzz_target!(|data: &[u8]| {
    let c = crypto();
    let Some(path) = path_of(data) else {
        let _ = Segment::decode(data, &SegmentPath {
            account_id: AccountId::from_bytes([1; 16]).expect("id"),
            vault_id: VaultId::from_bytes([2; 16]).expect("id"),
            device_id: InstallId::from_bytes([3; 16]).expect("id"),
            seq: 1,
        }, c);
        return;
    };
    // As given: the MAC fails (or, for a seed that is a whole file, may not).
    let _ = Segment::decode(data, &path, c);
    // With a MAC over the bytes: the parser.
    let mac = c.folder_mac(FolderMacPurpose::Segment, data).expect("mac");
    let mut forged = data.to_vec();
    forged.extend_from_slice(&mac);
    if let Ok(segment) = Segment::decode(&forged, &path, c) {
        let again = segment.encode(c).expect("a decoded segment encodes");
        assert_eq!(again, forged, "one segment, one encoding");
    }
});
