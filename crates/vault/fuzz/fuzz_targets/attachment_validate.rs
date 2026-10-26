//! `validate_attachment_ciphertext`, differentially against mosaic-core.
//!
//! - Mode `0xFD`: seal the rest of the input as an attachment under the
//!   fixed key; the store must accept it and core must open it.
//! - Otherwise: the store accepts arbitrary bytes exactly when core's
//!   `open_attachment` gets past its structural checks (it then either
//!   opens or fails with `Decryption`), and reports "too large" exactly
//!   when core reports `SizeLimit`.
//!
//! The key and IDs are fixed (the core golden values), so a crash
//! reproduces from its input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::{
    AttachmentId, AttachmentKey, Error as CoreError, ItemId, SecretBytes, VaultId,
    open_attachment, seal_attachment,
};
use mosaic_vault::storage::{Error, validate_attachment_ciphertext};

/// First input byte selecting the seal mode.
const SEAL: u8 = 0xFD;

fuzz_target!(|data: &[u8]| {
    let fk = AttachmentKey::from_secret(SecretBytes::from_array(&mut [0x40; 32]));
    let vault = VaultId::from_bytes([0x0B; 16]).expect("id");
    let item = ItemId::from_bytes([0x0C; 16]).expect("id");
    let attachment = AttachmentId::from_bytes([0x0D; 16]).expect("id");

    if let Some((&SEAL, plaintext)) = data.split_first() {
        let ct = seal_attachment(&fk, &vault, &item, &attachment, plaintext).expect("seal");
        validate_attachment_ciphertext(&ct).expect("a real ciphertext validates");
        let pt = open_attachment(&fk, &vault, &item, &attachment, &ct).expect("open");
        assert_eq!(pt.expose_secret(), plaintext);
        return;
    }

    let ours = validate_attachment_ciphertext(data);
    let core = open_attachment(&fk, &vault, &item, &attachment, data);
    let core_structural_ok = matches!(core, Ok(_) | Err(CoreError::Decryption));
    assert_eq!(
        ours.is_ok(),
        core_structural_ok,
        "store {ours:?} vs core {:?}",
        core.as_ref().err()
    );
    assert_eq!(
        matches!(ours, Err(Error::AttachmentTooLarge)),
        matches!(core, Err(CoreError::SizeLimit))
    );
    if let Err(e) = ours {
        assert!(matches!(
            e,
            Error::AttachmentMalformed | Error::AttachmentTooLarge
        ));
    }
});
