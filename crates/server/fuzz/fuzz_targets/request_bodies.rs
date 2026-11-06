//! The server input before authentication and storage: arbitrary bytes never
//! panic a request body parser, an OPAQUE message parser, the email
//! normalizer or the ciphertext envelope checks, and a request that parses
//! serializes to JSON that parses to the same value.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_core::{ItemCiphertext, TombstoneCiphertext, VaultMetaCiphertext};
use mosaic_sync::wire::email;
use mosaic_sync::wire::json::{self, DEFAULT_MAX_REQUEST_LEN, RecordKind};
use mosaic_sync::wire::opaque::{
    Suite,
    ke::{CredentialFinalization, CredentialRequest, RegistrationRequest, RegistrationUpload},
};
use serde::Serialize;
use serde::de::DeserializeOwned;

fn check<T: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug>(data: &[u8]) -> Option<T> {
    let value = json::parse::<T>(data, DEFAULT_MAX_REQUEST_LEN).ok()?;
    let again = json::to_vec(&value).expect("serializes");
    let back = json::parse::<T>(&again, DEFAULT_MAX_REQUEST_LEN).expect("re-parses");
    assert_eq!(back, value);
    Some(value)
}

fuzz_target!(|data: &[u8]| {
    check::<json::PreloginRequest>(data);
    check::<json::RegisterStartRequest>(data);
    check::<json::RegisterFinishRequest>(data);
    check::<json::LoginStartRequest>(data);
    check::<json::LoginFinishRequest>(data);
    check::<json::CodeRequest>(data);
    check::<json::DeleteAccountRequest>(data);
    check::<json::PasswordStartRequest>(data);
    check::<json::PasswordFinishRequest>(data);
    check::<json::RenameDeviceRequest>(data);
    check::<json::PutKeyRequest>(data);

    // The shape checks the server makes on stored objects.
    if let Some(req) = check::<json::CreateVaultRequest>(data) {
        let _ = VaultMetaCiphertext::from_bytes(&req.meta.0);
    }
    if let Some(req) = check::<json::PushMetaRequest>(data) {
        let _ = VaultMetaCiphertext::from_bytes(&req.ciphertext.0);
    }
    if let Some(req) = check::<json::PushRecordsRequest>(data) {
        for r in &req.records {
            match r.kind {
                RecordKind::Item => {
                    let _ = ItemCiphertext::from_bytes(&r.ciphertext.0);
                }
                RecordKind::Tombstone => {
                    let _ = TombstoneCiphertext::from_bytes(&r.ciphertext.0);
                }
            }
        }
    }

    // The raw bytes as an OPAQUE message of each kind and as envelopes.
    let _ = RegistrationRequest::<Suite>::deserialize(data);
    let _ = RegistrationUpload::<Suite>::deserialize(data);
    let _ = CredentialRequest::<Suite>::deserialize(data);
    let _ = CredentialFinalization::<Suite>::deserialize(data);
    let _ = ItemCiphertext::from_bytes(data);
    let _ = TombstoneCiphertext::from_bytes(data);
    let _ = VaultMetaCiphertext::from_bytes(data);
    if let Ok(text) = core::str::from_utf8(data) {
        let _ = email::normalize(text);
    }
});
