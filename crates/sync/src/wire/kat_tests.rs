//! Known-answer tests of the wire formats against the independent Python
//! script `crates/core/tests/vectors/sync_kat.py` (standard library only),
//! whose checked-in output `sync_kat.txt` holds a complete byte vector of a
//! checkpoint, a delete marker, an account file, a segment and a snapshot
//! part. The encoders must produce exactly these bytes and the decoders must
//! accept them.
//!
//! The MACs come from [`KatCrypto`], which computes HMAC-SHA-256 under the
//! script's derived keys with a few lines of RFC 2104 over `sha2`; it shares
//! nothing with `mosaic-core` but the key bytes read from the vector file
//! (the derivation of those keys from `AK` is covered by the KATs of
//! `mosaic-core::sync_auth` against the same file).

use std::collections::BTreeMap;

use mosaic_core::sync_auth::FolderMacPurpose;
use mosaic_core::{
    AccountId, AccountSalt, Argon2Profile, AttachmentId, InstallId, ItemId, KeyGeneration,
    SecretVec, TombstoneCiphertext, VaultId, WrappedKey,
};
use sha2::{Digest, Sha256};

use super::account_file::{AccountFile, AccountFilePath};
use super::blob_delete::{BlobDelete, BlobDeletePath};
use super::checkpoint::{Checkpoint, CheckpointEntry};
use super::segment::{Segment, SegmentObject, SegmentPath, SnapshotInfo, SnapshotTracker};
use super::{SyncCrypto, WireError};

fn kat() -> BTreeMap<String, Vec<u8>> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("core")
        .join("tests")
        .join("vectors")
        .join("sync_kat.txt");
    let text = std::fs::read_to_string(path).unwrap();
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| {
            let (name, hex) = l.split_once('=').unwrap();
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            (name.to_owned(), bytes)
        })
        .collect()
}

fn hmac_sha256(key: &[u8; 32], msg: &[u8]) -> [u8; 32] {
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for (i, k) in key.iter().enumerate() {
        ipad[i] ^= k;
        opad[i] ^= k;
    }
    let inner = Sha256::new()
        .chain_update(ipad)
        .chain_update(msg)
        .finalize();
    Sha256::new()
        .chain_update(opad)
        .chain_update(inner)
        .finalize()
        .into()
}

fn ctx(label: &str) -> Vec<u8> {
    let mut v = b"mosaic/v1/".to_vec();
    v.extend_from_slice(label.as_bytes());
    v
}

/// The MACs of the script's keys, computed independently of `mosaic-core`.
struct KatCrypto {
    k_cp: [u8; 32],
    k_fold: [u8; 32],
    install: InstallId,
}

impl KatCrypto {
    fn new(k: &BTreeMap<String, Vec<u8>>) -> Self {
        Self {
            k_cp: k["k_cp"].as_slice().try_into().unwrap(),
            k_fold: k["k_fold"].as_slice().try_into().unwrap(),
            install: InstallId::from_bytes([0x0E; 16]).unwrap(),
        }
    }

    fn checkpoint(&self, device: &InstallId, counter: u64, digest: &[u8; 32]) -> [u8; 32] {
        let mut msg = ctx("checkpoint");
        msg.extend_from_slice(device.as_bytes());
        msg.extend_from_slice(&counter.to_be_bytes());
        msg.extend_from_slice(digest);
        hmac_sha256(&self.k_cp, &msg)
    }

    fn folder_message(purpose: FolderMacPurpose, msg: &[u8]) -> Vec<u8> {
        let label = match purpose {
            FolderMacPurpose::Segment => "sync/folder-segment",
            FolderMacPurpose::Account => "sync/folder-account",
            FolderMacPurpose::BlobDelete => "sync/folder-blob-delete",
        };
        let mut full = ctx(label);
        full.push(0);
        full.extend_from_slice(msg);
        full
    }
}

impl SyncCrypto for KatCrypto {
    fn install_id(&self) -> InstallId {
        self.install
    }

    fn folder_mac(&self, purpose: FolderMacPurpose, msg: &[u8]) -> Result<[u8; 32], WireError> {
        Ok(hmac_sha256(
            &self.k_fold,
            &Self::folder_message(purpose, msg),
        ))
    }

