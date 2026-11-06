//! The hosted JSON bodies: arbitrary bytes never panic any body type's
//! parser, and a body that parses serializes to JSON that parses to the same
//! value.
#![no_main]

use libfuzzer_sys::fuzz_target;
use mosaic_sync::wire::json::{self, MAX_RESPONSE_LEN};
use serde::Serialize;
use serde::de::DeserializeOwned;

fn check<T: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug>(data: &[u8]) {
    if let Ok(value) = json::parse::<T>(data, MAX_RESPONSE_LEN) {
        let again = json::to_vec(&value).expect("serializes");
        let back = json::parse::<T>(&again, MAX_RESPONSE_LEN).expect("re-parses");
        assert_eq!(back, value);
    }
}

fuzz_target!(|data: &[u8]| {
    check::<json::InfoResponse>(data);
    check::<json::PreloginResponse>(data);
    check::<json::RegisterFinishRequest>(data);
    check::<json::LoginStartResponse>(data);
    check::<json::LoginFinishRequest>(data);
    check::<json::LoginResponse>(data);
    check::<json::AccountResponse>(data);
    check::<json::PasswordFinishRequest>(data);
    check::<json::DevicesResponse>(data);
    check::<json::VaultsResponse>(data);
    check::<json::CreateVaultRequest>(data);
    check::<json::ChangesResponse>(data);
    check::<json::PushRecordsRequest>(data);
    check::<json::PushRecordsResponse>(data);
    check::<json::PushMetaRequest>(data);
    check::<json::MetaResult>(data);
    check::<json::PutKeyRequest>(data);
    check::<json::AttachmentsResponse>(data);
    check::<json::CheckpointsResponse>(data);
    check::<json::PollResponse>(data);
    check::<json::ErrorBody>(data);
    check::<json::Empty>(data);
});
