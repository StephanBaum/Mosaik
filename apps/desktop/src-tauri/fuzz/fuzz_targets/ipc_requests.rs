//! The IPC request types: the first byte (minus `A`, modulo 23) picks the
//! type, the rest is the JSON body. Deserializing never panics, and
//! `draft::prepare` refuses a bad draft with `invalidInput` only.
#![no_main]
#![allow(dead_code, unused_imports)]

use libfuzzer_sys::fuzz_target;

#[path = "../../src/contract/mod.rs"]
mod contract;
#[path = "../../src/commands/draft.rs"]
mod draft;
#[path = "../../src/persist.rs"]
mod persist;

use contract::clipboard::CopyRequest;
use contract::errors::ErrorCode;
use contract::generator::GenerateRequest;
use contract::import::{
    ImportCancelRequest, ImportChooseRequest, ImportCommitRequest, ImportPreviewRequest,
};
use contract::items::{
    AddItemRequest, ItemRequest, ListItemsRequest, MoveItemRequest, RevealRequest,
    SetFavoriteRequest, UpdateItemRequest,
};
use contract::session::{
    CreateAccountRequest, QuickUnlockRequest, SaveKitRequest, UnlockRequest, VerifyKitRequest,
};
use contract::settings::UpdateSettingsRequest;
use contract::ssh::{SshRespondRequest, SshSetEnabledRequest};
use contract::vaults::{CreateVaultRequest, RenameVaultRequest};

fn parse<'a, T: serde::Deserialize<'a>>(json: &'a [u8]) -> Option<T> {
    serde_json::from_slice(json).ok()
}

fuzz_target!(|data: &[u8]| {
    let Some((sel, json)) = data.split_first() else {
        return;
    };
    let kind = usize::from(sel.wrapping_sub(b'A')) % 23;
    match kind {
        0 => drop(parse::<CreateAccountRequest>(json)),
        1 => drop(parse::<VerifyKitRequest>(json)),
        2 => drop(parse::<UnlockRequest>(json)),
        3 => drop(parse::<QuickUnlockRequest>(json)),
        4 => drop(parse::<CreateVaultRequest>(json)),
        5 => drop(parse::<RenameVaultRequest>(json)),
        6 => drop(parse::<ListItemsRequest>(json)),
        7 => drop(parse::<ItemRequest>(json)),
        8 => drop(parse::<RevealRequest>(json)),
        9 => {
            if let Some(r) = parse::<AddItemRequest>(json) {
                if let Err(error) = draft::prepare(r.draft) {
                    assert_eq!(error.code, ErrorCode::InvalidInput);
                }
            }
        }
        10 => {
            if let Some(r) = parse::<UpdateItemRequest>(json) {
                if let Err(error) = draft::prepare(r.draft) {
                    assert_eq!(error.code, ErrorCode::InvalidInput);
                }
            }
        }
        11 => drop(parse::<SetFavoriteRequest>(json)),
        12 => drop(parse::<MoveItemRequest>(json)),
        13 => drop(parse::<CopyRequest>(json)),
        14 => drop(parse::<GenerateRequest>(json)),
        15 => drop(parse::<UpdateSettingsRequest>(json)),
        16 => drop(parse::<ImportChooseRequest>(json)),
        17 => drop(parse::<ImportPreviewRequest>(json)),
        18 => drop(parse::<ImportCommitRequest>(json)),
        19 => drop(parse::<ImportCancelRequest>(json)),
        20 => drop(parse::<SshSetEnabledRequest>(json)),
        21 => drop(parse::<SaveKitRequest>(json)),
        _ => drop(parse::<SshRespondRequest>(json)),
    }
});