    fn verify_folder_mac(
        &self,
        purpose: FolderMacPurpose,
        msg: &[u8],
        mac: &[u8; 32],
    ) -> Result<bool, WireError> {
        Ok(self.folder_mac(purpose, msg)? == *mac)
    }

    fn checkpoint_mac(&self, counter: u64, seen_digest: &[u8; 32]) -> Result<[u8; 32], WireError> {
        Ok(self.checkpoint(&self.install, counter, seen_digest))
    }

    fn verify_checkpoint_mac(
        &self,
        device: InstallId,
        counter: u64,
        seen_digest: &[u8; 32],
        mac: &[u8; 32],
    ) -> Result<bool, WireError> {
        Ok(self.checkpoint(&device, counter, seen_digest) == *mac)
    }

    fn seal_device_info(&self, _plaintext: &[u8]) -> Result<Vec<u8>, WireError> {
        Err(WireError::Crypto)
    }

    fn open_device_info(&self, _device: InstallId, _sealed: &[u8]) -> Result<SecretVec, WireError> {
        Err(WireError::Crypto)
    }
}

fn install() -> InstallId {
    InstallId::from_bytes([0x0E; 16]).unwrap()
}

fn account() -> AccountId {
    AccountId::from_bytes([0x0A; 16]).unwrap()
}

fn vault() -> VaultId {
    VaultId::from_bytes([0x0B; 16]).unwrap()
}

#[test]
fn the_key_commit_matches_the_script_and_the_digest_is_version_2() {
    use mosaic_core::ItemId;
    use mosaic_vault::storage::ServedRow;

    let k = kat();
    let rows = [
        ServedRow {
            record_id: ItemId::from_bytes([0x0D; 16]).unwrap(),
            seq: 3,
            ct_hash: Sha256::digest(b"first").into(),
        },
        ServedRow {
            record_id: ItemId::from_bytes([0x0F; 16]).unwrap(),
            seq: 8,
            ct_hash: Sha256::digest(b"second").into(),
        },
    ];
    let commit = super::checkpoint::view_key_commit(&vault(), KeyGeneration::FIRST, &[0x77; 74]);
    assert_eq!(commit.as_slice(), k["key_commit"]);
    // The digest formula is version 2 now (`sync/view2`, with the metadata
    // commitment); its known answer is in `mosaic-core`
    // (`sync_auth::kat::view2_digest`). The script's `view_digest` value is
    // only an opaque 32-byte input of the checkpoint vectors below.
    let meta = super::checkpoint::view_meta_commit(
        &vault(),
        &super::checkpoint::ABSENT_OBJECT,
        &super::checkpoint::ABSENT_OBJECT,
    );
    let digest = super::checkpoint::view_digest(&vault(), 8, &commit, &meta, &rows).unwrap();
    assert_ne!(digest.as_slice(), k["view_digest"]);
}

#[test]
fn the_checkpoint_matches_the_script() {
    let k = kat();
    let crypto = KatCrypto::new(&k);
    let cp = Checkpoint {
        device_id: install(),
        counter: 7,
        issued_at_ms: 1_700_000_000_000,
        entries: vec![CheckpointEntry {
            vault_id: vault(),
            generation: KeyGeneration::FIRST,
            high_seq: 8,
            view_digest: k["view_digest"].as_slice().try_into().unwrap(),
        }],
    };
    assert_eq!(cp.seen_digest().unwrap().as_slice(), k["seen_digest"]);
    let bytes = cp.encode(&crypto).unwrap();
    assert_eq!(bytes, k["checkpoint_bytes"]);
    assert_eq!(
        bytes[bytes.len() - 32..],
        k["checkpoint_mac"][..],
        "the encoded MAC is the script's"
    );
    assert_eq!(Checkpoint::decode(&bytes, install(), &crypto).unwrap(), cp);
    assert!(cp.verify(&bytes, &crypto).unwrap());
    // Filed under another device, the same bytes are refused.
    let other = InstallId::from_bytes([0x0F; 16]).unwrap();
    assert_eq!(
        Checkpoint::decode(&bytes, other, &crypto),
        Err(WireError::Path)
    );
}

