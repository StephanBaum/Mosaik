//! Shared checks for the import fuzz targets: whatever a parser accepts
//! must make valid items within the size limit, and the preview must agree
//! with the import.
#![allow(dead_code, reason = "each target uses a subset")]

#[cfg(not(fuzzing))]
compile_error!(
    "the fuzz targets need `--cfg fuzzing`: build them with `cargo +nightly fuzz build`, \
     or check them with RUSTFLAGS=\"--cfg fuzzing\" cargo check"
);

use mosaic_core::{DeviceId, Timestamp};
use mosaic_import::{Error, Import, Result};

/// Checks an accepted import, or that a refusal is one of the documented
/// errors.
pub fn check(result: Result<Import>) {
    let import = match result {
        Ok(i) => i,
        Err(
            Error::TooLarge
            | Error::Malformed
            | Error::UnsupportedVersion
            | Error::Unsupported
            | Error::Encrypted
            | Error::PasswordRequired
            | Error::WrongCredentials
            | Error::LimitExceeded,
        ) => return,
        Err(e) => panic!("undocumented error {e:?}"),
    };
    let device = DeviceId::from_bytes([0x5A; 16]).expect("id");
    let mut indexes: Vec<u32> = import.entries.iter().map(|e| e.index).collect();
    indexes.extend(import.skipped.iter().map(|s| s.index));
    indexes.sort_unstable();
    for (i, index) in indexes.iter().enumerate() {
        assert_eq!(*index as usize, i, "indexes are dense and unique");
    }
    for e in &import.entries {
        let item = e
            .draft
            .to_item(device, Timestamp::from_unix_millis(0))
            .expect("an accepted entry makes an item");
        item.to_json().expect("an accepted entry fits the size limit");
        assert!(format!("{:?}", e.draft).starts_with("ItemDraft"));
        // An accepted passkey is a valid ES256 credential: the key loads and
        // re-encodes to what is stored.
        for p in &e.draft.passkeys {
            p.credential.validate().expect("an accepted passkey is valid");
            assert_eq!(p.credential.algorithm, mosaic_passkey::COSE_ALG_ES256);
            let key = mosaic_passkey::Es256Key::from_pkcs8_der(p.credential.private_key.expose_secret())
                .expect("an accepted passkey has a P-256 key");
            let der = key.to_pkcs8_der().expect("re-encodes");
            assert_eq!(der.expose_secret(), p.credential.private_key.expose_secret());
            assert!(!p.title.is_empty());
            assert!(p.credential.user_name.len() <= mosaic_passkey::MAX_ACCOUNT_NAME_BYTES);
        }
        assert!(e.draft.has_item() || !e.draft.passkeys.is_empty());
    }
    let preview = import.preview(&[]);
    assert_eq!(preview.entries.len(), import.entries.len());
    // One item per entry (none for an entry of passkeys only) and one per
    // passkey.
    let counted: u32 = preview.counts.iter().map(|(_, n)| n).sum();
    let items: usize = import
        .entries
        .iter()
        .map(|e| usize::from(e.draft.has_item()) + e.draft.passkeys.len())
        .sum();
    assert_eq!(counted as usize, items);
    for p in &preview.entries {
        assert!(p.duplicate_of.is_none());
        if let Some(earlier) = p.duplicate_in_file {
            assert!(earlier < p.index);
        }
    }
}