#[test]
fn the_delete_marker_matches_the_script() {
    let k = kat();
    let crypto = KatCrypto::new(&k);
    let marker = BlobDelete {
        attachment_id: AttachmentId::from_bytes([0x0C; 16]).unwrap(),
        vault_id: vault(),
        device_id: install(),
        deleted_at_ms: 1_700_000_100_000,
    };
    let bytes = marker.encode(&crypto).unwrap();
    assert_eq!(bytes.len(), 93);
    assert_eq!(bytes, k["blob_delete_bytes"]);
    let path = BlobDeletePath {
        vault_id: vault(),
        attachment_id: marker.attachment_id,
        device_id: install(),
    };
    assert_eq!(BlobDelete::decode(&bytes, &path, &crypto).unwrap(), marker);
}

#[test]
fn the_account_file_matches_the_script() {
    let k = kat();
    let crypto = KatCrypto::new(&k);
    let mut wrapped = vec![0x01, 0x01];
    wrapped.extend_from_slice(&[0x77; 72]);
    let file = AccountFile {
        account_id: account(),
        account_version: 2,
        device_id: install(),
        salt: AccountSalt::from_bytes([0x66; 16]),
        profile: Argon2Profile::Baseline,
        wrapped_ak: WrappedKey::from_bytes(&wrapped).unwrap(),
    };
    let bytes = file.encode(&crypto).unwrap();
    assert_eq!(bytes.len(), 168);
    assert_eq!(bytes, k["account_file_bytes"]);
    let path = AccountFilePath {
        account_id: account(),
        account_version: 2,
        device_id: install(),
    };
    assert_eq!(AccountFile::decode(&bytes, &path, &crypto).unwrap(), file);
}

fn objects() -> (SegmentObject, SegmentObject) {
    let mut wrapped = vec![0x01, 0x01];
    wrapped.extend_from_slice(&[0x55; 72]);
    let mut tombstone = vec![0x01, 0x01, 0, 0, 0, 1];
    tombstone.extend_from_slice(&[0x33; 24]);
    tombstone.extend_from_slice(&[0x44; 20]);
    (
        SegmentObject::VaultKey {
            generation: KeyGeneration::FIRST,
            wrapped: WrappedKey::from_bytes(&wrapped).unwrap(),
        },
        SegmentObject::Tombstone {
            record_id: ItemId::from_bytes([0x0D; 16]).unwrap(),
            ciphertext: TombstoneCiphertext::from_bytes(&tombstone).unwrap(),
        },
    )
}

#[test]
fn the_segment_matches_the_script() {
    let k = kat();
    let crypto = KatCrypto::new(&k);
    let (key, tombstone) = objects();
    let segment = Segment {
        snapshot: None,
        account_id: account(),
        vault_id: vault(),
        device_id: install(),
        seq: 3,
        created_at_ms: 1_700_000_000_000,
        objects: vec![key, tombstone],
    };
    let bytes = segment.encode(&crypto).unwrap();
    assert_eq!(bytes, k["segment_bytes"]);
    let path = SegmentPath {
        account_id: account(),
        vault_id: vault(),
        device_id: install(),
        seq: 3,
    };
    assert_eq!(Segment::decode(&bytes, &path, &crypto).unwrap(), segment);
}

#[test]
fn the_snapshot_part_matches_the_script() {
    let k = kat();
    let crypto = KatCrypto::new(&k);
    let (_, tombstone) = objects();
    let part = Segment {
        snapshot: Some(SnapshotInfo { base: 4, parts: 3 }),
        account_id: account(),
        vault_id: vault(),
        device_id: install(),
        seq: 5,
        created_at_ms: 1_700_000_000_000,
        objects: vec![tombstone],
    };
    let bytes = part.encode(&crypto).unwrap();
    assert_eq!(bytes, k["snapshot_segment_bytes"]);
    assert_eq!(bytes[5], 1, "the snapshot flag");
    let path = SegmentPath {
        account_id: account(),
        vault_id: vault(),
        device_id: install(),
        seq: 5,
    };
    let decoded = Segment::decode(&bytes, &path, &crypto).unwrap();
    assert_eq!(decoded, part);
    // Part 2 of 3 alone completes nothing.
    assert_eq!(SnapshotTracker::new().add(&decoded), Ok(None));
    // The same bytes under the path of another part are refused.
    let wrong = SegmentPath { seq: 6, ..path };
    assert_eq!(
        Segment::decode(&bytes, &wrong, &crypto),
        Err(WireError::Path)
    );
}
